use std::{collections::HashMap, io::Write, sync::{Arc, Mutex}};

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
  if elapsed < args.frame_interval {
    tokio::time::sleep(args.frame_interval - elapsed).await;
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
  let sectorizer: &mut HashMap<(u16, u16), Vec<(f64, f64)>> = &mut HashMap::new();

  { // First step: generate sectorizer hashmap to correctly get braille for sub-character drawing
    let flights_data: Vec<(Position, Label)> = flights_data.lock().unwrap().flights.clone();
    
    let dots_coord: Vec<(u16, u16)> = flights_data.iter()
      .map(|(position, _)| position.as_terminal_coord(args))
      .collect();
    let dots_coord_float: Vec<(f64, f64)> = flights_data.iter()
      .map(|(position, _)| position.as_terminal_coord_float(args))
      .collect();

    let zipped_dots: Vec<((u16, u16), (f64, f64))> = dots_coord.iter()
      .zip(dots_coord_float.iter())
      .map(|((col, row), (col_float, row_float))| ((*col, *row), (*col_float, *row_float)))
      .collect();


    for (coord, coord_float) in zipped_dots {
      sectorizer.entry(coord)
        .or_insert_with(Vec::new)
        .push(coord_float);
    }
  }

  for (sector, dots_coord_float) in sectorizer.iter() {
    let braille_coords: Vec<(usize, usize)> = dots_coord_float.iter()
      .map(|(col, row)| (((col - col.trunc()) * 2.0).trunc() as usize, ((row - row.trunc()) * 4.0).trunc() as usize))
      .collect();

    queue!(
      std::io::stdout(),
      cursor::MoveTo(sector.0, sector.1),
      style::Print(generate_subchar_braille(&braille_coords)),
    )?;
  }

  Ok(())
}

fn generate_subchar_braille(braille_coords: &Vec<(usize, usize)>) -> char {
  let braille_dots_raised: &mut [[bool; 2]; 4] = &mut [[false; 2]; 4];
  braille_coords.iter().for_each(|(col, row)| braille_dots_raised[*row][*col] = true);

  let mut braille_unicode: u32 = 0;
  let mut position = 0;
  braille_dots_raised.as_flattened().iter().for_each(|&bit| {
    if bit {
      braille_unicode |= 1 << position;
    }

    position += 1;
  });

  char::from_u32(10240 + braille_unicode).unwrap()
}

