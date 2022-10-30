use std::{error::Error, ops::Index, slice::Iter, str::FromStr};

use chrono::{DateTime, Utc};
use minidom::{Element, NSChoice};

/// relevant XML tags of TCX files
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tag {
    Time,
    Position,
    LatitudeDegrees,
    LongitudeDegrees,
    AltitudeMeters,
    DistanceMeters,
    HeartRateBpm,
    Value,
    Cadence,
    Extensions,
    TPX,
    Speed,
    Watts,
    RunCadence,
    Activities,
    Activity,
    Lap,
    Track,
    Trackpoint,
}

// implementing as ref allows to use the type in minidom directly
impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        match self {
            Tag::Time => "Time",
            Tag::Position => "Position",
            Tag::LatitudeDegrees => "LatitudeDegrees",
            Tag::LongitudeDegrees => "LongitudeDegrees",
            Tag::AltitudeMeters => "AltitudeMeters",
            Tag::DistanceMeters => "DistanceMeters",
            Tag::HeartRateBpm => "HeartRateBpm",
            Tag::Value => "Value",
            Tag::Cadence => "Cadence",
            Tag::Extensions => "Extensions",
            Tag::TPX => "TPX",
            Tag::Speed => "Speed",
            Tag::Watts => "Watts",
            Tag::RunCadence => "RunCadence",
            Tag::Activities => "Activities",
            Tag::Activity => "Activity",
            Tag::Lap => "Lap",
            Tag::Track => "Track",
            Tag::Trackpoint => "Trackpoint",
        }
    }
}

/// Fields in the trackpoint enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrkPtField {
    Latitude,
    Longitude,
    Altitude,
    Distance,
    Heartrate,
    Cadence,
    Speed,
    Power,
}

impl Into<&str> for &TrkPtField {
    /// Convert TrkPtFields to str representations
    fn into(self) -> &'static str {
        match self {
            TrkPtField::Latitude => "Latitude",
            TrkPtField::Longitude => "Longitued",
            TrkPtField::Altitude => "Altitude",
            TrkPtField::Distance => "Distance",
            TrkPtField::Heartrate => "Heartrate",
            TrkPtField::Cadence => "Cadence",
            TrkPtField::Speed => "Speed",
            TrkPtField::Power => "Power",
        }
    }
}

/// array of all possible TrkPtFields
const TRK_PT_FIELDS: [TrkPtField; 8] = [
    TrkPtField::Latitude,
    TrkPtField::Longitude,
    TrkPtField::Altitude,
    TrkPtField::Distance,
    TrkPtField::Heartrate,
    TrkPtField::Cadence,
    TrkPtField::Speed,
    TrkPtField::Power,
];

impl TrkPtField {
    /// returns an iterator over all possible TrkPtFields
    pub fn iter() -> Iter<'static, TrkPtField> {
        TRK_PT_FIELDS.iter()
    }
}

/// a track point
#[derive(Debug, PartialEq, Default)]
pub struct Trackpoint {
    pub time: DateTime<Utc>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub distance: Option<f64>,
    pub heartrate: Option<f64>,
    pub cadence: Option<f64>,
    pub speed: Option<f64>,
    pub power: Option<f64>,
}

impl Index<&TrkPtField> for Trackpoint {
    type Output = Option<f64>;

    fn index(&self, index: &TrkPtField) -> &Self::Output {
        match index {
            TrkPtField::Latitude => &self.latitude,
            TrkPtField::Longitude => &self.longitude,
            TrkPtField::Altitude => &self.altitude,
            TrkPtField::Distance => &self.distance,
            TrkPtField::Heartrate => &self.heartrate,
            TrkPtField::Cadence => &self.cadence,
            TrkPtField::Speed => &self.speed,
            TrkPtField::Power => &self.power,
        }
    }
}

trait TcxElement {
    fn is_tag(&self, tag: Tag) -> bool;
    fn child_value<T: FromStr>(&self, tags: &[Tag]) -> Result<Option<T>, <T as FromStr>::Err>;
}

impl TcxElement for Element {
    fn is_tag(&self, tag: Tag) -> bool {
        self.is(tag, NSChoice::Any)
    }

    fn child_value<T: FromStr>(&self, tags: &[Tag]) -> Result<Option<T>, <T as FromStr>::Err> {
        let mut e = Some(self);
        for tag in tags {
            e = e.map(|e| e.get_child(*tag, NSChoice::Any)).flatten();
        }
        e.map(|e| e.text().parse()).transpose()
    }
}

impl Trackpoint {
    pub fn from_tcx(tcx: &Element, filter: fn(&Self) -> bool) -> Result<Vec<Self>, Box<dyn Error>> {
        // traverse document
        let mut points = [tcx]
            .iter()
            .flat_map(|e| e.children().filter(|e| e.is_tag(Tag::Activities)))
            .flat_map(|e| e.children().filter(|e| e.is_tag(Tag::Activity)))
            .flat_map(|e| e.children().filter(|e| e.is_tag(Tag::Lap)))
            .flat_map(|e| e.children().filter(|e| e.is_tag(Tag::Track)))
            .flat_map(|e| e.children().filter(|e| e.is_tag(Tag::Trackpoint)))
            .map(|trackpoint| Trackpoint::parse(trackpoint))
            .filter(|t| t.as_ref().map_or(true, filter))
            .collect::<Result<Vec<_>, _>>()?;

        // remove duplicates
        points.dedup();

        Ok(points)
    }

    pub fn parse(trackpoint: &Element) -> Result<Self, Box<dyn Error>> {
        let time = trackpoint
            .child_value(&[Tag::Time])?
            .ok_or_else(|| format!("Missing time in {:?}", trackpoint))?;
        let latitude = trackpoint.child_value(&[Tag::Position, Tag::LatitudeDegrees])?;
        let longitude = trackpoint.child_value(&[Tag::Position, Tag::LongitudeDegrees])?;
        let altitude = trackpoint.child_value(&[Tag::AltitudeMeters])?;
        let distance = trackpoint.child_value(&[Tag::DistanceMeters])?;
        let heartrate = trackpoint.child_value(&[Tag::HeartRateBpm, Tag::Value])?;
        let cadence = trackpoint.child_value(&[Tag::Cadence])?;
        let speed = trackpoint.child_value(&[Tag::Extensions, Tag::TPX, Tag::Speed])?;
        let power = trackpoint.child_value(&[Tag::Extensions, Tag::TPX, Tag::Watts])?;
        let cadence = match cadence {
            Some(_) => cadence,
            None => trackpoint.child_value(&[Tag::Extensions, Tag::TPX, Tag::RunCadence])?,
        };

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
}
