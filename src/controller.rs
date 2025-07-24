use std::sync::{Arc, Mutex};

use chrono::Utc;
use tokio::time::{timeout, Instant};

use crate::model::{ADSBData, FRadarArgs, FRadarData, FRadarState, FlightData, Label, Position};


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

      let result = match timeout(args.data_interval, request_future).await {
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

      let updated_adsb_position_data: Vec<Position> = updated_adsb_data.ac.clone()
        .into_iter()
        .map(Position::try_from)
        .collect::<anyhow::Result<Vec<Position>>>()?;

      let updated_adsb_label_data: Vec<Label> = updated_adsb_data.ac.clone()
        .into_iter()
        .map(Label::try_from)
        .collect::<anyhow::Result<Vec<Label>>>()?;

      let unified_data: Vec<(Position, Label)> = updated_adsb_position_data
        .iter()
        .zip(updated_adsb_label_data.iter())
        .map(|(position, label)| (position.clone(), label.clone()))
        .collect();
      
      let updated_flights_data: FlightData = FlightData { 
        flights: unified_data,
        epoch_timestamp: Utc::now().timestamp_millis(),
      };

      {
        let flights_data: Arc<Mutex<FlightData>> = fradar_data.lock().unwrap().flights_data.clone();
        let flights_data_ref: &mut FlightData = &mut flights_data.lock().unwrap();
        *flights_data_ref = updated_flights_data;
      }

      // TODO: revisit this logic, do we need to force data rate?
      let elapsed = start_time.elapsed();
      if elapsed < args.data_interval {
        tokio::time::sleep(args.data_interval - elapsed).await;
      }
    }

    Ok(())
  })
}
