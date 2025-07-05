use std::{sync::{Arc, Mutex}, time::Duration};

use controller::controller_thread;
use crossterm::execute;
use model::{FlightData, Position};
use view::view_thread;

use crate::{event_dispatcher::event_dispatch_thread, model::{FRadarArgs, FRadarData, FRadarState}};

mod config;
mod controller;
mod event_dispatcher;
mod model;
mod view;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command_line_args: FRadarArgs = FRadarArgs {  // TODO: add cli parsing through `config.rs`
        origin: Position {
            lat: 37.6191,
            long: 122.3816,
        },
        radius: 250,
        data_rate: Duration::from_millis((1.0 / 1.0 * 1000.0) as u64),
        frame_rate: Duration::from_millis((1.0 / 4.0 * 1000.0) as u64),
    };

    let fradar_data: Arc<Mutex<FRadarData>> = Arc::new(Mutex::new(FRadarData {
        flights_data: Arc::new(Mutex::new(FlightData::default())),
        state: FRadarState::MAIN,
        args: command_line_args,
    }));
    
    let controller_thread_handle = controller_thread(fradar_data.clone()).await;
    let view_thread_handle = view_thread(fradar_data.clone()).await;

    let event_dispatch_thread_handle = event_dispatch_thread(fradar_data.clone()).await;

    ctrlc::set_handler(|| {
        execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
        ).unwrap();
        std::process::exit(0);
    })?;

    controller_thread_handle.await??;
    view_thread_handle.await??;

    event_dispatch_thread_handle.await?;

    Ok(())
}
