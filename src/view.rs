use std::{cmp::min, io::Write, sync::{Arc, Mutex}};

use crossterm::{cursor, execute, queue, style::{self}, terminal::{size, Clear, ClearType}};
use tokio::time::Instant;

use crate::model::{FRadarArgs, FRadarData, FlightData, Position};


pub async fn view_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<Result<(), reqwest::Error>> {
  tokio::spawn(async move {
    crossterm::terminal::enable_raw_mode().unwrap();
    execute!(
      std::io::stdout(),
      crossterm::terminal::EnterAlternateScreen,
      crossterm::cursor::Hide,
    ).unwrap();

    loop {
      // TODO: match the error: if stdio error then ignore, if reqwest error then propogate
      draw(fradar_data.clone()).await.unwrap();
    }
  })
}

pub async fn draw(fradar_data: Arc<Mutex<FRadarData>>) -> Result<(), Box<dyn std::error::Error>> {
  let start_time = Instant::now();

  execute!(
    std::io::stdout(),
    Clear(ClearType::All),
  ).unwrap();
  
  let fradar_data_locked: FRadarData = fradar_data.lock().unwrap().clone();
  let args: FRadarArgs = fradar_data_locked.args;

  // Draw side borders. Important to do this before the radar layer.
  let (terminal_cols, terminal_rows) = size().unwrap();
  draw_box(0, 0, terminal_cols, terminal_rows).await?;

  // Draw planes as dots on a radar.
  draw_radar_layer(fradar_data_locked.flights_data.clone(), args).await?;

  let elapsed = start_time.elapsed();
  if elapsed < args.frame_rate {
    tokio::time::sleep(args.frame_rate - elapsed).await;
  }

  std::io::stdout().flush().unwrap();

  Ok(())
}

async fn draw_box(x: u16, y: u16, w: u16, h: u16) -> Result<(), Box<dyn std::error::Error>> {
  queue!(
    std::io::stdout(),
    cursor::MoveTo(x, y),
    style::Print("┌"),
    cursor::MoveTo(x + 1, y),
    style::Print(str::repeat("─", (w - 2).into())),
    cursor::MoveTo(x + w - 1, y),
    style::Print("┐"),
  )?;

  for i in (y + 1)..(y + h - 1) {
    queue!(
      std::io::stdout(),
      cursor::MoveTo(x, i),
      style::Print("│"),
      cursor::MoveTo(x + w, i),
      style::Print("│"),
    )?;
  }

  queue!(
    std::io::stdout(),
    cursor::MoveTo(x, y + h),
    style::Print("└"),
    cursor::MoveTo(x + 1, y + h),
    style::Print(str::repeat("─", (w - 2).into())),
    cursor::MoveTo(x + w - 1, y + h - 1),
    style::Print("┘"),
  )?;

  Ok(())
}

async fn draw_radar_layer(flights_data: Arc<Mutex<FlightData>>, args: FRadarArgs) -> Result<(), Box<dyn std::error::Error>> {
  let (terminal_cols, terminal_rows) = size().unwrap();
  let terminal_mid_cols = terminal_cols / 2;
  let terminal_mid_rows = terminal_rows / 2;
  let scale_factor: f64 = (args.radius as f64) / (min(terminal_cols, terminal_rows) as f64);

  {
    let flights: Vec<Position> = flights_data.lock().unwrap().flights.clone();
    for flight in flights {
      let terminal_coord = position_to_terminal_coord(flight, args.origin, scale_factor);
      queue!(
        std::io::stdout(),
        cursor::MoveTo(terminal_mid_cols + terminal_coord.0, terminal_mid_rows + terminal_coord.1),
        style::Print("."),
      )?;
    }
  }

  Ok(())
}

fn position_to_terminal_coord(pos: Position, origin: Position, scale_factor: f64) -> (u16, u16) {
  let lat_diff = pos.lat - origin.lat;
  let long_diff = pos.long - origin.long;
  ((lat_diff / scale_factor) as u16, (long_diff / scale_factor) as u16)
}

