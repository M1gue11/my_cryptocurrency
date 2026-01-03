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
    #[command(subcommand)]
    Node(NodeCommands),

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
pub enum NodeCommands {
    Init,

    Mempool,

    ClearMempool,
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

    /// Rollback the blockchain by N blocks (for debugging)
    Rollback {
        /// Number of blocks to rollback
        #[arg(short, long)]
        count: u32,
    },

    Utxos {
        /// Limit the number of UTXOs displayed
        #[arg(short, long, default_value_t = 20)]
        limit: u32,
    },
}

#[derive(Subcommand)]
pub enum WalletCommands {
    /// Create a new wallet
    New {
        /// Seed phrase for wallet generation
        #[arg(short, long)]
        seed: String,

        #[arg(short, long)]
        name: Option<String>,
    },

    List,

    /// Get a new receive address from the miner's wallet
    Address {
        /// Name of the wallet to get the address from
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Check wallet balance
    Balance {
        /// Wallet seed phrase
        #[arg(short, long)]
        seed: String,
    },

    /// Send a transaction
    Send {
        /// Name of the wallet to send from
        #[arg(short, long)]
        from: Option<String>,

        /// Recipient address
        #[arg(short, long)]
        to: String,

        /// Amount to send
        #[arg(short, long)]
        amount: f64,

        /// Transaction fee
        #[arg(short, long)]
        fee: Option<f64>,

        /// Optional message
        #[arg(short, long)]
        message: Option<String>,
    },

    /// Generate n keys from the miner's wallet
    GenerateKeys {
        /// Number of keys to generate
        #[arg(short, long, default_value = "5")]
        count: u32,

        /// Name of the wallet to generate keys from
        #[arg(short, long)]
        name: Option<String>,

        /// Type of derivation (0 = receive, 1 = change)
        #[arg(short, long, default_value = "0")]
        type_: Option<u32>,
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
}
