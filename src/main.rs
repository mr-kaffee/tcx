use clap::Parser;
use std::{
    error::Error,
    fs::{self, File},
    io::Write,
    path::PathBuf,
    str::FromStr,
};
use tcx::*;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// the TCX file to parse
    #[arg(name = "TCX-FILE")]
    path: PathBuf,

    /// print human readable output
    #[arg(short)]
    pretty: bool,

    /// distance in meters for QDH gradient evaluation
    #[arg(long, default_value_t = 50.0, value_parser = parse_f64_non_neg)]
    qdh: f64,

    #[arg(short, hide(true))]
    debug: Option<Debug>,

    #[arg(short, long, default_value_t = Grouping::Length(GroupBy::Duration, 600.0))]
    grouping: Grouping,
}

fn parse_f64_non_neg(s: &str) -> Result<f64, String> {
    let v: f64 = s
        .parse()
        .map_err(|_| format!("'{}' is not a valid number", s))?;
    if v.is_finite() && v >= 0.0 {
        Ok(v)
    } else {
        Err(format!("'{}' is not a finite, non-negative number", s))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum GroupBy {
    Distance,
    Duration,
}

impl GroupBy {
    fn get(&self, m: &Trackpoint, n: &Trackpoint) -> f64 {
        match self {
            GroupBy::Distance => n
                .distance
                .map(|d| d - m.distance.unwrap_or(d))
                .unwrap_or(0.0),
            GroupBy::Duration => n.time.signed_duration_since(m.time).num_seconds() as f64,
        }
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
enum Grouping {
    Length(GroupBy, f64),
    Count(GroupBy, usize),
}

impl std::fmt::Display for Grouping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Length(group_by, length) => write!(f, "{:?},Length,{}", group_by, length),
            Self::Count(group_by, count) => write!(f, "{:?},Count,{}", group_by, count),
        }
    }
}

impl FromStr for Grouping {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split(',').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(format!(
                "'{}' is not a valid grouping value, expected {{duration|distance}},{{length|count}},<value>",
                s
            ));
        }

        let value_s = parts.pop().expect("UNREACHABLE! Popped 1 of 3");
        let mode_s = parts.pop().expect("UNREACHABLE! Popped 2 of 3");
        let group_by_s = parts.pop().expect("UNREACHABLE! Popped 3 of 3");

        let group_by = match group_by_s.to_lowercase().as_str() {
            "duration" => GroupBy::Duration,
            "distance" => GroupBy::Distance,
            _ => {
                return Err(format!(
                    "'{}' is not a valid group by part. Expected {{duration|distance}}",
                    group_by_s
                ))
            }
        };

        match mode_s.to_lowercase().as_str() {
            "length" => match value_s.parse() {
                Ok(length) => Ok(Grouping::Length(group_by, length)),
                Err(e) => Err(format!("Parse error: {}", e.to_string())),
            },
            "count" => match value_s.parse() {
                Ok(count) => Ok(Grouping::Count(group_by, count)),
                Err(e) => Err(format!("Parse error: {}", e.to_string())),
            },
            _ => Err(format!(
                "'{}' is not a valid mode part. Expected {{length|count}}",
                mode_s
            )),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Debug {
    Json,
    Csv,
}

impl FromStr for Debug {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Debug::Json),
            "csv" => Ok(Debug::Csv),
            _ => Err(format!("'{}' is not a valid debug option.", s)),
        }
    }
}

impl Debug {
    fn create_file(&self) -> Result<File, std::io::Error> {
        match self {
            Debug::Json => File::create("debug.json"),
            Debug::Csv => File::create("debug.csv"),
        }
    }

