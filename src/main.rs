use clap::Parser;
use std::{error::Error, fs, io::Write};
use tcx::*;

mod cli {
    use super::GroupBy;
    use clap::Parser;
    use std::{path::PathBuf, str::FromStr};

    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct Cli {
        /// the TCX file to parse
        #[arg(name = "TCX-FILE")]
        pub path: PathBuf,

        /// print human readable output
        #[arg(short)]
        pub pretty: bool,

        /// distance in meters for QDH gradient evaluation
        #[arg(long, default_value_t = 50.0, value_parser = parse_f64_non_neg)]
        pub qdh: f64,

        #[arg(short, hide(true))]
        pub debug: Option<Debug>,

        #[arg(short, long, default_value_t = Grouping::Length(GroupBy::Duration, 600.0))]
        pub grouping: Grouping,
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

    #[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
    pub enum Grouping {
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
    pub enum Debug {
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
}

mod debug {
    use super::cli::Debug;
    use super::*;
    use std::fs::File;

    fn create_file(debug: &Debug) -> Result<File, std::io::Error> {
        match debug {
            Debug::Json => File::create("debug.json"),
            Debug::Csv => File::create("debug.csv"),
        }
    }

    fn debug_json(mut file: File, points: Vec<Trackpoint>) -> Result<(), Box<dyn Error>> {
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

    fn debug_csv(mut file: File, points: Vec<Trackpoint>) -> Result<(), Box<dyn Error>> {
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

    pub fn debug(debug: &Debug, points: Vec<Trackpoint>) -> Result<(), Box<dyn Error>> {
        let file = create_file(debug)?;
        println!("Debugging, {} points to {:?}", points.len(), file);

        match debug {
            Debug::Json => debug_json(file, points)?,
            Debug::Csv => debug_csv(file, points)?,
        }

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum GroupBy {
    Distance,
    Duration,
}

impl GroupBy {
    pub fn get(&self, m: &Trackpoint, n: &Trackpoint) -> f64 {
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
struct Values {
    group_len: f64,
    duration: f64,
    distance: f64,
    elevation: f64,
    power: f64,
    heartrate: f64,
}

impl Values {
    fn add(&self, other: &Self) -> Self {
        Self {
            group_len: self.group_len + other.group_len,
            duration: self.duration + other.duration,
            distance: self.distance + other.distance,
            elevation: self.elevation + other.elevation,
            power: self.power + other.power,
            heartrate: self.heartrate + other.heartrate,
        }
    }

    fn mult(&self, f: f64) -> Self {
        Self {
            group_len: f * self.group_len,
            duration: f * self.duration,
            distance: f * self.distance,
            elevation: f * self.elevation,
            power: f * self.power,
            heartrate: f * self.heartrate,
        }
    }

    fn zero() -> Self {
        Self {
            group_len: 0.0,
            duration: 0.0,
            distance: 0.0,
            elevation: 0.0,
            power: 0.0,
            heartrate: 0.0,
        }
    }

    fn delta(m: &Trackpoint, n: &Trackpoint, group_by: GroupBy) -> Self {
        Self {
            group_len: group_by.get(m, n),
            distance: GroupBy::Distance.get(m, n),
            duration: GroupBy::Duration.get(m, n),
            elevation: (n
                .altitude
                .expect("UNREACHABLE! Points w/o altitude filtered out")
                - m.altitude
                    .expect("UNREACHABLE! Points w/o altitude filtered out"))
            .max(0.0),
            power: (n.power.unwrap_or(0.0) + m.power.unwrap_or(0.0)) / 2.0
                * (GroupBy::Duration.get(m, n) as f64),
            heartrate: (n.heartrate.unwrap_or(0.0) + m.heartrate.unwrap_or(0.0)) / 2.0
                * (GroupBy::Duration.get(m, n) as f64),
        }
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
struct Qdh {
    qdh: f64,
    distance: f64,
    elevation: f64,
}

impl Qdh {
    fn update(&mut self, inc_distance: f64, inc_elevation: f64, group_length: f64, flush: bool) {
        let mut inc_distance = inc_distance;
        let mut inc_elevation = inc_elevation;
        // let mut vals_qdh = self;
        while self.distance + inc_distance >= group_length {
            let f = (group_length - self.distance) / inc_distance;
            self.increment(f * inc_distance, f * inc_elevation);
            self.flush();
            inc_distance = (1.0 - f) * inc_distance;
            inc_elevation = (1.0 - f) * inc_elevation;
        }

        self.increment(inc_distance, inc_elevation);
        if flush {
            self.flush();
        }
    }

    fn increment(&mut self, inc_distance: f64, inc_elevation: f64) {
        self.distance += inc_distance;
        self.elevation += inc_elevation;
    }

    fn flush(&mut self) {
        if self.distance > 0.0 {
            // distance/1km * (gradient/1%)^2
            // distance/1m / 1000 * (elevation/1m)^2 / (distance/1m)^2 * 100^2
            // (elevation/1m)^2 / (distance/1m) * 10
            self.qdh += self.elevation * self.elevation / self.distance * 10.0;
        }
        self.distance = 0.0;
        self.elevation = 0.0;
    }

    fn zero() -> Self {
        Self {
            qdh: 0.0,
            distance: 0.0,
            elevation: 0.0,
        }
    }
}

fn write_window(vals: &Values, qdh: &Qdh, pretty: bool) {
    // group_length, distance, duration, elevation, power, heartrate
    if pretty {
        // print human readable
        println!(
            "{:6.2}W / {:6.2}bpm for {:8.2}s ({:7.3}km, {:5.2}km/h, {:4.0}m, {:5.1} m/km, QDH: {:6.1})",
            vals.power / vals.duration,
            vals.heartrate / vals.duration,
            vals.duration,
            vals.distance / 1000.0,
            vals.distance / vals.duration * 3.6,
            vals.elevation,
            vals.elevation / vals.distance * 1000.0,
            qdh.qdh
        )
    } else {
        // print CSV style
        println!(
            "{:6.2}{sep}{:6.2}{sep}{:8.2}{sep}{:7.3}{sep}{:5.2}{sep}{:4.0}{sep}{:5.1}{sep}{:6.1}",
            vals.power / vals.duration,
            vals.heartrate / vals.duration,
            vals.duration,
            vals.distance / 1000.0,
            vals.distance / vals.duration * 3.6,
            vals.elevation,
            vals.elevation / vals.distance * 1000.0,
            qdh.qdh,
            sep = ','
        )
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // parse command line
    let cli = cli::Cli::parse();

    // get points (filtered if not debug mode)
    let filter: fn(&Trackpoint) -> bool = if cli.debug.is_some() {
        |_| true
    } else {
        |t| t.altitude.is_some() && t.distance.is_some()
    };
    let points = Trackpoint::from_tcx(&fs::read_to_string(cli.path)?.parse()?, filter)?;

    if let Some(debug) = cli.debug {
        // write debug output and exit
        return debug::debug(&debug, points);
    }

    // get group by from CLI
    let group_by = match cli.grouping {
        cli::Grouping::Count(group_by, _) => group_by,
        cli::Grouping::Length(group_by, _) => group_by,
    };

    // determine length of group in secends based on command line options
    let group_len = match cli.grouping {
        cli::Grouping::Length(_, length) => length,
        cli::Grouping::Count(group_by, count) => {
            let tot = group_by.get(
                points.first().expect("No points"),
                points.last().expect("UNREACHABLE! First but no last point"),
            );
            tot / (count as f64)
        }
    };
    assert!(group_len > 0.0);

    let mut values = Values::zero();
    let mut qdh = Qdh::zero();

    for (m, n) in points.iter().zip(points.iter().skip(1)) {
        // increments (group_length, distance, duration, elevation, power, heartrate)
        let incs = Values::delta(m, n, group_by);

        // check if group is done
        if values.group_len + incs.group_len >= group_len {
            let f = (group_len - values.group_len) / incs.group_len;
            qdh.update(incs.distance * f, incs.elevation * f, cli.qdh, true);

            // print group
            write_window(&values.add(&incs.mult(f)), &qdh, cli.pretty);

            // reset Qdh and Values
            qdh = Qdh::zero();
            qdh.update(
                incs.distance * (1.0 - f),
                incs.elevation * (1.0 - f),
                cli.qdh,
                false,
            );
            values = incs.mult(1.0 - f);
        } else {
            // update Qdh and Values
            qdh.update(incs.distance, incs.elevation, cli.qdh, false);
            values = values.add(&incs);
        };
    }

    // print last group if applicable
    if values.group_len > 1e-6 * group_len {
        qdh.update(0.0, 0.0, cli.qdh, true);
        write_window(&values, &qdh, cli.pretty);
    }

    Ok(())
}
