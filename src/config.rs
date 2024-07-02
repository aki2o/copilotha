use garde::Validate;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::sync::OnceCell;

#[derive(Debug)]
pub struct Config {
  pub debug: bool,
  pub values: Values,
}

#[derive(Serialize, Deserialize, Validate, Debug)]
pub struct Values {
  #[garde(required)]
  pub allow_insecure: Option<bool>,
  #[garde(required)]
  pub model: Option<String>,
  #[garde(required)]
  pub temperature: Option<f32>,
  #[garde(required)]
  pub machine_id: Option<String>,
}

static INSTANCE: OnceCell<Config> = OnceCell::const_new();

pub fn current() -> &'static Config {
  return INSTANCE.get().unwrap();
}

pub fn setup() {
  let f = file_path();

  if !f.exists() {
    create();
  }

  INSTANCE.set(load()).unwrap();
}

pub fn root() -> Box<PathBuf> {
  if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME") {
    let xdg_path = Path::new(&xdg_config_home);

    if xdg_path.exists() && xdg_path.is_dir() {
      return Box::new(xdg_path.to_path_buf());
    }
  }

  #[cfg(target_os = "windows")]
  {
    let home_dir = dirs::home_dir().unwrap();

    return Box::new(home_dir.join("AppData").join("Local"));
  }

  #[cfg(not(target_os = "windows"))]
  {
    let home_dir = dirs::home_dir().unwrap();

    return Box::new(home_dir.join(".config"));
  }
}

fn file_path() -> Box<PathBuf> {
  return Box::new(root().join("copilotha.toml"));
}

#[tracing::instrument]
fn load() -> Config {
  let f = file_path();
  let s = fs::read_to_string(*f.clone()).expect(&format!("Failed to read {:?}", f));
  let values: Values = toml::from_str(&s).expect(&format!("Failed to load config from {:?}", f));

  if let Err(e) = values.validate(&()) {
    panic!("Invalid config {:?} : {e}", f);
  }
  tracing::debug!("Loaded");

  return Config { debug: false, values: values };
}

#[tracing::instrument]
fn create() {
  let f = file_path();

  let v = Values {
    allow_insecure: Some(false),
    model: Some("gpt-3.5".to_string()),
    temperature: Some(0.1),
    machine_id: Some(generate_machine_id()),
  };

  create_file(f, toml::to_string(&v).unwrap());
  tracing::info!("Done");
}

fn create_file(f: Box<PathBuf>, s: String) {
  let error_message = format!("Failed to write {:?}", f);
  let mut fs = File::create(*f.clone()).expect(&error_message);
  write!(fs, "{}", s).expect(&error_message);
  fs.flush().expect(&error_message);
  println!("Saved {:?}", f);
}

fn generate_machine_id() -> String {
  let mut rng = rand::thread_rng();

  return std::iter::repeat(()).map(|()| rng.sample(Alphanumeric)).filter(|c| c.is_ascii_hexdigit()).map(char::from).take(64).collect();
}
