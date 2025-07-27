use std::{collections::VecDeque, sync::{Arc, Mutex}, time::Duration};

use controller::controller_thread;
use crossterm::terminal::size;
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
        radius: 50.0,
        data_interval: Duration::from_millis((1.0 / 1.0 * 1000.0) as u64),
        frame_interval: Duration::from_millis((1.0 / 4.0 * 1000.0) as u64),
        event_interval: Duration::from_millis(100),

        terminal_cols: size()?.0,
        terminal_rows: size()?.1,

        label_label_repelling_force: 4.0,
        label_point_repelling_force: 4.0,
        label_snapping_radius: 2.0,

        history_rolling_limit: 20,
    };

    let fradar_data: Arc<Mutex<FRadarData>> = Arc::new(Mutex::new(FRadarData {
        flights_data: Arc::new(Mutex::new(FlightData::default())),
        flights_data_history: VecDeque::default(),
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
