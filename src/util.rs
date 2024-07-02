use std::{
  collections::HashMap,
  time::{SystemTime, UNIX_EPOCH},
};

use uuid::Uuid;

use crate::config;

pub fn now() -> u64 {
  return SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
}

pub async fn generate_headers(token: String, session_id: String) -> HashMap<String, String> {
  let mut headers = HashMap::new();

  headers.insert("authorization".to_string(), format!("Bearer {}", token.as_str()));
  headers.insert("x-request-id".to_string(), Uuid::new_v4().to_string());
  headers.insert("vscode-sessionid".to_string(), session_id);
  headers.insert("vscode-machineid".to_string(), config::current().values.machine_id.clone().unwrap());
  headers.insert("copilot-integration-id".to_string(), "vscode-chat".to_string());
  headers.insert("openai-organization".to_string(), "github-copilot".to_string());
  headers.insert("openai-intent".to_string(), "conversation-panel".to_string());
  headers.insert("content-type".to_string(), "application/json".to_string());

  for (k, v) in version_headers().iter() {
    headers.insert(k.clone(), v.clone());
  }

  return headers;
}

pub fn version_headers() -> HashMap<String, String> {
  let mut headers = HashMap::new();

  headers.insert("editor-version".to_string(), "Neovim/8.0.0".to_string());
  headers.insert("editor-plugin-version".to_string(), "CopilotChat.nvim/2.0.0".to_string());
  headers.insert("user-agent".to_string(), "CopilotChat.nvim/2.0.0".to_string());

  return headers;
}
