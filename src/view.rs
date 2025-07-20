use std::{io::{Write}, sync::{Arc, Mutex}};

use crossterm::{cursor, execute, queue, style::{self}, terminal::{size, Clear, ClearType}};
use tokio::time::Instant;

use crate::model::{FRadarArgs, FRadarData, FRadarState, FlightData, Label, Position};


pub async fn view_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<anyhow::Result<()>> {
  tokio::spawn(async move {
    crossterm::terminal::enable_raw_mode()?;
    execute!(
      std::io::stdout(),
      crossterm::terminal::EnterAlternateScreen,
      crossterm::cursor::Hide,
      crossterm::event::EnableMouseCapture,
    )?;

    while fradar_data.lock().unwrap().state != FRadarState::GracefulKill {
      // TODO: match the error: if stdio error then ignore, if reqwest error then propogate
      draw(fradar_data.clone()).await?;
    }

    Ok(())
  })
}

pub async fn draw(fradar_data: Arc<Mutex<FRadarData>>) -> anyhow::Result<()> {
  let start_time = Instant::now();

  queue!(
    std::io::stdout(),
    Clear(ClearType::All),
  )?;
  
  let fradar_data_locked: FRadarData = fradar_data.lock().unwrap().clone();
  let args: FRadarArgs = fradar_data_locked.args;
  let (terminal_cols, terminal_rows) = size()?;

  // Draw planes as dots on a radar.
  draw_radar_layer(fradar_data_locked.flights_data.clone(), args)?;

  // Draw side borders.
  draw_box_with_label(0, 0, terminal_cols, terminal_rows, " fradar ".to_string())?;

  // Draw center crosshair
  draw_crosshair()?;
  
  std::io::stdout().flush()?;

  let elapsed = start_time.elapsed();
  if elapsed < args.frame_rate {
    tokio::time::sleep(args.frame_rate - elapsed).await;
  }

  Ok(())
}

fn draw_crosshair() -> anyhow::Result<()> {
  let (terminal_cols, terminal_rows) = size()?;

  queue!(
    std::io::stdout(),
    cursor::MoveTo(terminal_cols / 2, terminal_rows / 2),
    style::Print("⌖"),
  )?;

  Ok(())
}

fn draw_box_with_label(x: u16, y: u16, w: u16, h: u16, label: String) -> anyhow::Result<()> {
  draw_box(x, y, w, h)?;
  queue!(
    std::io::stdout(),
    cursor::MoveTo(x + w / 2 - label.len() as u16 / 2, y),
    style::Print(label),
  )?;

  Ok(())
}

fn draw_box(x: u16, y: u16, w: u16, h: u16) -> anyhow::Result<()> {
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

fn draw_radar_layer(flights_data: Arc<Mutex<FlightData>>, args: FRadarArgs) -> anyhow::Result<()> {
  let (terminal_cols, terminal_rows) = size()?;

  {
    let flights: Vec<Position> = flights_data.lock().unwrap().flights.clone();
    let labels: Vec<Label> = flights_data.lock().unwrap().labels.clone();
    let flights_terminal_pos: Vec<(f64, f64)> = flights.iter().map(|flight| position_to_terminal_pos(*flight, args)).collect();

    for (i, flight) in flights_terminal_pos.clone().iter().enumerate() {
      queue!(
        std::io::stdout(),
        cursor::MoveTo(flight.0 as u16, flight.1 as u16),
        style::Print("•"),
      )?;

      let mut do_label = true;
      for other_flight in flights_terminal_pos.clone() {
        if terminal_coord_within_box(
          other_flight.0 as u16,
          other_flight.1 as u16,
          (flight.0 + 1.0) as u16,
          (flight.1 + 1.0) as u16,
          labels[i].len() as u16,
          5) {
          do_label = false;
          break;
        }
      }

      if do_label {
        labels[i].draw_righthanded(flight.0 as u16 + 1, flight.1 as u16 + 1);
      }
    }
  }

  Ok(())
}

fn position_to_terminal_pos(pos: Position, args: FRadarArgs) -> (f64, f64) {
  let terminal_cols: f64 = size().unwrap().0.into();
  let terminal_rows: f64 = size().unwrap().1.into();

  // Multiply delta_lat, delta_long by scale factor to get delta_col, delta_row
  let latlong_to_miles: f64 = 69.44;
  let char_aspect_ratio: f64 = 2.0; // TODO: dynamically find value
  let lat_scale_factor: f64 = (f64::min(terminal_cols / 2.0, terminal_rows / 2.0)) / (args.radius as f64) * latlong_to_miles * char_aspect_ratio;
  let long_scale_factor: f64 = (f64::min(terminal_cols / 2.0, terminal_rows / 2.0)) / (args.radius as f64) * latlong_to_miles;

  let delta_lat = pos.lat - args.origin.lat;
  let delta_long = pos.long - args.origin.long;
  
  let delta_cols = delta_lat * lat_scale_factor; 
  let delta_rows = delta_long * long_scale_factor;

  let col = terminal_cols / 2.0 + delta_cols;
  let row = terminal_rows / 2.0 + delta_rows;

  clamp_terminal_coords(col, row)
}

fn clamp_terminal_coords(col: f64, row: f64) -> (f64, f64) {
  let terminal_cols: f64 = size().unwrap().0.into();
  let terminal_rows: f64 = size().unwrap().1.into();

  let clamped_col = col.clamp(0.0, terminal_cols);
  let clamped_row = row.clamp(0.0, terminal_rows);

  (clamped_col, clamped_row)
}

fn terminal_coord_within_box(col: u16, row: u16, x: u16, y: u16, w: u16, h: u16) -> bool {
  if col >= x && col <= x + w &&
     row >= y && row <= y + h {
    return true
  }
  false
}

