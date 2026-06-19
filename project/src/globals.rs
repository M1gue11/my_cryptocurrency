use once_cell::sync::Lazy;
use primitive_types::U256;
use std::env;

#[derive(Debug, Clone)]
pub struct Settings {
    pub persisted_chain_path: String,
    pub db_path: String,
    pub miner_wallet_seed_path: String,
    pub miner_wallet_password: String,
    pub max_mining_attempts: u32,
    pub p2p_port: u16,
    pub p2p_advertised_addr: String,
    pub peers: Vec<String>,
    pub max_peer_connections: Option<usize>,
    pub rpc_port: u16,
    pub http_port: u16,
    pub pbkdf2_iterations: u32,
    pub log_file_path: String,
    pub log_mode: String,
    pub mining_threads: usize,
    /// Address the RPC and HTTP servers bind to. Defaults to 127.0.0.1
    /// (loopback only). Set to 0.0.0.0 to expose the daemon inside a
    /// container so an external orchestrator can reach it (used by the
    /// distributed simulation experiment).
    pub bind_addr: String,
    pub max_fork_blocks: usize,
    /// Root directory for wallet keystores accepted from RPC requests.
    /// All user-supplied wallet paths are resolved inside this directory.
    pub wallet_keys_dir: String,
}

pub static CONFIG: Lazy<Settings> = Lazy::new(|| {
    dotenv::dotenv().ok();
    let p2p_port = env::var("P2P_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(6000);

    Settings {
        persisted_chain_path: env::var("PERSISTED_CHAIN_PATH")
            .unwrap_or_else(|_| "saved_files".to_string()),
        db_path: env::var("DB_PATH").unwrap_or_else(|_| "saved_files/bd".to_string()),
        miner_wallet_seed_path: env::var("MINER_WALLET_SEED_PATH")
            .unwrap_or_else(|_| "keys/miner_wallet.json".to_string()),
        miner_wallet_password: env::var("MINER_WALLET_PASSWORD")
            .unwrap_or_else(|_| "password123".to_string()),
        max_mining_attempts: env::var("MAX_MINING_ATTEMPTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3),
        p2p_port,
        p2p_advertised_addr: env::var("P2P_ADVERTISED_ADDR")
            .unwrap_or_else(|_| format!("127.0.0.1:{}", p2p_port)),
        peers: env::var("PEERS")
            .unwrap_or_else(|_| "".to_string())
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect(),
        max_peer_connections: env::var("MAX_PEER_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .or(Some(8)),
        rpc_port: env::var("RPC_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7000),
        http_port: env::var("HTTP_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7001),
        pbkdf2_iterations: env::var("PBKDF2_ITERATIONS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(600_000),
        log_file_path: env::var("LOG_FILE_PATH")
            .unwrap_or_else(|_| "saved_files/node.log".to_string()),
        log_mode: env::var("LOG_MODE").unwrap_or_else(|_| "full".to_string()),
        mining_threads: env::var("MINING_THREADS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1),
        bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string()),
        max_fork_blocks: env::var("MAX_FORK_BLOCKS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000),
        wallet_keys_dir: env::var("WALLET_KEYS_DIR").unwrap_or_else(|_| "keys".to_string()),
    }
});

pub struct ConsensusRules {
    /// Initial mining target. Hashes must be lower than this value.
    pub initial_target: U256,
    pub max_block_size_kb: f32,
    /// TODO: Implement block reward halving every N blocks
    pub block_reward: i64,
    /// LWMA window size: how many recent blocks to consider for difficulty adjustment
    pub lwma_n: usize,
    /// Target block time in seconds
    pub target_block_time_secs: u64,
}

pub static CONSENSUS_RULES: Lazy<ConsensusRules> = Lazy::new(|| ConsensusRules {
    // 12 leading zero bits.
    initial_target: U256::MAX >> 12u32,
    max_block_size_kb: 10.0,
    block_reward: 1 * COIN,
    lwma_n: 10,
    target_block_time_secs: 10,
});

pub const COIN: i64 = 1_000_000;
