use std::{sync::{Arc, Mutex}, time::Duration};

use controller::controller_thread;
use model::{FlightData, Position};
use view::view_thread;

use crate::{event_dispatcher::event_dispatch_thread, model::{FRadarArgs, FRadarData, FRadarState}};

mod config;
mod controller;
mod event_dispatcher;
mod model;
mod view;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    
    // TODO: add cli parsing through `config.rs`
    let command_line_args: FRadarArgs = FRadarArgs {
        origin: Position {
            lat: 37.6191,
            long: -122.3816,
        },
        radius: 100,
        data_rate: Duration::from_millis((1.0 / 1.0 * 1000.0) as u64),
        frame_rate: Duration::from_millis((1.0 / 4.0 * 1000.0) as u64),
        event_rate: Duration::from_millis(200),
    };

    let fradar_data: Arc<Mutex<FRadarData>> = Arc::new(Mutex::new(FRadarData {
        flights_data: Arc::new(Mutex::new(FlightData::default())),
        state: FRadarState::default(),
        args: command_line_args,
    }));

    let event_dispatch_thread_handle = event_dispatch_thread(fradar_data.clone()).await;    
    let controller_thread_handle = controller_thread(fradar_data.clone()).await;
    let view_thread_handle = view_thread(fradar_data.clone()).await;

    event_dispatch_thread_handle.await??;
    controller_thread_handle.await??;
    view_thread_handle.await??;

    Ok(())
}
