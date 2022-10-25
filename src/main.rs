use clap::Parser;
use minidom::{Element, NSChoice};
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

const F_ACTIVITIES: fn(&&Element) -> bool = |e| e.is("Activities", NSChoice::Any);
const F_ACTIVITY: fn(&&Element) -> bool = |e| e.is("Activity", NSChoice::Any);
const F_LAP: fn(&&Element) -> bool = |e| e.is("Lap", NSChoice::Any);
const F_TRACK: fn(&&Element) -> bool = |e| e.is("Track", NSChoice::Any);
const F_TRACKPOINT: fn(&&Element) -> bool = |e| e.is("Trackpoint", NSChoice::Any);

fn write_it(pwr: f64, hr: f64, secs: i64, dist: f64, elev: f64, pretty: bool) {
    if pretty {
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
        println!(
            "{:6.2};{:6.2};{};{:5.3};{:5.2};{:5.1}",
            pwr / (secs as f64),
            hr / (secs as f64),
            secs,
            dist,
            dist / (secs as f64) * 3600.0,
            elev
        )
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    let root: Element = fs::read_to_string(cli.path)?.parse()?;

    let it = root.children().filter(F_ACTIVITIES);
    let it = it.map(|e| e.children().filter(F_ACTIVITY)).flatten();
    let it = it.map(|e| e.children().filter(F_LAP)).flatten();
    let it = it.map(|e| e.children().filter(F_TRACK)).flatten();
    let it = it.map(|e| e.children().filter(F_TRACKPOINT)).flatten();

    let mut points = it
        .map(|trackpoint| Trackpoint::parse(trackpoint))
        .collect::<Result<Vec<_>, _>>()?;
    points.dedup();

    let start = points.first().ok_or("No points!")?;

    let mut distance_0 = start.distance()?;
    let mut distance = 0.0;
    let mut group = 0;
    let mut duration = 0;
    let mut power = 0.0;
    let mut heartrate = 0.0;
    let mut ascend = 0.0;

    let group_duration = if let Some(number) = cli.number {
        let number = number as i64;
        (points
            .last()
            .expect("UNREACHABLE (first but no last)")
            .duration_since(start)
            + number
            - 1)
            / number
    } else {
        cli.time
    };

    for (m, n) in points.iter().zip(points.iter().skip(1)) {
        let duration_delta = n.duration_since(m);
        duration += duration_delta;
        power += (n.power()? + m.power()?) / 2.0 * (duration_delta as f64);
        heartrate += (n.heartrate()? + m.heartrate()?) / 2.0 * (duration_delta as f64);
        distance = (n.distance()? - distance_0) / 1000.0;
        if n.altitude() > m.altitude() {
            ascend += n.altitude()? - m.altitude()?;
        }

        let group_n = n.duration_since(start) / group_duration;
        if group_n > group {
            write_it(power, heartrate, duration, distance, ascend, cli.pretty);
            group = group_n;
            power = 0.0;
            heartrate = 0.0;
            duration = 0;
            distance_0 = n.distance()?;
            distance = 0.0;
            ascend = 0.0;
        }
    }
    if duration > 0 {
        write_it(power, heartrate, duration, distance, ascend, cli.pretty);
    }

    Ok(())
}
