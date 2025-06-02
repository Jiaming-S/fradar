use std::{sync::{Arc, Mutex}, time::Duration};

use controller::controller_thread;
use crossterm::{cursor::Show, execute};
use model::{FlightData, Position};
use view::view_thread;

pub mod controller;
pub mod model;
pub mod view;

#[derive(Debug, Clone, Copy)]
pub struct Args {
    pub origin: Position,
    pub radius: i16,

    pub data_rate: Duration,
    pub frame_rate: Duration,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let flights_data: Arc<Mutex<FlightData>> = Arc::new(Mutex::new(FlightData::empty()));
    
    let command_line_args: Args = Args {  // TODO: add cli parsing through `config.rs`
        origin: Position {
            lat: 51.89508,
            long: 2.79437,
        },
        radius: 250,
        data_rate: Duration::from_secs(1),
        frame_rate: Duration::from_millis((1.0 / 60.0 * 1000.0) as u64),
    };
    
    let controller_thread_handle = controller_thread(flights_data.clone(), command_line_args).await;
    let view_thread_handle = view_thread(flights_data.clone(), command_line_args).await;

    ctrlc::set_handler(|| {
        execute!(std::io::stdout(), Show).unwrap();
        println!("Exiting...");
        std::process::exit(0);
    })?;

    controller_thread_handle.await??;
    view_thread_handle.await??;
    Ok(())
}
