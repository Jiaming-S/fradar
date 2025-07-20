use std::sync::{Arc, Mutex};

use tokio::time::{timeout, Instant};

use crate::model::{ADSBData, FRadarArgs, FRadarData, FRadarState, FlightData, Position};


pub async fn controller_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<anyhow::Result<()>> {
  tokio::spawn(async move {
    let client = reqwest::Client::new();
    
    while fradar_data.lock().unwrap().state != FRadarState::GracefulKill {
      let start_time = Instant::now();

      let args: FRadarArgs = fradar_data.lock().unwrap().args;

      let url = format!("https://api.adsb.lol/v2/point/{}/{}/{}", args.origin.lat, args.origin.long, args.radius);
      let request_future = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send();

      let result = match timeout(args.data_rate, request_future).await {
        Ok(res) => res?,
        Err(_) => {
          // eprintln!("[{:?}] Request timed out (Exceeded {:?})", Utc::now().time(), args.data_rate);
          continue;
        }
      };

      if !result.status().is_success() {
        // eprintln!("[{:?}] Request failed: {}", Utc::now().time(), result.status());
        continue;
      }
      else {
        // println!("[{:?}] Request successful.", Utc::now().time());
      }

      let updated_adsb_data: ADSBData = result.json::<ADSBData>().await?;
      let updated_adsb_position_data: Result<Vec<Position>, _> = updated_adsb_data.ac.into_iter().map(Position::try_from).collect();
      
      let updated_flights_data: FlightData = FlightData { 
        flights: updated_adsb_position_data.unwrap() 
      };

      {
        let flights_data: Arc<Mutex<FlightData>> = fradar_data.lock().unwrap().flights_data.clone();
        let flights_data_ref: &mut FlightData = &mut flights_data.lock().unwrap();
        *flights_data_ref = updated_flights_data;
      }

      // TODO: revisit this logic, do we need to force data rate?
      let elapsed = start_time.elapsed();
      if elapsed < args.data_rate {
        tokio::time::sleep(args.data_rate - elapsed).await;
      }
    }

    Ok(())
  })
}