    fn debug_json(&self, mut file: File, points: Vec<Trackpoint>) -> Result<(), Box<dyn Error>> {
        // header
        writeln!(file, "[")?;

        // body
        let mut first = true;
        for point in points {
            if first {
                first = false;
            } else {
                writeln!(file, ",")?;
            }

            write!(file, "{{\"Time\": \"{}\"", point.time)?;
            for field in &TRK_PT_FIELD {
                write!(file, ", \"{}\": ", field.as_ref())?;
                match point[field] {
                    Some(v) => write!(file, "{}", v)?,
                    None => write!(file, "null")?,
                }
            }
            write!(file, "}}")?;
        }

        // footer
        writeln!(file, "]")?;

        Ok(())
    }

    fn debug_csv(&self, mut file: File, points: Vec<Trackpoint>) -> Result<(), Box<dyn Error>> {
        // header
        write!(file, "Time")?;
        for field in &TRK_PT_FIELD {
            write!(file, ",{}", field.as_ref())?;
        }
        writeln!(file)?;

        // body
        for point in points {
            write!(file, "{}", point.time)?;
            for field in &TRK_PT_FIELD {
                write!(file, ",")?;
                if let Some(v) = point[field] {
                    write!(file, "{}", v)?;
                }
            }
            writeln!(file)?;
        }

        // no footer in CSV

        Ok(())
    }

    fn debug(&self, points: Vec<Trackpoint>) -> Result<(), Box<dyn Error>> {
        let file = self.create_file()?;
        println!("Debugging, {} points to {:?}", points.len(), file);

        match self {
            Debug::Json => self.debug_json(file, points)?,
            Debug::Csv => self.debug_csv(file, points)?,
        }

        Ok(())
    }
}

fn write_window(vals: (f64, f64, f64, f64, f64, f64), qdh: f64, pretty: bool) {
    // group_length, distance, duration, elevation, power, heartrate
    if pretty {
        // print human readable
        println!(
            "{:6.2}W / {:6.2}bpm for {:8.2}s ({:7.3}km, {:5.2}km/h, {:4.0}m, {:5.1} m/km, QDH: {:6.1})",
            vals.4 / vals.2,
            vals.5 / vals.2,
            vals.2,
            vals.1 / 1000.0,
            vals.1 / vals.2 * 3.6,
            vals.3,
            vals.3 / vals.1 * 1000.0,
            qdh
        )
    } else {
        // print CSV style
        println!(
            "{:6.2}{sep}{:6.2}{sep}{:8.2}{sep}{:7.3}{sep}{:5.2}{sep}{:4.0}{sep}{:5.1}{sep}{:6.1}",
            vals.4 / vals.2,
            vals.5 / vals.2,
            vals.2,
            vals.1 / 1000.0,
            vals.1 / vals.2 * 3.6,
            vals.3,
            vals.3 / vals.1 * 1000.0,
            qdh,
            sep = ','
        )
    }
}

fn update_qdh(s_qdh: QdhType, inc_d: f64, inc_a: f64, d_qdh: f64, flush: bool) -> QdhType {
    // update qdh
    let mut inc_d = inc_d;
    let mut inc_a = inc_a;
    let mut vals_qdh = s_qdh;
    while vals_qdh.1 + inc_d >= d_qdh {
        let f = (d_qdh - vals_qdh.1) / inc_d;
        vals_qdh = (
            vals_qdh.0 + calc_qdh(d_qdh, vals_qdh.2 + f * inc_a),
            0.0,
            0.0,
        );
        inc_d = (1.0 - f) * inc_d;
        inc_a = (1.0 - f) * inc_a;
    }
    if flush {
        (
            vals_qdh.0 + calc_qdh(vals_qdh.1 + inc_d, vals_qdh.2 + inc_a),
            0.0,
            0.0,
        )
    } else {
        (vals_qdh.0, vals_qdh.1 + inc_d, vals_qdh.2 + inc_a)
    }
}

fn calc_qdh(distance: f64, elevation: f64) -> f64 {
    if distance > 0.0 {
        // distance/1km * (gradient/1%)^2
        // distance/1m / 1000 * (elevation/1m)^2 / (distance/1m)^2 * 100^2
        // (elevation/1m)^2 / (distance/1m) * 10
        elevation * elevation / distance * 10.0
    } else {
        0.0
    }
}

