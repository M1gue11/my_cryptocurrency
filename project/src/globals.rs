use crate::model::Node;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
pub struct Settings {
    /// Difficulty level for mining in number of leading zero bits
    pub difficulty: usize,
    pub persisted_chain_path: String,
    pub block_reward: f64,
}

pub static CONFIG: Lazy<Settings> = Lazy::new(|| {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("Settings.toml"))
        .build()
        .unwrap();
    settings.try_deserialize().unwrap()
});

pub static NODE: Lazy<Mutex<Node>> = Lazy::new(|| Mutex::new(Node::new()));
