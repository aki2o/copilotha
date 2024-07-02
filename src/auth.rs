use crate::config;
use crate::util;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct Authenticator {
  github_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Token {
  token: String,
  expires_at: u64,
}

pub async fn start(mut rx: mpsc::Receiver<oneshot::Sender<String>>) {
  let github_token = find_github_token();

  if github_token.is_none() {
    panic!("Can't authenticate : Not found github token");
  }

  let authenticator = Authenticator { github_token: github_token.unwrap() };

  let mut token = Token {
    token: "".to_string(),
    expires_at: util::now(),
  };

  while let Some(tx) = rx.recv().await {
    if !token.is_available() {
      token = authenticator.auth().await.unwrap();
    }

    tx.send(token.token.clone()).unwrap();
  }
}

impl Token {
  pub fn is_available(&self) -> bool {
    return self.expires_at > util::now();
  }
}

impl Authenticator {
  #[tracing::instrument]
  pub async fn auth(&self) -> anyhow::Result<Token> {
    let client = reqwest::Client::new();
    let mut req = client
      .get("https://api.github.com/copilot_internal/v2/token")
      .header("Authorization", format!("token {}", self.github_token.as_str()))
      .header("Accept", "application/json");

    for (k, v) in util::version_headers().iter() {
      req = req.header(k, v);
    }

    let res = req.send().await?;

    if !res.status().is_success() {
      return Err(anyhow!(format!("Failed to request token : {}", res.status())));
    }

    let token = res.json::<Token>().await?;

    return Ok(token);
  }
}

#[tracing::instrument]
fn find_github_token() -> Option<String> {
  // Loading token from the environment only in GitHub Codespaces
  let token = env::var("GITHUB_TOKEN").ok();
  let codespaces = env::var("CODESPACES").is_ok();
  if token.is_some() && codespaces {
    return token;
  }

  // Loading token from the file
  let file_path = config::root().join("github-copilot/hosts.json");
  if !file_path.is_file() {
    return None;
  }

  let data = fs::read_to_string(file_path).ok()?;
  let userdata: Value = serde_json::from_str(&data).ok()?;

  return userdata["github.com"]["oauth_token"].as_str().map(|s| s.to_string());
}
