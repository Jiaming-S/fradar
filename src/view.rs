use std::{collections::HashMap, io::Write, sync::{Arc, Mutex}};

use crossterm::{cursor, execute, queue, style::{self}, terminal::{Clear, ClearType}};
use tokio::{time::Instant};

use crate::model::{Coord, FRadarArgs, FRadarData, FRadarState, FlightData, Label, LabelPosition, Position};


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
  
  let args: FRadarArgs;

  {
    let fradar_data_locked: FRadarData = fradar_data.lock().unwrap().clone();
    let flights_data: Arc<Mutex<FlightData>> = fradar_data_locked.flights_data;
    args = fradar_data_locked.args;


    // Draw planes as dots on a radar.
    draw_radar_layer(flights_data.clone(), args)?;
  }

  // Draw side borders.
  draw_box_with_label(0, 0, args.terminal_cols, args.terminal_rows, " fradar ".to_string())?;

  // Draw center crosshair
  draw_crosshair(&args)?;
  
  std::io::stdout().flush()?;

  let elapsed = start_time.elapsed();
  if elapsed < args.frame_interval {
    tokio::time::sleep(args.frame_interval - elapsed).await;
  }

  Ok(())
}

fn draw_crosshair(args: &FRadarArgs) -> anyhow::Result<()> {
  if args.origin.roughly_eq(&args.starting_origin) {
    queue!(
      std::io::stdout(),
      cursor::MoveTo(args.terminal_cols / 2, args.terminal_rows / 2),
      style::Print("●︎"),
    )?;
  }
  else {
    let lat_threshold: f64  = 0.1 / Position::latlong_miles_ratio() / Position::character_aspect_ratio();
    let long_threshold: f64 = 0.1 / Position::latlong_miles_ratio();
    let c: char = match (args.starting_origin.lat - args.origin.lat, args.starting_origin.long - args.origin.long) {
      (delta_lat, delta_long) if delta_lat >  lat_threshold && delta_long < -long_threshold => '↖',
      (delta_lat, delta_long) if delta_lat >  lat_threshold && delta_long >  long_threshold => '↗',
      (delta_lat, delta_long) if delta_lat < -lat_threshold && delta_long >  long_threshold => '↘',
      (delta_lat, delta_long) if delta_lat < -lat_threshold && delta_long < -long_threshold => '↙',
      (delta_lat,  _) if delta_lat  >  lat_threshold  => '↑',
      (delta_lat,  _) if delta_lat  < -lat_threshold  => '↓',
      (_, delta_long) if delta_long >  long_threshold => '→',
      (_, delta_long) if delta_long < -long_threshold => '←',
      _ => '?',
    };
    
    queue!(
      std::io::stdout(),
      cursor::MoveTo(args.terminal_cols / 2, args.terminal_rows / 2),
      style::Print(c),
    )?;
  }

  Ok(())
}

