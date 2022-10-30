use clap::Parser;
use std::{
    error::Error,
    path::PathBuf, fs,
};
use tcx::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// the TCX file to parse
    #[arg(name = "TCX-FILE")]
    path: PathBuf,

    /// print human readable output
    #[arg(short)]
    pretty: bool,

    /// number of windows (1 .. 255)
    #[arg(short, group = "windows", value_parser = clap::value_parser!(u8).range(1..))]
    number: Option<u8>,

    /// length of windows in seconds (> 0)
    #[arg(short, group = "windows", default_value_t = 600, value_parser = clap::value_parser!(i64).range(1..))]
    time: i64,

    /// distance in meters for QDH gradient evaluation
    #[arg(long, default_value_t = 50.0, value_parser = parse_f64_non_neg)]
    qdh: f64,
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

fn write_window(
    power: f64,
    heartrate: f64,
    duration: i64,
    distance: f64,
    elevation: f64,
    qdh: f64,
    pretty: bool,
) {
    if pretty {
        // print human readable
        println!(
            "{:6.2}W / {:6.2}bpm for {:5}s ({:7.3}km, {:5.2}km/h, {:4.0}m, {:5.1} m/km, QDH: {:6.1})",
            power / (duration as f64),
            heartrate / (duration as f64),
            duration,
            distance / 1000.0,
            distance / (duration as f64) * 3.6,
            elevation,
            elevation / distance * 1000.0,
            qdh
        )
    } else {
        // print CSV style
        println!(
            "{:6.2}{sep}{:6.2}{sep}{:5}{sep}{:7.3}{sep}{:5.2}{sep}{:4.0}{sep}{:5.1}{sep}{:6.1}",
            power / (duration as f64),
            heartrate / (duration as f64),
            duration,
            distance / 1000.0,
            distance / (duration as f64) * 3.6,
            elevation,
            elevation / distance * 1000.0,
            qdh,
            sep = ','
        )
    }
}

fn calc_qdh(elevation: f64, distance: f64) -> f64 {
    if distance > 0.0 {
        // distance/1km * (gradient/1%)^2
        // distance/1m / 1000 * (elevation/1m)^2 / (distance/1m)^2 * 100^2
        // (elevation/1m)^2 / (distance/1m) * 10
        elevation * elevation / distance * 10.0
    } else {
        0.0
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // parse command line
    let cli = Cli::parse();

    // get points
    let filter: fn(&Trackpoint) -> bool = |t| t.altitude.is_some() && t.distance.is_some();
    let points = Trackpoint::from_tcx(&fs::read_to_string(cli.path)?.parse()?, filter)?;

    // first point (error if there is none)
    let start = points.first().ok_or("No points!")?;

    // determine length of group in secends based on command line options
    let group_duration = if let Some(number) = cli.number {
        let number = number as i64;
        let end = points.last().expect("UNREACHABLE (first but no last)");
        (end.time.signed_duration_since(start.time).num_seconds() + number - 1) / number
    } else {
        cli.time
    };

    let mut distance = 0.0;
    let mut duration = 0;
    let mut elevation = 0.0;
    let mut power = 0.0;
    let mut heartrate = 0.0;

    let mut distance_qdh = 0.0;
    let mut elevation_qdh = 0.0;
    let mut qdh = 0.0;

    for (m, n) in points.iter().zip(points.iter().skip(1)) {
        // increments
        let duration_inc = n.time.signed_duration_since(m.time).num_seconds();
        let distance_inc = n
            .distance
            .expect("UNREACHABLE (points w/o distance filtered out)")
            - m.distance
                .expect("UNREACHABLE (points w/o distance filtered out)");
        let elevation_inc = n
            .altitude
            .expect("UNREACHABLE (points w/o altitude filtered out)")
            - m.altitude
                .expect("UNREACHABLE (points w/o altitude filtered out)");

        // update distance
        distance += distance_inc;

        // increment duration
        duration += duration_inc;

        // increment ascend if applicable
        if elevation_inc > 0.0 {
            elevation += elevation_inc;
            elevation_qdh += elevation_inc;
        }

        // increment power / heartrate accumulators
        power += (n.power.unwrap_or(0.0) + m.power.unwrap_or(0.0)) / 2.0 * (duration_inc as f64);
        heartrate +=
            (n.heartrate.unwrap_or(0.0) + m.heartrate.unwrap_or(0.0)) / 2.0 * (duration_inc as f64);

        // update qdh
        distance_qdh += distance_inc;
        if distance_qdh >= cli.qdh {
            qdh += calc_qdh(elevation_qdh, distance_qdh);
            elevation_qdh = 0.0;
            distance_qdh = 0.0;
        }

        // check if group is done
        if duration >= group_duration {
            qdh += calc_qdh(elevation_qdh, distance_qdh);
            elevation_qdh = 0.0;
            distance_qdh = 0.0;

            // print group
            write_window(
                power, heartrate, duration, distance, elevation, qdh, cli.pretty,
            );

            // reset accumulators
            distance = 0.0;
            duration = 0;
            elevation = 0.0;
            power = 0.0;
            heartrate = 0.0;
            qdh = 0.0;
        }
    }

    // print last group if applicable
    if duration > 0 {
        qdh += calc_qdh(elevation_qdh, distance_qdh);
        write_window(
            power, heartrate, duration, distance, elevation, qdh, cli.pretty,
        );
    }

    Ok(())
}
