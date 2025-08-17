use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    /// Difficulty level for mining in number of leading zero bits
    pub difficulty: usize,
    pub node_name: String,
    pub persisted_chain_path: String,
}

pub static CONFIG: Lazy<Settings> = Lazy::new(|| {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("Settings.toml"))
        .build()
        .unwrap();
    settings.try_deserialize().unwrap()
});
