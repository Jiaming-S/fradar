use std::{cmp::min, default, io::Write, sync::{Arc, Mutex}};

use crossterm::{cursor::{self, Hide}, execute, queue, style::{self, Stylize}, terminal::{size, Clear, ClearType}};
use tokio::time::Instant;

use crate::{model::{FlightData, Position}, Args};


pub async fn view_thread(flights_data: Arc<Mutex<FlightData>>, args: Args) -> tokio::task::JoinHandle<Result<(), reqwest::Error>> {
  tokio::spawn(async move {
    startup().await?;
    loop {
      draw(flights_data.clone(), &args).await.unwrap(); // TODO: match the error: if stdio error then ignore, if reqwest error then propogate
    }
  })
}

pub async fn startup() -> Result<(), reqwest::Error> {
  execute!(
    std::io::stdout(),
    // Hide,
    Clear(ClearType::All)
  ).unwrap();

  Ok(())
}

pub async fn draw(flights_data: Arc<Mutex<FlightData>>, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
  let start_time = Instant::now();

  fresh_terminal().await?;

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

async fn fresh_terminal() -> Result<(), Box<dyn std::error::Error>> {
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

async fn position_to_terminal_coord(pos: Position, scale_factor: f64) {

}

