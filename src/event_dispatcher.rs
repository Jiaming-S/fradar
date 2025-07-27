use std::sync::{Arc, Mutex};

use crossterm::{event::{read, Event, KeyCode}, execute};

use crate::model::{FRadarArgs, FRadarData, FRadarState, Position};

pub async fn event_dispatch_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<anyhow::Result<()>> {
  tokio::task::spawn_blocking(move || {
    while fradar_data.lock().unwrap().state != FRadarState::GracefulKill {
      let args: FRadarArgs = fradar_data.lock().unwrap().args;

      match read()? {
        Event::Key(key_event) => {
          match key_event.code {
            KeyCode::Delete | KeyCode::Esc | KeyCode::End | KeyCode::Char('q') => graceful_shutdown(fradar_data.clone()),
            KeyCode::Char('w') | KeyCode::Up    => change_origin(fradar_data.clone(),  lat_per_pixel(&args),  0.0),
            KeyCode::Char('s') | KeyCode::Down  => change_origin(fradar_data.clone(), -lat_per_pixel(&args),  0.0),
            KeyCode::Char('a') | KeyCode::Left  => change_origin(fradar_data.clone(),  0.0, -long_per_pixel(&args)),
            KeyCode::Char('d') | KeyCode::Right => change_origin(fradar_data.clone(),  0.0,  long_per_pixel(&args)),
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
        Event::Resize(new_width, new_height) => change_term_size(fradar_data.clone(), new_width, new_height),
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
    let fradar_args: &mut FRadarArgs = &mut fradar_data.lock().unwrap().args;
    fradar_args.radius = fradar_args.radius as f64 * factor;
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

pub fn change_term_size(fradar_data: Arc<Mutex<FRadarData>>, new_width: u16, new_height: u16) {
  {
    let fradar_args: &mut FRadarArgs = &mut fradar_data.lock().unwrap().args;
    fradar_args.terminal_cols = new_width;
    fradar_args.terminal_rows = new_height;
  }
}

fn lat_per_pixel(args: &FRadarArgs) -> f64 {
  (args.radius as f64) /
    Position::latlong_miles_ratio() /
    (f64::min(args.terminal_cols as f64 / 2.0, args.terminal_rows as f64 / 2.0)) /
    Position::character_aspect_ratio()
}

fn long_per_pixel(args: &FRadarArgs) -> f64 {
  (args.radius as f64) /
    Position::latlong_miles_ratio() /
    (f64::min(args.terminal_cols as f64 / 2.0, args.terminal_rows as f64 / 2.0))
}

