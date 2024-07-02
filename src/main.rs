use copilotha::auth;
use copilotha::config;
use copilotha::embed;
use copilotha::storage;
use copilotha::util;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use uuid::Uuid;

#[tokio::main]
async fn main() {
  config::setup();

  setup_tracing();

  let session_id = format!("{}{}", Uuid::new_v4(), util::now());
  let (auth_tx, auth_rx) = mpsc::channel::<oneshot::Sender<String>>(100);
  let (embed_tx, embed_rx) = mpsc::channel::<embed::Command>(100);
  let (storage_tx, storage_rx) = mpsc::channel::<storage::Command>(100);

  tokio::spawn(async move {
    auth::start(auth_rx).await;
  });

  let embed_tx2 = embed_tx.clone();
  tokio::spawn(async move {
    embed::start(embed_rx, embed_tx2, storage_tx).await;
  });

  tokio::spawn(async move {
    storage::start(storage_rx).await;
  });

  loop {
    let mut input = String::new();

    print!("> ");
    io::stdout().flush().unwrap(); // Ensure the prompt is displayed immediately
    io::stdin().read_line(&mut input).unwrap();

    let words: Vec<&str> = input.trim().split_whitespace().collect();

    match words.get(0) {
      Some(&"exit") => {
        println!("Goodbye!");
        return;
      }
      Some(&"open") => {
        let file = words.get(1).unwrap();
        let (tx, rx) = oneshot::channel::<String>();

        let c = embed::Command::Embed {
          value: embed::EmbedFile {
            path: Box::new(PathBuf::from(file)),
            language: "rust".to_string(),
          },
          session_id: session_id.clone(),
          token_rx: rx,
          trials: 1,
        };

        auth_tx.send(tx).await.unwrap();
        embed_tx.send(c).await.unwrap();
      }
      _ => {
        println!("Unknown command");
      }
    }
  }
}

fn setup_tracing() {
  tracing_subscriber::fmt().with_max_level(tracing::Level::TRACE).init();

  // let file_appender = tracing_appender::rolling::hourly("/tmp/copilotha", "prefix.log");
  // let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

  // let subscriber = tracing_subscriber::fmt()
  //   .with_max_level(tracing::Level::TRACE)
  //   .with_writer(non_blocking)
  //   .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
  //   .finish();

  // tracing::subscriber::set_global_default(subscriber).unwrap();
}
