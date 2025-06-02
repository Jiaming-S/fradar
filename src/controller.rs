use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::time::{timeout, Instant};

use crate::{model::{ADSBData, FlightData, Position}, Args};

pub async fn controller_thread(flight_data: Arc<Mutex<FlightData>>, args: Args) -> tokio::task::JoinHandle<Result<(), reqwest::Error>> {
  tokio::spawn(async move {
    let client = reqwest::Client::new();
    
    loop {
      let start_time = Instant::now();

      let url = format!("https://api.adsb.lol/v2/point/{}/{}/{}", args.origin.lat, args.origin.long, args.radius);
      
      let request_future = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send();

      let result = match timeout(args.refresh_rate, request_future).await {
        Ok(res) => res?,
        Err(_) => {
          eprintln!("[{:?}] Request timed out (Exceeded {:?})", Utc::now().time(), args.refresh_rate);
          continue;
        }
      };

      if !result.status().is_success() {
        eprintln!("[{:?}] Request failed: {}", Utc::now().time(), result.status());
        continue;
      }
      else {
        println!("[{:?}] Request successful.", Utc::now().time());
      }

      let updated_adsb_data: ADSBData = result.json::<ADSBData>().await?;
      let updated_adsb_position_data: Result<Vec<Position>, _> = updated_adsb_data.ac.into_iter().map(Position::try_from).collect();
      
      let updated_flight_data: FlightData = FlightData { 
        flights: updated_adsb_position_data.unwrap() 
      };

      {
        let flight_data_ref: &mut FlightData = &mut flight_data.lock().unwrap();
        *flight_data_ref = updated_flight_data;
      }

      let elapsed = start_time.elapsed();
      if elapsed < args.refresh_rate {
        tokio::time::sleep(args.refresh_rate - elapsed).await;
      }
    }
  })
}
