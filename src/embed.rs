use crate::storage;
use crate::util;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum Command {
  Embed(EmbedCommand),
}

#[derive(Debug)]
pub struct EmbedCommand {
  value: EmbedFile,
  session_id: String,
  token_rx: oneshot::Receiver<String>,
  trials: u8,
}

#[derive(Debug)]
pub struct EmbedFile {
  pub path: Box<PathBuf>,
  pub language: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbeddingRequest {
  input: Vec<String>,
  model: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbeddingResponse {
  data: Vec<EmbeddedScores>,
  usage: EmbeddedTokens,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbeddedScores {
  index: u16,
  embedding: Vec<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbeddedTokens {
  prompt_tokens: u32,
  total_tokens: u32,
}

pub async fn start(mut rx: mpsc::Receiver<Command>, auth_tx: mpsc::Sender<oneshot::Sender<String>>, retry_tx: mpsc::Sender<Command>, storage_tx: mpsc::Sender<storage::Command>) {
  while let Some(command) = rx.recv().await {
    match command {
      Command::Embed(command) => {
        let auth_tx2 = auth_tx.clone();
        let retry_tx2 = retry_tx.clone();
        let storage_tx2 = storage_tx.clone();

        tokio::spawn(async move {
          match command.token_rx.await {
            Ok(token) => {
              let model = "copilot-text-embedding-ada-002";
              let body = EmbeddingRequest {
                model: model.to_string(),
                input: vec![command.value.to_input()],
              };
              match embedding(token, command.session_id.clone(), body).await {
                Ok(res) => {
                  let updated_at = util::now();
                  let stored = storage::Command::Embedded {
                    file: command.value.path.clone(),
                    language: command.value.language.clone(),
                    updated_at: updated_at,
                    scores: res.data[0].embedding.clone(),
                  };

                  storage_tx2.send(stored).await.unwrap();
                }
                Err(e) => {
                  tracing::error!("Failed to request embed : {:?}", e);
                  if command.trials <= 3 {
                    tokio::time::sleep(tokio::time::Duration::from_secs((command.trials * 30).into())).await;

                    let (tx, rx) = oneshot::channel::<String>();

                    let c = EmbedCommand {
                      value: EmbedFile {
                        path: command.value.path.clone(),
                        language: command.value.language.clone(),
                      },
                      session_id: command.session_id.clone(),
                      token_rx: rx,
                      trials: command.trials + 1,
                    };

                    auth_tx2.send(tx).await.unwrap();
                    retry_tx2.send(Command::Embed(c)).await.unwrap();
                  }
                }
              }
            }
            Err(e) => {
              tracing::error!("Failed to receive token : {:?}", e);
              if command.trials <= 3 {
                tokio::time::sleep(tokio::time::Duration::from_secs((command.trials * 30).into())).await;

                let (tx, rx) = oneshot::channel::<String>();

                let c = EmbedCommand {
                  value: EmbedFile {
                    path: command.value.path.clone(),
                    language: command.value.language.clone(),
                  },
                  session_id: command.session_id.clone(),
                  token_rx: rx,
                  trials: command.trials + 1,
                };

                auth_tx2.send(tx).await.unwrap();
                retry_tx2.send(Command::Embed(c)).await.unwrap();
              }
            }
          }
        });
      }
    }
  }
}

impl EmbedFile {
  pub fn to_input(&self) -> String {
    let text = std::fs::read_to_string(self.path.as_ref()).unwrap();

    return format!("File: `{}`\n```{}\n{}\n```\n", self.path.clone().into_os_string().into_string().unwrap(), self.language, text);
  }
}

#[tracing::instrument]
async fn embedding(token: String, session_id: String, body: EmbeddingRequest) -> anyhow::Result<EmbeddingResponse> {
  let client = reqwest::Client::new();
  let mut req = client.post("https://api.githubcopilot.com/embeddings").json(&body);
  let headers = util::generate_headers(token, session_id).await;

  for (k, v) in headers.iter() {
    req = req.header(k, v);
  }

  let res = req.send().await?;

  if !res.status().is_success() {
    return Err(anyhow!("[{:?}] {:?}", res.status(), res.text().await));
  }

  return Ok(res.json::<EmbeddingResponse>().await?);
}

// use reqwest::Client;
// use serde::{Deserialize, Serialize};
// use uuid::Uuid;

// #[derive(Serialize, Deserialize, Debug)]
// struct AskRequest {
//     intent: bool,
//     model: String,
//     n: u32,
//     stream: bool,
//     temperature: f32,
//     top_p: f32,
//     messages: Vec<Message>,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct Message {
//     content: String,
//     role: String,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct AskInput {
//     prompt: String,
//     model: String,
//     temperature: f32,
//     // Add other fields as necessary
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct AskResponse {
//     // Define the response structure
// }

// async fn ask(client: &Client, input: AskInput, token: &str) -> Result<AskResponse, reqwest::Error> {
//     let response = client
//         .post("https://api.githubcopilot.com/chat/completions")
//         .bearer_auth(token)
//         .json(&input)
//         .send()
//         .await?
//         .json::<AskResponse>()
//         .await?;

//     Ok(response)
// }

// #[tokio::main]
// async fn main() -> Result<(), reqwest::Error> {
//     let client = Client::new();
//     let token = "your_github_copilot_token_here"; // You need to handle authentication and token retrieval
//     let session_id = Uuid::new_v4().to_string();
//     let machine_id = Uuid::new_v4().to_string();

//     let ask_request = AskRequest {
//         intent: true,
//         model: "gpt-4".to_string(),
//         n: 1,
//         stream: true,
//         temperature: 0.1,
//         top_p: 1.0,
//         messages: vec![Message {
//             content: "Your prompt here".to_string(),
//             role: "user".to_string(),
//         }],
//     };

//     let response = client
//         .post("https://api.githubcopilot.com/chat/completions")
//         .bearer_auth(token)
//         .header("x-request-id", Uuid::new_v4().to_string())
//         .header("vscode-sessionid", session_id)
//         .header("vscode-machineid", machine_id)
//         .header("copilot-integration-id", "vscode-chat")
//         .header("openai-organization", "github-copilot")
//         .header("openai-intent", "conversation-panel")
//         .header("content-type", "application/json")
//         .json(&ask_request)
//         .send()
//         .await?;

//     let response_text = response.text().await?;
//     println!("Response: {}", response_text);

//     Ok(())
// }
