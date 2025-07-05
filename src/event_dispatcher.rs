use std::sync::{Arc, Mutex};

use crate::model::{FRadarData};

pub async fn event_dispatch_thread(fradar_data: Arc<Mutex<FRadarData>>) -> tokio::task::JoinHandle<()> {
  tokio::spawn(async move {
    
  })
}