type ValType = (f64, f64, f64, f64, f64, f64);
type QdhType = (f64, f64, f64);

const VAL_ZERO: ValType = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
const QDH_ZERO: QdhType = (0.0, 0.0, 0.0);

fn mul(v: ValType, f: f64) -> ValType {
    (v.0 * f, v.1 * f, v.2 * f, v.3 * f, v.4 * f, v.5 * f)
}

fn add(v1: ValType, v2: ValType) -> ValType {
    (
        v1.0 + v2.0,
        v1.1 + v2.1,
        v1.2 + v2.2,
        v1.3 + v2.3,
        v1.4 + v2.4,
        v1.5 + v2.5,
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    // parse command line
    let cli = Cli::parse();

    // get points
    let filter: fn(&Trackpoint) -> bool = if cli.debug.is_some() {
        |_| true
    } else {
        |t| t.altitude.is_some() && t.distance.is_some()
    };
    let points = Trackpoint::from_tcx(&fs::read_to_string(cli.path)?.parse()?, filter)?;

    if let Some(debug) = cli.debug {
        return debug.debug(points);
    }

    let group_by = match cli.grouping {
        Grouping::Count(group_by, _) => group_by,
        Grouping::Length(group_by, _) => group_by,
    };

    // determine length of group in secends based on command line options
    let group_length = match cli.grouping {
        Grouping::Length(_, length) => length,
        Grouping::Count(group_by, count) => {
            let tot = group_by.get(
                points.first().expect("No points"),
                points.last().expect("UNREACHABLE! First but no last point"),
            );
            println!(
                "{:?}: {} / {} = {}",
                group_by,
                tot,
                count,
                tot / (count as f64)
            );
            tot / (count as f64)
        }
    };
    assert!(
        group_length > 0.0,
            "Something went wrong with the group length!"
    );

    // group_length, distance, duration, elevation, power, heartrate
    let mut vals = VAL_ZERO;
    let mut s_qdh = QDH_ZERO;

    for (m, n) in points.iter().zip(points.iter().skip(1)) {
        // increments (group_length, distance, duration, elevation, power, heartrate)
        let incs = (
            group_by.get(m, n),
            GroupBy::Distance.get(m, n),
            GroupBy::Duration.get(m, n),
            (n.altitude
                .expect("UNREACHABLE! Points w/o altitude filtered out")
                - m.altitude
                    .expect("UNREACHABLE! Points w/o altitude filtered out"))
            .max(0.0),
            (n.power.unwrap_or(0.0) + m.power.unwrap_or(0.0)) / 2.0
                * (GroupBy::Duration.get(m, n) as f64),
            (n.heartrate.unwrap_or(0.0) + m.heartrate.unwrap_or(0.0)) / 2.0
                * (GroupBy::Duration.get(m, n) as f64),
        );

        // check if group is done
        (s_qdh, vals) = if vals.0 + incs.0 >= group_length {
            let f = (group_length - vals.0) / incs.0;
            let (qdh, ..) = update_qdh(s_qdh, incs.2 * f, incs.3 * f, cli.qdh, true);

            // print group
            write_window(add(vals, mul(incs, f)), qdh, cli.pretty);

            (
                update_qdh(
                    QDH_ZERO,
                    incs.2 * (1.0 - f),
                    incs.3 * (1.0 - f),
                    cli.qdh,
                    false,
                ),
                mul(incs, 1.0 - f),
            )
        } else {
            (
                update_qdh(s_qdh, incs.2, incs.3, cli.qdh, false),
                add(vals, incs),
            )
        };
    }

    // print last group if applicable
    if vals.0 > 1e-6 * group_length {
        let (qdh, ..) = update_qdh(s_qdh, 0.0, 0.0, cli.qdh, true);
        write_window(vals, qdh, cli.pretty);
    }

    Ok(())
}
