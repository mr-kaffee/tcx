use std::{error::Error, str::FromStr};

use chrono::{DateTime, Utc};
use minidom::{Element, NSChoice};

#[derive(Debug, PartialEq, Default)]
pub struct Trackpoint {
    time: DateTime<Utc>,
    latitude: f64,
    longitude: f64,
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

fn get_child_or_error<'a>(e: &'a Element, n: &str) -> Result<&'a Element, Box<dyn Error>> {
    get_child(e, n).ok_or_else(|| format!("No such child '{}'", n).into())
}

fn child_value<T: FromStr>(e: &Element) -> Result<T, <T as FromStr>::Err> {
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

    pub fn parse(trackpoint: &Element) -> Result<Self, Box<dyn Error>> {
        let time = child_value(get_child_or_error(trackpoint, Self::TIME)?)?;
        let position = get_child_or_error(trackpoint, Self::POSITION)?;
        let latitude = child_value(get_child_or_error(position, Self::LATITUDE_DEGREES)?)?;
        let longitude = child_value(get_child_or_error(position, Self::LONGITUDE_DEGREES)?)?;
        let altitude = get_child(trackpoint, Self::ALTITUDE_METERS)
            .map(child_value)
            .transpose()?;
        let distance = get_child(trackpoint, Self::DISTANCE_METERS)
            .map(child_value)
            .transpose()?;
        let heartrate = get_child(trackpoint, Self::HEART_RATE_BPM)
            .map(|e| get_child(e, Self::VALUE))
            .flatten()
            .map(child_value)
            .transpose()?;
        let cadence = get_child(trackpoint, Self::CADENCE)
            .map(child_value)
            .transpose()?;
        let tpx = get_child(trackpoint, Self::EXTENSIONS)
            .map(|e| get_child(e, Self::TPX))
            .flatten();
        let speed = tpx
            .map(|e| get_child(e, Self::SPEED))
            .flatten()
            .map(child_value)
            .transpose()?;
        let power = tpx
            .map(|e| get_child(e, Self::WATTS))
            .flatten()
            .map(child_value)
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

    pub fn power(&self) -> Result<f64, String> {
        self.power
            .ok_or_else(|| format!("Missing power in {:?}", self))
    }
}
