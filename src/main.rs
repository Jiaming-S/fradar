use std::{sync::{Arc, Mutex}, thread, time::Duration};

use controller::controller_thread;
use model::{FlightData, Position};

pub mod controller;
pub mod model;
pub mod view;

#[derive(Debug, Clone)]
pub struct Args {
    pub origin: Position,
    pub radius: i16,
    pub refresh_rate: Duration,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flight_data: Arc<Mutex<FlightData>> = Arc::new(Mutex::new(FlightData::empty()));
    
    let command_line_args: Args = Args {  // TODO: add cli parsing through `config.rs`
        origin: Position {
            lat: 51.89508,
            long: 2.79437,
        },
        radius: 250,
        refresh_rate: Duration::from_secs(1),
    };
    
    let controller_thread_handle = controller_thread(flight_data, command_line_args).await;
    
    thread::sleep(Duration::from_secs(1));

    controller_thread_handle.await??;
    Ok(())
}
