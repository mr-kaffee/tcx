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
}

const SEPARATOR: char = ';';

fn write(pwr: f64, hr: f64, secs: i64, dist: f64, elev: f64, pretty: bool) {
    if pretty {
        // print human readable
        println!(
            "{:6.2}W / {:6.2}bpm for {}s ({:5.3}km, {:5.2}km/h, {:3.0}m)",
            pwr / (secs as f64),
            hr / (secs as f64),
            secs,
            dist,
            dist / (secs as f64) * 3600.0,
            elev
        )
    } else {
        // print CSV style
        println!(
            "{:6.2}{sep}{:6.2}{sep}{}{sep}{:5.3}{sep}{:5.2}{sep}{:5.1}",
            pwr / (secs as f64),
            hr / (secs as f64),
            secs,
            dist,
            dist / (secs as f64) * 3600.0,
            elev, 
            sep = SEPARATOR
        )
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // parse command line
    let cli = Cli::parse();

    // get points
    let filter: fn(&Trackpoint) -> bool =
        |t| t.has_altitude() && t.has_distance() && t.has_heartrate() && t.has_power();
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

    let mut group_id = 0;
    let mut distance_0 = start.distance()?;
    let mut distance = 0.0;
    let mut duration = 0;
    let mut ascend = 0.0;
    let mut power = 0.0;
    let mut heartrate = 0.0;

    for (m, n) in points.iter().zip(points.iter().skip(1)) {
        let duration_delta = n.duration_since(m);

        // update distance in km
        distance = (n.distance()? - distance_0) / 1000.0;
        
        // increment duration
        duration += duration_delta;
        
        // increment ascend if applicable
        if n.altitude() > m.altitude() {
            ascend += n.altitude()? - m.altitude()?;
        }

        // increment power / heartrate accumulators
        power += (n.power()? + m.power()?) / 2.0 * (duration_delta as f64);
        heartrate += (n.heartrate()? + m.heartrate()?) / 2.0 * (duration_delta as f64);

        // check if group is done
        let next_group_id = n.duration_since(start) / group_duration;
        if next_group_id > group_id {
            // print group
            write(power, heartrate, duration, distance, ascend, cli.pretty);

            // reset accumulators
            group_id = next_group_id;
            distance_0 = n.distance()?;
            distance = 0.0;
            duration = 0;
            ascend = 0.0;
            power = 0.0;
            heartrate = 0.0;
        }
    }

    // print last group if applicable
    if duration > 0 {
        write(power, heartrate, duration, distance, ascend, cli.pretty);
    }

    Ok(())
}
