use clap::Parser;
use std::{error::Error, fs, path::PathBuf};
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

    /// number of windows
    #[arg(short, group = "windows")]
    number: Option<u8>,

    /// length of windows in seconds
    #[arg(short, group = "windows", default_value_t = 600)]
    time: i64,

    /// distance in meters for QDH gradient evaluation
    #[arg(long, default_value_t = 50.0)]
    qdh: f64,
}

const SEPARATOR: char = ';';

fn write(pwr: f64, hr: f64, secs: i64, dist: f64, elev: f64, qdh: f64, pretty: bool) {
    if pretty {
        // print human readable
        println!(
            "{:6.2}W / {:6.2}bpm for {}s ({:5.3}km, {:5.2}km/h, {:3.0}m, QDH: {:5.1})",
            pwr / (secs as f64),
            hr / (secs as f64),
            secs,
            dist / 1000.0,
            dist / (secs as f64) * 3.6,
            elev,
            qdh
        )
    } else {
        // print CSV style
        println!(
            "{:6.2}{sep}{:6.2}{sep}{}{sep}{:5.3}{sep}{:5.2}{sep}{:5.1}{sep}{:5.1}",
            pwr / (secs as f64),
            hr / (secs as f64),
            secs,
            dist / 1000.0,
            dist / (secs as f64) * 3.6,
            elev,
            qdh,
            sep = SEPARATOR
        )
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // parse command line
    let cli = Cli::parse();

    // get points
    let filter: fn(&Trackpoint) -> bool = |t| t.has_altitude() && t.has_distance();
    let points = Trackpoint::from_tcx(&fs::read_to_string(cli.path)?.parse()?, filter)?;

    // first point (error if there is none)
    let start = points.first().ok_or("No points!")?;

    // determine length of group in secends based on command line options
    let group_duration = if let Some(number) = cli.number {
        let number = number as i64;
        let end = points.last().expect("UNREACHABLE (first but no last)");
        (end.duration_since(start) + number - 1) / number
    } else {
        cli.time
    };

    let mut distance = 0.0;
    let mut duration = 0;
    let mut ascend = 0.0;
    let mut power = 0.0;
    let mut heartrate = 0.0;

    let mut distance_qdh = 0.0;
    let mut ascend_qdh = 0.0;
    let mut qdh = 0.0;

    for (m, n) in points.iter().zip(points.iter().skip(1)) {
        // increments
        let dur = n.duration_since(m);
        let asc = n.altitude()? - m.altitude()?;

        // update distance
        distance += n.distance()? - m.distance()?;

        // increment duration
        duration += dur;

        // increment ascend if applicable
        if asc > 0.0 {
            ascend += asc;
            ascend_qdh += asc;
        }

        // increment power / heartrate accumulators
        power += (n.power_or_default() + m.power_or_default()) / 2.0 * (dur as f64);
        heartrate +=
            (n.heartrate_or_default() + m.heartrate_or_default()) / 2.0 * (dur as f64);

        // update qdh
        distance_qdh += n.distance()? - m.distance()?;
        if distance_qdh >= cli.qdh {
            qdh += ascend_qdh * ascend_qdh / distance_qdh * 10.0;
            ascend_qdh = 0.0;
            distance_qdh = 0.0;
        }

        // check if group is done
        if duration >= group_duration {
            if distance_qdh > 0.0 {
                qdh += ascend_qdh * ascend_qdh / distance_qdh / 1000.0;
            }
            ascend_qdh = 0.0;
            distance_qdh = 0.0;

            // print group
            write(
                power, heartrate, duration, distance, ascend, qdh, cli.pretty,
            );

            // reset accumulators
            distance = 0.0;
            duration = 0;
            ascend = 0.0;
            power = 0.0;
            heartrate = 0.0;
            qdh = 0.0;
        }
    }

    // print last group if applicable
    if duration > 0 {
        if distance_qdh > 0.0 {
            qdh += ascend_qdh * ascend_qdh / distance_qdh / 1000.0;
        }
        write(
            power, heartrate, duration, distance, ascend, qdh, cli.pretty,
        );
    }

    Ok(())
}
