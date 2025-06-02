use std::{io::Write, sync::{Arc, Mutex}};

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
    Hide,
    Clear(ClearType::All)
  ).unwrap();

  Ok(())
}

pub async fn draw(flights_data: Arc<Mutex<FlightData>>, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
  let start_time = Instant::now();

  let terminal_size = size().unwrap();
  for col in 1..(terminal_size.0 + 1) {
    for row in 1..(terminal_size.1 + 1) {
      if (col <= 2 || col >= terminal_size.0 - 2) || (row == 1 || row == terminal_size.1) {
        queue!(
          std::io::stdout(),
          cursor::MoveTo(col, row),
          style::PrintStyledContent("█".dark_grey())
        )?;
      }
      else {
        queue!(
          std::io::stdout(),
          cursor::MoveTo(col, row),
          style::Print(" ")
        )?;
      }
    }
  }




  let elapsed = start_time.elapsed();
  if elapsed < args.frame_rate {
    tokio::time::sleep(args.frame_rate - elapsed).await;
  }

  std::io::stdout().flush().unwrap();

  Ok(())
}

