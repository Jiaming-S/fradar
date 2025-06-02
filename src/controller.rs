use std::sync::{Arc, Mutex};

use chrono::Utc;

use crate::{model::{ADSBData, FlightData, Position}, Args};

pub async fn controller_thread(flight_data: Arc<Mutex<FlightData>>, args: Args) -> tokio::task::JoinHandle<Result<(), reqwest::Error>> {
  tokio::spawn(async move {
    let client = reqwest::Client::new();
    
    loop {
      let url = format!("https://api.adsb.lol/v2/point/{}/{}/{}", args.origin.lat, args.origin.long, args.radius);
      let res = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await?;

      if !res.status().is_success() {
        eprintln!("[{:?}] Request failed: {}", Utc::now().time(), res.status());
        continue;
      }

      let updated_adsb_data: ADSBData = res.json::<ADSBData>().await?;
      let updated_adsb_position_data: Result<Vec<Position>, _> = updated_adsb_data.ac.into_iter().map(Position::try_from).collect();
      
      let updated_flight_data: FlightData = FlightData { 
        flights: updated_adsb_position_data.unwrap() 
      };

      {
        let flight_data_ref: &mut FlightData = &mut flight_data.lock().unwrap();
        *flight_data_ref = updated_flight_data;
      }
    }
  })
}
