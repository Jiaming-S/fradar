use std::{collections::VecDeque, sync::{Arc, Mutex}, time::Duration};

use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct FRadarData {
  pub flights_data: Arc<Mutex<FlightData>>,
  pub flights_data_history: VecDeque<Arc<Mutex<FlightData>>>,

  pub state: FRadarState,
  pub args: FRadarArgs,
}

impl FRadarData {
  pub fn enqueue_data(&mut self) {
    self.flights_data_history.push_back(self.flights_data.clone());

    if self.flights_data_history.len() > self.args.history_rolling_limit {
      self.flights_data_history.pop_front();
    }
  }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum FRadarState {
  #[default]
  Main,
  GracefulKill,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct FRadarArgs {
  pub origin: Position,
  pub radius: f64,

  pub starting_origin: Position,

  pub data_interval: Duration,
  pub frame_interval: Duration,
  pub event_interval: Duration,

  pub terminal_cols: u16,
  pub terminal_rows: u16,

  pub terminal_edge_margins: u16,

  pub label_label_repelling_force: f64,
  pub label_point_repelling_force: f64,
  pub label_snapping_radius: f64,

  pub history_rolling_limit: usize,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct FlightData {
  pub flights: Vec<(Position, Label)>,
  pub epoch_timestamp: i64,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Position {
  pub lat: f64,
  pub long: f64,
}

impl Position {
  pub fn latlong_miles_ratio() -> f64 {
    69.44 // TODO: dynamically find value
  }
  
  pub fn character_aspect_ratio() -> f64 {
    2.0 // TODO: dynamically find value
  }

  pub fn roughly_eq(&self, other: &Self) -> bool {
    (self.lat - other.lat).abs() * Self::latlong_miles_ratio() < 0.1 && (self.long - other.long).abs() * Self::latlong_miles_ratio() < 0.1
  }

  pub fn as_terminal_coord(&self, args: &FRadarArgs) -> anyhow::Result<Coord<u16>> {
    self.as_terminal_coord_float(args).try_into()
  }

  pub fn as_terminal_coord_float(&self, args: &FRadarArgs) -> Coord<f64> {
    let terminal_cols: f64 = args.terminal_cols.into();
    let terminal_rows: f64 = args.terminal_rows.into();
  
    let latlong_to_miles: f64  = Self::latlong_miles_ratio();     // TODO: dynamically find value
    let char_aspect_ratio: f64 = Self::character_aspect_ratio();  // TODO: dynamically find value
    let lat_scale_factor: f64  = (f64::max(terminal_cols / 2.0, terminal_rows / 2.0)) / (args.radius as f64) * latlong_to_miles * char_aspect_ratio;
    let long_scale_factor: f64 = (f64::max(terminal_cols / 2.0, terminal_rows / 2.0)) / (args.radius as f64) * latlong_to_miles;

    let delta_lat  = self.lat - args.origin.lat;
    let delta_long = self.long - args.origin.long;
    
    let delta_rows = -delta_lat * lat_scale_factor;
    let delta_cols =  delta_long * long_scale_factor;

    let col = terminal_cols / 2.0 + delta_cols;
    let row = terminal_rows / 2.0 + delta_rows;

    let clamped_col = col.clamp(0.0, terminal_cols);
    let clamped_row = row.clamp(0.0, terminal_rows);

    Coord {
      col: clamped_col,
      row: clamped_row,
    }
  }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Hash, Eq)]
pub struct Coord<T> {
  pub col: T,
  pub row: T,
}

impl Coord<u16> {
  pub fn is_in_box(&self, x: u16, y: u16, w: u16, h: u16) -> bool {
    self.col >= x && self.col <= x + w && self.row >= y && self.row <= y + h
  }
}

impl Coord<f64> {
  pub fn squared_dist(&self, other: Self) -> f64 {
    (self.col - other.col).powi(2) + (self.row - other.row).powi(2)
  }
}

impl TryFrom<Coord<f64>> for Coord<u16> {
  type Error = anyhow::Error;

  fn try_from(value: Coord<f64>) -> Result<Self, Self::Error> {
    if value.col < 0.0 || value.row < 0.0 {
      return Err(anyhow!("Can't convert negative value col: {} row: {}", value.col, value.row));
    }

    Ok(Coord {
      col: value.col as u16,
      row: value.row as u16,
    })
  }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct Label {
  pub registration: String,
  pub flight: String,
  pub plane: String,
  pub squawk: String,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub enum LabelPosition {
  TopLeft,
  #[default]
  TopRight,
  BottomLeft,
  BottomRight,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct ADSBData {
  pub ac: Vec<ADSBAircraftInformation>,
  pub msg: String,
  pub now: i64,
  pub total: u32,
  
  #[serde(with = "chrono::serde::ts_milliseconds")]
  pub ctime: DateTime<Utc>,
  pub ptime: i64,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ADSBAircraftInformation {
  pub hex: String,
  #[serde(rename = "type")]
  pub aircraft_type: Option<String>,
  pub flight: Option<String>,
  pub r: Option<String>,
  pub t: Option<String>,
  pub dbFlags: Option<u32>,
  #[serde(default)]
  #[serde(deserialize_with = "deserialize_to_string")]
  pub alt_baro: Option<String>,
  pub alt_geom: Option<i32>,
  pub gs: Option<f32>,
  pub tas: Option<u32>,
  pub track: Option<f32>,
  pub roll: Option<f32>,
  pub geom_rate: Option<i32>,
  pub squawk: Option<String>,
  pub emergency: Option<String>,
  pub category: Option<String>,
  pub nav_qnh: Option<f32>,
  pub lat: f64,
  pub lon: f64,
  pub nic: Option<u8>,
  pub rc: Option<u32>,
  pub seen_pos: Option<f32>,
  pub version: Option<u8>,
  pub nic_baro: Option<u8>,
  pub nac_p: Option<u8>,
  pub nac_v: Option<u8>,
  pub sil: Option<u8>,
  pub sil_type: Option<String>,
  pub gva: Option<u8>,
  pub sda: Option<u8>,
  pub alert: Option<u8>,
  pub spi: Option<u8>,
  pub mlat: Vec<String>,
  pub tisb: Vec<String>,
  pub messages: Option<u32>,
  pub seen: Option<f32>,
  pub rssi: Option<f32>,
  pub dst: Option<f64>,
  pub dir: Option<f32>,
}

impl TryFrom<ADSBAircraftInformation> for Position {
  type Error = anyhow::Error;

  fn try_from(adsb_aircraft_info: ADSBAircraftInformation) -> Result<Self, Self::Error> {
    Ok(Position {
      lat: adsb_aircraft_info.lat,
      long: adsb_aircraft_info.lon,
    })
  }
}

impl TryFrom<ADSBAircraftInformation> for Label {
  type Error = anyhow::Error;

  fn try_from(adsb_aircraft_info: ADSBAircraftInformation) -> Result<Self, Self::Error> {
    Ok(Label {
      registration: adsb_aircraft_info.r.unwrap_or_default(),
      flight: adsb_aircraft_info.flight.unwrap_or_default(),
      plane: adsb_aircraft_info.t.unwrap_or_default(),
      squawk: adsb_aircraft_info.squawk.unwrap_or_default(),
    })
  }
}

impl Label {
  pub fn to_string(&self, label_position: LabelPosition) -> String {
    match label_position {
      LabelPosition::TopLeft => format!("{}\n{:>width$}", self.lefthanded_string(), "\\", width = self.len()),
      LabelPosition::TopRight => format!("{}\n/", self.righthanded_string()),
      LabelPosition::BottomLeft => format!("{:>width$}\n{}", "/", self.lefthanded_string(), width = self.len()),
      LabelPosition::BottomRight => format!("\\\n{}", self.righthanded_string()),
    }
  }

  pub fn all_notna_fields(&self) -> Vec<String> {
    let mut result = Vec::from([
      self.registration.clone(),
      self.flight.clone(),
      self.plane.clone(),
      self.squawk.clone(),
    ]);
    result.retain(|str| str.len() > 0);
    result
  }

  pub fn len(&self) -> usize {
    self.all_notna_fields().iter().map(|str| str.len()).max().unwrap_or(0)
  }

  pub fn height(&self) -> usize {
    self.all_notna_fields().len()
  }

  fn lefthanded_string(&self) -> String {
    self.all_notna_fields().iter()
      .map(|str| format!("{:>width$}", str, width = self.len()))
      .collect::<Vec<String>>()
      .join("\n")
  }

  fn righthanded_string(&self) -> String {
    self.all_notna_fields().join("\n")
  }

  pub fn compute_display_delta(&self, label_position: LabelPosition) -> (i32, i32) {
    match label_position {
      LabelPosition::TopLeft => (-1 - self.len() as i32, -1 - self.height() as i32),
      LabelPosition::TopRight => (1, -1 - self.height() as i32),
      LabelPosition::BottomLeft => (-1 - self.len() as i32, 1),
      LabelPosition::BottomRight => (1, 1),
    }
  }
}

pub fn deserialize_to_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where D: Deserializer<'de> {
  let value: Option<Value> = Option::deserialize(deserializer)?;
  let result = value.map(|v| Some(match v {
    Value::String(s) => s,
    Value::Number(n) => n.to_string(),
    Value::Bool(b) => b.to_string(),
    Value::Null => return None,
    _ => "".to_string(),
  }));
    
  Ok(result.unwrap())
}
