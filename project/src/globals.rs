use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub persisted_chain_path: String,
    pub db_path: String,
    pub miner_wallet_seed_path: String,
    pub miner_wallet_password: String,
    pub max_mining_attempts: u32,
}

pub static CONFIG: Lazy<Settings> = Lazy::new(|| {
    let settings = config::Config::builder()
        .add_source(config::File::with_name("Settings.toml"))
        .build()
        .unwrap();
    settings.try_deserialize().unwrap()
});

pub struct ConsensusRules {
    /// Difficulty level for mining in number of leading zero bits
    pub difficulty: usize,
    pub max_block_size_kb: f32,
    pub block_reward: i64,
}

pub static CONSENSUS_RULES: Lazy<ConsensusRules> = Lazy::new(|| ConsensusRules {
    difficulty: 8,
    max_block_size_kb: 1.0,
    block_reward: 100,
});

pub const COIN: i64 = 1_000_000;

// pub static NODE: Lazy<RwLock<Node>> = Lazy::new(|| RwLock::new(Node::new()));
