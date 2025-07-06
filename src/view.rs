use std::{cmp::{max, min}, io::Write, sync::{Arc, Mutex}};

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
  std::io::stdout().flush().unwrap();

  // Draw planes as dots on a radar.
  draw_radar_layer(fradar_data_locked.flights_data.clone(), args).await?;
  std::io::stdout().flush().unwrap();

  // Draw center crosshair
  draw_crosshair().await?;
  std::io::stdout().flush().unwrap();

  let elapsed = start_time.elapsed();
  if elapsed < args.frame_rate {
    tokio::time::sleep(args.frame_rate - elapsed).await;
  }

  Ok(())
}

async fn draw_crosshair() -> Result<(), Box<dyn std::error::Error>> {
  let (terminal_cols, terminal_rows) = size().unwrap();

  queue!(
    std::io::stdout(),
    cursor::MoveTo(terminal_cols / 2, terminal_rows / 2),
    style::Print("⌖"),
  )?;

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
    cursor::MoveTo(x, y + h - 1),
    style::Print("└"),
    cursor::MoveTo(x + 1, y + h - 1),
    style::Print(str::repeat("─", (w - 2).into())),
    cursor::MoveTo(x + w - 1, y + h - 1),
    style::Print("┘"),
  )?;

  Ok(())
}

async fn draw_radar_layer(flights_data: Arc<Mutex<FlightData>>, args: FRadarArgs) -> Result<(), Box<dyn std::error::Error>> {
  let (terminal_cols, terminal_rows) = size().unwrap();

  {
    let flights: Vec<Position> = flights_data.lock().unwrap().flights.clone();
    for flight in flights {
      let (col, row) = position_to_terminal_coords(flight, args);
      queue!(
        std::io::stdout(),
        cursor::MoveTo(col, row),
        style::Print("."),
      )?;
    }
  }

  Ok(())
}

fn position_to_terminal_coords(pos: Position, args: FRadarArgs) -> (u16, u16) {
  let (terminal_cols, terminal_rows) = size().unwrap();

  // Multiply delta_lat, delta_long by scale factor to get delta_col, delta_row
  let latlong_to_miles = 69.44;
  let char_aspect_ratio = 2.0;
  let lat_scale_factor: f64 = (min(terminal_cols / 2, terminal_rows / 2) as f64) / (args.radius as f64) * latlong_to_miles * char_aspect_ratio;
  let long_scale_factor: f64 = (min(terminal_cols / 2, terminal_rows / 2) as f64) / (args.radius as f64) * latlong_to_miles;

  let delta_lat = pos.lat - args.origin.lat;
  let delta_long = pos.long - args.origin.long;
  
  let delta_cols = (delta_lat * lat_scale_factor) as i32; 
  let delta_rows = (delta_long * long_scale_factor) as i32;

  let col = (terminal_cols / 2) as i32 + delta_cols;
  let row = (terminal_rows / 2) as i32 + delta_rows;

  clamp_terminal_coords(col, row)
}

fn clamp_terminal_coords(col: i32, row: i32) -> (u16, u16) {
  let (terminal_cols, terminal_rows) = size().unwrap();
  let clamped_col = min(max(col, 2), terminal_cols as i32 - 2) as u16;
  let clamped_row = min(max(row, 2), terminal_rows as i32 - 2) as u16;
  (clamped_col, clamped_row)
}

