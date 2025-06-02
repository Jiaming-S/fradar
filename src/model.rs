use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;


#[derive(Debug, Clone)]
pub struct FlightData {
  pub flights: Vec<Position>,
}

impl FlightData {
  pub fn empty() -> Self {
    FlightData {
      flights: vec![],
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct Position {
  pub lat: f64,
  pub long: f64,
}

#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
pub struct ADSBAircraftInformation {
  pub hex: String,
  #[serde(rename = "type")]
  pub aircraft_type: Option<String>,
  pub flight: Option<String>,
  pub r: Option<String>,
  pub t: Option<String>,
  pub dbFlags: Option<u32>,
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
  type Error = Box<dyn std::error::Error>;

  fn try_from(adsb_aircraft_info: ADSBAircraftInformation) -> Result<Self, Self::Error> {
    Ok(Position {
      lat: adsb_aircraft_info.lat,
      long: adsb_aircraft_info.lon,
    })
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
