use std::{cmp::min, io::Write, sync::{Arc, Mutex}};

use crossterm::{cursor::{self, Hide}, execute, queue, style::{self}, terminal::{size, Clear, ClearType}};
use tokio::time::Instant;

use crate::model::{FRadarArgs, FRadarData, FlightData, Position};


pub async fn view_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<Result<(), reqwest::Error>> {
  tokio::spawn(async move {
    execute!(
      std::io::stdout(),
      crossterm::terminal::EnterAlternateScreen,
      Hide,
    ).unwrap();

    loop {
      // TODO: match the error: if stdio error then ignore, if reqwest error then propogate
      draw(fradar_data.clone()).await.unwrap();
    }
  })
}

pub async fn draw(fradar_data: Arc<Mutex<FRadarData>>) -> Result<(), Box<dyn std::error::Error>> {
  let start_time = Instant::now();

  let args: FRadarArgs = fradar_data.lock().unwrap().args;
  draw_borders().await?;

  let (terminal_cols, terminal_rows) = size().unwrap();
  let terminal_mid_cols = terminal_cols / 2;
  let terminal_mid_rows = terminal_rows / 2;
  let scale_factor: f64 = (args.radius as f64) / (min(terminal_cols, terminal_rows) as f64);




  let elapsed = start_time.elapsed();
  if elapsed < args.frame_rate {
    tokio::time::sleep(args.frame_rate - elapsed).await;
  }

  std::io::stdout().flush().unwrap();

  Ok(())
}

async fn draw_borders() -> Result<(), Box<dyn std::error::Error>> {
  execute!(
    std::io::stdout(),
    Clear(ClearType::All),
  ).unwrap();

  let (terminal_cols, terminal_rows) = size().unwrap();
  for col in 0..(terminal_cols) {
    for row in 0..(terminal_rows) {
      let printed_character = match (col, row) {
        (0, 0) => "┌",
        (c, 0) if c == terminal_cols - 1 => "┐",
        (0, r) if r == terminal_rows - 1 => "└",
        (c, r) if c == terminal_cols - 1 && r == terminal_rows - 1 => "┘",
        (_, r) if r == terminal_rows - 1 || r == 0 => "─",
        (c, _) if c == terminal_cols - 1 || c == 0 => "│",
        _ => continue,
      };

      queue!(
        std::io::stdout(),
        cursor::MoveTo(col, row),
        style::Print(printed_character)
      )?;
    }
  }

  Ok(())
}

async fn draw_radar_layer(flights_data: FlightData, origin: Position) -> Result<(), Box<dyn std::error::Error>> {
  Ok(())
}

async fn position_to_terminal_coord(pos: Position, origin: Position, scale_factor: f64) -> (i32, i32) {
  let lat_diff = pos.lat - origin.lat;
  let long_diff = pos.long - origin.long;
  ((lat_diff / scale_factor) as i32, (long_diff / scale_factor) as i32)
}