fn draw_box_with_label(x: u16, y: u16, w: u16, h: u16, label: String) -> anyhow::Result<()> {
  draw_box(x, y, w, h)?;
  queue!(
    std::io::stdout(),
    // cursor::MoveTo(x + w / 2 - label.len() as u16 / 2, y),
    cursor::MoveTo(x + 2, y),
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
  let flights_data: Vec<(Position, Label)> = flights_data.lock().unwrap().flights.clone();

  // Preemptive step: spin up label engine
  let label_engine_handle = label_engine(flights_data.clone(), args);

  // First step: generate sectorizer hashmap to correctly get braille for sub-character drawing
  let sectorizer: &mut HashMap<Coord<u16>, Vec<Coord<f64>>> = &mut HashMap::new();

  let dots_coord: Vec<Coord<u16>> = flights_data.iter()
    .map(|(position, _)| position.as_terminal_coord(&args).unwrap()) // TODO: handle the unwrap from converting float to coord
    .collect();
  let dots_coord_float: Vec<Coord<f64>> = flights_data.iter()
    .map(|(position, _)| position.as_terminal_coord_float(&args))
    .collect();

  let zipped_dots: Vec<(Coord<u16>, Coord<f64>)> = dots_coord.iter()
    .zip(dots_coord_float.iter())
    .map(|(coord, coord_float)| (*coord, *coord_float))
    .collect();

  for (coord, coord_float) in zipped_dots {
    sectorizer.entry(coord)
      .or_insert_with(Vec::new)
      .push(coord_float);
  }

  // Second step: using sectorizer hashmap, determine what braille character to display, and queue draw to stdout
  for (sector, dots_coord_float) in sectorizer.iter() {
    let braille_coords: Vec<Coord<usize>> = dots_coord_float.iter()
      .map(|coord_float| Coord::<usize> {
        col: ((coord_float.col - coord_float.col.trunc()) * 2.0).trunc() as usize, // extract only decimal place, then
        row: ((coord_float.row - coord_float.row.trunc()) * 4.0).trunc() as usize, // multiple by col or row respectively
      })
      .collect();

    queue!(
      std::io::stdout(),
      cursor::MoveTo(sector.col, sector.row),
      style::Print(generate_subchar_braille(&braille_coords)),
    )?;
  }

  // Third step: join label engine thread
  label_engine_handle.join().unwrap()?;

  Ok(())
}

fn label_engine(flights_data: Vec<(Position, Label)>, args: FRadarArgs) -> std::thread::JoinHandle<anyhow::Result<()>> {
  std::thread::spawn(move || -> anyhow::Result<()> {
    for (position, label) in flights_data.iter() {
      // Moving ("pushed") coordinate for this label
      let pushed_coord = &mut position.as_terminal_coord_float(&args);

      // Initialize to the "top right"
      pushed_coord.col += 1.0;
      pushed_coord.row -= 1.0;
      
      // Simulate inverse gravitational forces between labels and points
      for (other_position, _) in flights_data.iter() {
        let other_coord_float: Coord<f64> = other_position.as_terminal_coord_float(&args);
        let hypot_squared: f64 = pushed_coord.squared_dist(other_coord_float);
        if hypot_squared > 0.1 {
          pushed_coord.col -= args.label_point_repelling_force *
            (other_coord_float.col - pushed_coord.col) /
            hypot_squared;
          pushed_coord.row -= args.label_point_repelling_force *
            (other_coord_float.row - pushed_coord.row) /
            hypot_squared;
        }
      }

      let original_coord: Coord<f64> = position.as_terminal_coord_float(&args);
      let label_position = match (pushed_coord.col - original_coord.col, pushed_coord.row - original_coord.row) {
        (dc, dr) if (dc > 0.0 && dr > 0.0) => LabelPosition::BottomRight,
        (dc, dr) if (dc < 0.0 && dr > 0.0) => LabelPosition::BottomLeft,
        (dc, dr) if (dc > 0.0 && dr < 0.0) => LabelPosition::TopRight,
        (dc, dr) if (dc < 0.0 && dr < 0.0) => LabelPosition::TopLeft,
        (_, _) => LabelPosition::default(),
      };

      let (del_col, del_row) = label.compute_display_delta(label_position);
      let (res_col, res_row) = (del_col as f64 + original_coord.col, del_row as f64 + original_coord.row);
      if res_col < 3.0 || res_col + label.len() as f64 > -3.0 + args.terminal_cols as f64 ||
         res_row < 3.0 || res_row + label.height() as f64 > -3.0 + args.terminal_rows as f64 {
        continue;
      }

      let mut do_draw = true;
      for (other_position, _) in flights_data.iter() {
        if other_position.as_terminal_coord(&args)?.is_in_box(res_col as u16, res_row as u16, label.len() as u16, label.height() as u16) {
          do_draw = false;
        }
      }

      if !do_draw {
        continue;
      }

      let label_string: String = label.to_string(label_position);
      label_string.split("\n").enumerate().for_each(|(ind, str)| {
          queue!(
            std::io::stdout(),
            cursor::MoveTo(res_col as u16, (res_row as usize + ind) as u16),
            style::Print(str),
          ).unwrap()
      });
    }

    Ok(())
  })
}

fn generate_subchar_braille(braille_coords: &Vec<Coord<usize>>) -> char {
  let mut braille_unicode: u32 = 0;

  for &coord in braille_coords {
    let bit_index = match (coord.col, coord.row) {
      (0, 0) => 0,
      (0, 1) => 1,
      (0, 2) => 2,
      (1, 0) => 3,
      (1, 1) => 4,
      (1, 2) => 5,
      (0, 3) => 6,
      (1, 3) => 7,
      _ => continue,
    };

    braille_unicode |= 1 << bit_index;
  }

  char::from_u32(0x2800 + braille_unicode).unwrap_or(' ')
}

