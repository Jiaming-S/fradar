use std::sync::{Arc, Mutex};

use crossterm::{event::{read, Event, KeyCode}, execute, terminal::size};

use crate::model::{FRadarArgs, FRadarData, FRadarState, Position};

pub async fn event_dispatch_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<anyhow::Result<()>> {
  tokio::task::spawn_blocking(move || {
    while fradar_data.lock().unwrap().state != FRadarState::GracefulKill {
      let args: FRadarArgs = fradar_data.lock().unwrap().args;

      match read()? {
        Event::Key(key_event) => {
          match key_event.code {
            KeyCode::Delete | KeyCode::Esc | KeyCode::End | KeyCode::Char('q') => graceful_shutdown(fradar_data.clone()),
            KeyCode::Char('w') | KeyCode::Up    => change_origin(fradar_data.clone(), 0.0, -long_per_pixel(args)),
            KeyCode::Char('a') | KeyCode::Left  => change_origin(fradar_data.clone(), -lat_per_pixel(args), 0.0),
            KeyCode::Char('s') | KeyCode::Down  => change_origin(fradar_data.clone(), 0.0, long_per_pixel(args)),
            KeyCode::Char('d') | KeyCode::Right => change_origin(fradar_data.clone(), lat_per_pixel(args), 0.0),
            _ => continue,
          }
        },
        Event::Mouse(mouse_event) => {
          match mouse_event.kind {
              crossterm::event::MouseEventKind::ScrollDown => change_radius(fradar_data.clone(), 0.8),
              crossterm::event::MouseEventKind::ScrollUp => change_radius(fradar_data.clone(), 1.25),
              _ => continue,
          }
        },
        _ => {},
      }
    }

    Ok(())
  })
}

pub fn graceful_shutdown(fradar_data: Arc<Mutex<FRadarData>>) {
  let fradar_state: &mut FRadarState = &mut fradar_data.lock().unwrap().state;
  *fradar_state = FRadarState::GracefulKill;
  
  execute!(
    std::io::stdout(),
    crossterm::terminal::LeaveAlternateScreen,
    crossterm::cursor::Show,
    crossterm::event::DisableMouseCapture,
  ).unwrap();
  
  crossterm::terminal::disable_raw_mode().unwrap();
}

pub fn change_radius(fradar_data: Arc<Mutex<FRadarData>>, factor: f64) {
  {
    let fradar_radius: &mut u32 = &mut fradar_data.lock().unwrap().args.radius;
    *fradar_radius = (*fradar_radius as f64 * factor).max(1.0).ceil() as u32;
  }

  execute!(
    std::io::stdout(),
    crossterm::cursor::MoveTo(3, 3),
    crossterm::style::Print(fradar_data.lock().unwrap().args.radius),
  ).unwrap();
}

pub fn change_origin(fradar_data: Arc<Mutex<FRadarData>>, delta_lat: f64, delta_long: f64) {
  {
    let fradar_origin: &mut Position = &mut fradar_data.lock().unwrap().args.origin;
    (*fradar_origin).lat += delta_lat;
    (*fradar_origin).long += delta_long;
  }

  execute!(
    std::io::stdout(),
    crossterm::cursor::MoveTo(3, 4),
    crossterm::style::Print(format!("{:?}", fradar_data.lock().unwrap().args.origin)),
  ).unwrap();
}

fn lat_per_pixel(args: FRadarArgs) -> f64 {
  let terminal_cols: f64 = size().unwrap().0.into();
  let terminal_rows: f64 = size().unwrap().1.into();
  (args.radius as f64) / Position::latlong_miles_ratio() / (f64::min(terminal_cols / 2.0, terminal_rows / 2.0)) / Position::character_aspect_ratio()
}

fn long_per_pixel(args: FRadarArgs) -> f64 {
  let terminal_cols: f64 = size().unwrap().0.into();
  let terminal_rows: f64 = size().unwrap().1.into();
  (args.radius as f64) / Position::latlong_miles_ratio() / (f64::min(terminal_cols / 2.0, terminal_rows / 2.0))
}

