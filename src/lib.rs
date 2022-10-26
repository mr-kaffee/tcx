use std::{error::Error, str::FromStr};

use chrono::{DateTime, Utc};
use minidom::{Element, NSChoice};

#[derive(Debug, PartialEq, Default)]
pub struct Trackpoint {
    time: DateTime<Utc>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    altitude: Option<f64>,
    distance: Option<f64>,
    heartrate: Option<f64>,
    cadence: Option<f64>,
    speed: Option<f64>,
    power: Option<f64>,
}

fn get_child<'a>(e: &'a Element, n: &str) -> Option<&'a Element> {
    e.get_child(n, NSChoice::Any)
}

fn value<T: FromStr>(e: &Element) -> Result<T, <T as FromStr>::Err> {
    e.text().parse()
}

impl Trackpoint {
    const TIME: &str = "Time";
    const POSITION: &str = "Position";
    const LATITUDE_DEGREES: &str = "LatitudeDegrees";
    const LONGITUDE_DEGREES: &str = "LongitudeDegrees";
    const ALTITUDE_METERS: &str = "AltitudeMeters";
    const DISTANCE_METERS: &str = "DistanceMeters";
    const HEART_RATE_BPM: &str = "HeartRateBpm";
    const VALUE: &str = "Value";
    const CADENCE: &str = "Cadence";
    const EXTENSIONS: &str = "Extensions";
    const TPX: &str = "TPX";
    const SPEED: &str = "Speed";
    const WATTS: &str = "Watts";

    const F_ACTIVITIES: fn(&&Element) -> bool = |e| e.is("Activities", NSChoice::Any);
    const F_ACTIVITY: fn(&&Element) -> bool = |e| e.is("Activity", NSChoice::Any);
    const F_LAP: fn(&&Element) -> bool = |e| e.is("Lap", NSChoice::Any);
    const F_TRACK: fn(&&Element) -> bool = |e| e.is("Track", NSChoice::Any);
    const F_TRACKPOINT: fn(&&Element) -> bool = |e| e.is("Trackpoint", NSChoice::Any);

    pub fn from_tcx(tcx: &Element, filter: fn(&Self) -> bool) -> Result<Vec<Self>, Box<dyn Error>> {
        // traverse document
        let it = tcx.children().filter(Self::F_ACTIVITIES);
        let it = it.map(|e| e.children().filter(Self::F_ACTIVITY)).flatten();
        let it = it.map(|e| e.children().filter(Self::F_LAP)).flatten();
        let it = it.map(|e| e.children().filter(Self::F_TRACK)).flatten();
        let it = it
            .map(|e| e.children().filter(Self::F_TRACKPOINT))
            .flatten();

        // collect trackpoints in vector
        let mut points = it
            .map(|trackpoint| Trackpoint::parse(trackpoint))
            .filter(|t| t.as_ref().map_or(true, filter))
            .collect::<Result<Vec<_>, _>>()?;

        // remove duplicates
        points.dedup();

        Ok(points)
    }

    pub fn parse(trackpoint: &Element) -> Result<Self, Box<dyn Error>> {
        let time = value(
            get_child(trackpoint, Self::TIME)
                .ok_or_else(|| format!("Missing time in {:?}", trackpoint))?,
        )?;

        let position = get_child(trackpoint, Self::POSITION);
        let latitude = position
            .map(|e| get_child(e, Self::LATITUDE_DEGREES))
            .flatten()
            .map(value)
            .transpose()?;
        let longitude = position
            .map(|e| get_child(e, Self::LONGITUDE_DEGREES))
            .flatten()
            .map(value)
            .transpose()?;

        let altitude = get_child(trackpoint, Self::ALTITUDE_METERS)
            .map(value)
            .transpose()?;

        let distance = get_child(trackpoint, Self::DISTANCE_METERS)
            .map(value)
            .transpose()?;

        let heartrate = get_child(trackpoint, Self::HEART_RATE_BPM)
            .map(|e| get_child(e, Self::VALUE))
            .flatten()
            .map(value)
            .transpose()?;

        let cadence = get_child(trackpoint, Self::CADENCE)
            .map(value)
            .transpose()?;

        let tpx = get_child(trackpoint, Self::EXTENSIONS)
            .map(|e| get_child(e, Self::TPX))
            .flatten();
        let speed = tpx
            .map(|e| get_child(e, Self::SPEED))
            .flatten()
            .map(value)
            .transpose()?;
        let power = tpx
            .map(|e| get_child(e, Self::WATTS))
            .flatten()
            .map(value)
            .transpose()?;

        Ok(Trackpoint {
            time,
            latitude,
            longitude,
            altitude,
            distance,
            heartrate,
            cadence,
            speed,
            power,
        })
    }

    pub fn time(&self) -> DateTime<Utc> {
        self.time
    }

    pub fn duration_since(&self, rhs: &Self) -> i64 {
        self.time.signed_duration_since(rhs.time).num_seconds()
    }

    pub fn altitude(&self) -> Result<f64, String> {
        self.altitude
            .ok_or_else(|| format!("Missing altitude in {:?}", self))
    }

    pub fn distance(&self) -> Result<f64, String> {
        self.distance
            .ok_or_else(|| format!("Missing distance in {:?}", self))
    }

    pub fn heartrate(&self) -> Result<f64, String> {
        self.heartrate
            .ok_or_else(|| format!("Missing heartrate in {:?}", self))
    }

    pub fn heartrate_or_default(&self) -> f64 {
        self.heartrate.unwrap_or_default()
    }

    pub fn power(&self) -> Result<f64, String> {
        self.power
            .ok_or_else(|| format!("Missing power in {:?}", self))
    }

    pub fn power_or_default(&self) -> f64 {
        self.power.unwrap_or_default()
    }

    pub fn has_altitude(&self) -> bool {
        self.altitude.is_some()
    }

    pub fn has_distance(&self) -> bool {
        self.distance.is_some()
    }

    pub fn has_heartrate(&self) -> bool {
        self.heartrate.is_some()
    }

    pub fn has_power(&self) -> bool {
        self.power.is_some()
    }
}
