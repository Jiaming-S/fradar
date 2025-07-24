use std::sync::{Arc, Mutex};

use crossterm::{event::{poll, read, Event, KeyCode}, execute};

use crate::model::{FRadarArgs, FRadarData, FRadarState};

pub async fn event_dispatch_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<anyhow::Result<()>> {
  tokio::spawn(async move {
    while fradar_data.lock().unwrap().state != FRadarState::GracefulKill {
      let args: FRadarArgs = fradar_data.lock().unwrap().args;

      if poll(args.event_interval)? {
        match read()? {
          Event::Key(key_event) => {
            match key_event.code {
              KeyCode::Delete | KeyCode::Esc | KeyCode::End | KeyCode::Char('q') => graceful_shutdown(fradar_data.clone()),
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
    *fradar_radius = (*fradar_radius as f64 * factor).max(1.0) as u32;
  }

  execute!(
    std::io::stdout(),
    crossterm::cursor::MoveTo(3, 3),
    crossterm::style::Print(fradar_data.lock().unwrap().args.radius),
  ).unwrap();
}

