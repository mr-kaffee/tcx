use std::{
    error::Error,
    ops::{Index, IndexMut},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use minidom::{Element, NSChoice};

/// relevant XML tags of TCX files
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
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

impl AsRef<str> for Tag {
    /// Get `Tag` as `&str`
    ///
    /// # Examples
    /// ```
    /// # use tcx::*;
    /// let tag = Tag::Extensions;
    /// assert_eq!(format!("Tag '{}'", tag.as_ref()), "Tag 'Extensions'");
    /// ```
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

/// Fields of the [`Trackpoint`] enum
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrkPtField {
    /// Represent [`Trackpoint::latitude`]
    Latitude,
    /// Represent [`Trackpoint::longitude`]
    Longitude,
    /// Represent [`Trackpoint::altitude`]
    Altitude,
    /// Represent [`Trackpoint::distance`]
    Distance,
    /// Represent [`Trackpoint::heartrate`]
    Heartrate,
    /// Represent [`Trackpoint::cadence`]
    Cadence,
    /// Represent [`Trackpoint::speed`]
    Speed,
    /// Represent [`Trackpoint::power`]
    Power,
}

impl TrkPtField {
    pub fn get_tags(&self) -> &[&[Tag]] {
        match self {
            TrkPtField::Latitude => &[&[Tag::Position, Tag::LatitudeDegrees]],
            TrkPtField::Longitude => &[&[Tag::Position, Tag::LongitudeDegrees]],
            TrkPtField::Altitude => &[&[Tag::AltitudeMeters]],
            TrkPtField::Distance => &[&[Tag::DistanceMeters]],
            TrkPtField::Heartrate => &[&[Tag::HeartRateBpm, Tag::Value]],
            TrkPtField::Cadence => &[
                &[Tag::Cadence],
                &[Tag::Extensions, Tag::TPX, Tag::RunCadence],
            ],
            TrkPtField::Speed => &[&[Tag::Extensions, Tag::TPX, Tag::Speed]],
            TrkPtField::Power => &[&[Tag::Extensions, Tag::TPX, Tag::Watts]],
        }
    }
}

impl AsRef<str> for TrkPtField {
    /// Get `TrkPtField` as `&str`
    ///
    /// # Examples
    /// ```
    /// # use tcx::*;
    /// let trk_pt_field = TrkPtField::Altitude;
    /// assert_eq!(format!("Track Point Field '{}'", trk_pt_field.as_ref()), "Track Point Field 'Altitude'");
    /// ```
    fn as_ref(&self) -> &str {
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

/// array of all variants for the enum [`TrkPtField`]
pub const TRK_PT_FIELDS: [TrkPtField; 8] = [
    TrkPtField::Latitude,
    TrkPtField::Longitude,
    TrkPtField::Altitude,
    TrkPtField::Distance,
    TrkPtField::Heartrate,
    TrkPtField::Cadence,
    TrkPtField::Speed,
    TrkPtField::Power,
];

/// a track point
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Trackpoint {
    /// Timestamp when the trackpoint was recorded (`<Time>`)
    pub time: DateTime<Utc>,
    /// Longitude part of the position ([`<Position><LatitudeDegrees>`][TrkPtField::Latitude])
    pub latitude: Option<f64>,
    /// Latitude part of the position ([`<Position><LongitudeDegrees>`][TrkPtField::Longitude])
    pub longitude: Option<f64>,
    /// Altitude at the track point's position ([`<AltitudeMeters>`][TrkPtField::Altitude])
    pub altitude: Option<f64>,
    /// Distance travelled in track until this track point ([`<DistanceMeters>`][TrkPtField::Distance])
    pub distance: Option<f64>,
    /// Instantaneous heart rate ([`<HeartRateBpm><Value>`][TrkPtField::Heartrate])
    pub heartrate: Option<f64>,
    /// Instantaneous cadence ([`<Cadence>` or `<Extensions><TPX><RunCadence>`][TrkPtField::Cadence])
    pub cadence: Option<f64>,
    /// Instantaneous speed ([`<Extensions><TPX><Speed>`][TrkPtField::Speed])
    pub speed: Option<f64>,
    /// Instantaneous power ([`<Extensions><TPX><Watts>`][TrkPtField::Power])
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

impl IndexMut<&TrkPtField> for Trackpoint {
    fn index_mut(&mut self, index: &TrkPtField) -> &mut Self::Output {
        match index {
            TrkPtField::Latitude => &mut self.latitude,
            TrkPtField::Longitude => &mut self.longitude,
            TrkPtField::Altitude => &mut self.altitude,
            TrkPtField::Distance => &mut self.distance,
            TrkPtField::Heartrate => &mut self.heartrate,
            TrkPtField::Cadence => &mut self.cadence,
            TrkPtField::Speed => &mut self.speed,
            TrkPtField::Power => &mut self.power,
        }
    }
}

pub trait TcxElement {
    /// Check whether a given `TcxElement` is a `tag` ignoring name spaces
    fn is_tag(&self, tag: Tag) -> bool;

    /// Get text of child paresd into `T`
    ///
    /// The function will descend the hiearchy given by the `tags` slice.
    fn child_value<T: FromStr>(&self, tags: &[Tag]) -> Result<Option<T>, <T as FromStr>::Err>;
}

impl TcxElement for Element {
    fn is_tag(&self, tag: Tag) -> bool {
        self.is(tag, NSChoice::Any)
    }

    /// # Examples
    /// ```
    /// # use tcx::*;
    /// use minidom::Element;
    ///
    /// let doc = r#"<Root xmlns="arbitrary">
    ///   <Extensions>
    ///     <TPX>
    ///       <Speed>42.0</Speed>
    ///     </TPX>
    ///   </Extensions>
    /// </Root>"#;
    ///
    /// let val: f64 = doc.parse::<Element>().unwrap()
    ///     .child_value(&[Tag::Extensions, Tag::TPX, Tag::Speed])
    ///     .expect("Parse error").expect("Missing node");
    /// assert_eq!(val, 42.0);
    /// ```
    fn child_value<T: FromStr>(&self, tags: &[Tag]) -> Result<Option<T>, <T as FromStr>::Err> {
        let mut e = Some(self);
        for tag in tags {
            e = e.map(|e| e.get_child(*tag, NSChoice::Any)).flatten();
        }
        e.map(|e| e.text().parse()).transpose()
    }
}

impl Trackpoint {
    /// Read track points from TCX element flattening any structure
    ///
    /// This function assumes that [`<Trackpoint>`][Tag::Trackpoint]s are nested in [`<Track>`][Tag::Track]s, [`<Track>`][Tag::Track]s
    /// are nested in [`<Lap>`][Tag::Lap]s, [`<Lap>`][Tag::Lap]s are nested in [`<Activity>`][Tag::Activity]s, and
    /// [`<Activity>`][Tag::Activity]s are nested in [`<Activities>`][Tag::Activities]'
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

    /// Parse a single trackpoint for a [`<Trackpoint>`][Tag::Trackpoint]
    ///
    /// # Examples
    /// ```
    /// # use tcx::*;
    ///
    /// let doc = r#"<Trackpoint xmlns="arbitrary">
    ///   <Time>2022-12-31 23:59:59 UTC</Time>
    ///   <Position>
    ///     <LongitudeDegrees>9.0</LongitudeDegrees>
    ///     <LatitudeDegrees>48.640970</LatitudeDegrees>
    ///   </Position>
    ///   <AltitudeMeters>450.0</AltitudeMeters>
    ///   <HeartRateBpm><Value>100</Value></HeartRateBpm>
    ///   <Extensions>
    ///     <TPX>
    ///       <Watts>250</Watts>
    ///       <RunCadence>90</RunCadence>
    ///     </TPX>
    ///   </Extensions>
    ///   <AnotherTag>will do no harm</AnotherTag>
    /// </Trackpoint>"#;
    ///
    /// let trackpoint = Trackpoint::parse(&doc.parse().unwrap()).unwrap();
    ///
    /// assert_eq!(trackpoint.longitude, Some(9.0));
    /// assert_eq!(trackpoint.latitude, Some(48.640970));
    /// assert_eq!(trackpoint.altitude, Some(450.0));
    /// assert_eq!(trackpoint.distance, None);
    /// assert_eq!(trackpoint.heartrate, Some(100.0));
    /// assert_eq!(trackpoint.speed, None);
    /// assert_eq!(trackpoint.power, Some(250.0));
    /// assert_eq!(trackpoint.cadence, Some(90.0));
    /// ```
    pub fn parse(trackpoint: &Element) -> Result<Self, Box<dyn Error>> {
        let time = trackpoint
            .child_value(&[Tag::Time])?
            .ok_or_else(|| format!("Missing time in {:?}", trackpoint))?;
        let mut point = Trackpoint {
            time,
            ..Trackpoint::default()
        };

        for field in &TRK_PT_FIELDS {
            for tags in field.get_tags() {
                if let Some(val) = trackpoint.child_value(tags)? {
                    point[field] = Some(val);
                    break;
                }
            }
        }

        Ok(point)
    }
}
