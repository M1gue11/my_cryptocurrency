use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Cryptocurrency Node")]
#[command(about = "A CLI for interacting with cryptocurrency node functionalities", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize and start the node
    Init,
    
    /// Mining operations
    #[command(subcommand)]
    Mine(MineCommands),
    
    /// Blockchain operations
    #[command(subcommand)]
    Chain(ChainCommands),
    
    /// Wallet operations
    #[command(subcommand)]
    Wallet(WalletCommands),
    
    /// Transaction operations
    #[command(subcommand)]
    Transaction(TransactionCommands),
}

#[derive(Subcommand)]
pub enum MineCommands {
    /// Mine a new block with pending transactions
    Block,
}

#[derive(Subcommand)]
pub enum ChainCommands {
    /// Display the entire blockchain
    Show,
    
    /// Validate the blockchain integrity
    Validate,
    
    /// Save the blockchain to disk
    Save,
    
    /// Get blockchain status
    Status,
}

#[derive(Subcommand)]
pub enum WalletCommands {
    /// Create a new wallet
    New {
        /// Seed phrase for wallet generation
        #[arg(short, long)]
        seed: String,
    },
    
    /// Get a new receive address from the miner's wallet
    Address,
    
    /// Check wallet balance
    Balance {
        /// Wallet seed phrase
        #[arg(short, long)]
        seed: String,
    },
    
    /// Send a transaction
    Send {
        /// Recipient address
        #[arg(short, long)]
        to: String,
        
        /// Amount to send
        #[arg(short, long)]
        amount: f64,
        
        /// Optional message
        #[arg(short, long)]
        message: Option<String>,
    },
    
    /// Generate n keys from the miner's wallet
    GenerateKeys {
        /// Number of keys to generate
        #[arg(short, long, default_value = "5")]
        count: u32,
    },
}

#[derive(Subcommand)]
pub enum TransactionCommands {
    /// View transaction details by ID (hex format)
    View {
        /// Transaction ID in hex format
        #[arg(short, long)]
        id: String,
    },
    
    /// List all pending transactions in mempool
    Pending,
}
