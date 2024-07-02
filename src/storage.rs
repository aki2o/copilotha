use std::path::PathBuf;

use tokio::sync::mpsc;

pub enum Command {
  Embedded { file: Box<PathBuf>, language: String, updated_at: u64, scores: Vec<f64> },
}

pub async fn start(mut rx: mpsc::Receiver<Command>) {
  while let Some(command) = rx.recv().await {
    match command {
      Command::Embedded { file, language, updated_at, scores } => {
        println!("Embedded file: {:?}, language: {}, updated_at: {}", file, language, updated_at);
      }
    }
  }
}
