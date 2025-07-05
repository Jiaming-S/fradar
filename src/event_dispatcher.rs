use std::sync::{Arc, Mutex};

use crossterm::{event::{poll, read, Event, KeyCode}, execute};

use crate::model::{FRadarArgs, FRadarData, FRadarState};

pub async fn event_dispatch_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<Result<(), std::io::Error>> {
  tokio::spawn(async move {
    loop {
      let args: FRadarArgs = fradar_data.lock().unwrap().args;

      if poll(args.event_rate)? {
        match read()? {
          Event::Key(key_event) => {
            match key_event.code {
              KeyCode::Delete | KeyCode::Esc | KeyCode::End | KeyCode::Char('q') => {
                println!("Received exit key, awaiting graceful shutdown...");
                graceful_shutdown(fradar_data.clone()).await;
              },
              _ => continue,
            }
          },
          _ => {},
        }
      }
    }
  })
}

pub async fn graceful_shutdown(fradar_data: Arc<Mutex<FRadarData>>) {
  let fradar_state: &mut FRadarState = &mut fradar_data.lock().unwrap().state;
  *fradar_state = FRadarState::GracefulKill;
  
  execute!(
    std::io::stdout(),
    crossterm::terminal::LeaveAlternateScreen,
    crossterm::cursor::Show,
  ).unwrap();
  
  crossterm::terminal::disable_raw_mode().unwrap();

  std::process::exit(0);
}



