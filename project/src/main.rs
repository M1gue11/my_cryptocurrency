mod cli;
mod common;
mod daemon;
mod db;
mod front;
mod globals;
mod model;
mod network;
mod security_utils;
mod utils;

#[cfg(test)]
mod tests;

use clap::{Parser, Subcommand};
use daemon::lifecycle::{handle_daemon_command, DaemonCommand};

// CLI Command enum for passing to execute_command
pub enum CliCommand {
    Node(NodeSubcommands),
    Mine(MineSubcommands),
    Chain(ChainSubcommands),
    Wallet(WalletSubcommands),
}

#[derive(Parser)]
#[command(name = "caramuru")]
#[command(about = "Caramuru cryptocurrency node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Daemon control commands
    Daemon {
        #[command(subcommand)]
        cmd: DaemonSubcommands,
    },
    /// Node operations
    Node {
        #[command(subcommand)]
        cmd: NodeSubcommands,
    },
    /// Mining operations
    Mine {
        #[command(subcommand)]
        cmd: MineSubcommands,
    },
    /// Blockchain operations
    Chain {
        #[command(subcommand)]
        cmd: ChainSubcommands,
    },
    /// Wallet operations
    Wallet {
        #[command(subcommand)]
        cmd: WalletSubcommands,
    },
}

#[derive(Subcommand)]
enum DaemonSubcommands {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status,
    /// Restart the daemon
    Restart,
}

#[derive(Subcommand)]
enum NodeSubcommands {
    /// Show node status
    Status,
}

#[derive(Subcommand)]
enum MineSubcommands {
    /// Mine a new block
    Block,
}

#[derive(Subcommand)]
enum ChainSubcommands {
    /// Show blockchain status
    Status,
}

#[derive(Subcommand)]
enum WalletSubcommands {
    /// Check wallet balance
    Balance {
        /// Wallet name (optional, defaults to miner_wallet)
        #[arg(long)]
        name: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Daemon { cmd }) => {
            let daemon_cmd = match cmd {
                DaemonSubcommands::Start => DaemonCommand::Start,
                DaemonSubcommands::Stop => DaemonCommand::Stop,
                DaemonSubcommands::Status => DaemonCommand::Status,
                DaemonSubcommands::Restart => DaemonCommand::Restart,
            };

            if let Err(e) = handle_daemon_command(daemon_cmd).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Node { cmd }) => {
            use crate::cli::execute_command;
            if let Err(e) = execute_command(CliCommand::Node(cmd)).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Mine { cmd }) => {
            use crate::cli::execute_command;
            if let Err(e) = execute_command(CliCommand::Mine(cmd)).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Chain { cmd }) => {
            use crate::cli::execute_command;
            if let Err(e) = execute_command(CliCommand::Chain(cmd)).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Some(Commands::Wallet { cmd }) => {
            use crate::cli::execute_command;
            if let Err(e) = execute_command(CliCommand::Wallet(cmd)).await {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        None => {
            // Default behavior: run interactive mode (for backwards compatibility)
            use crate::{db::db::init_db, globals::CONFIG, network::server::run_server};
            use front::run_interactive_mode;

            init_db();
            let port = CONFIG.p2p_port;
            let main_peers = CONFIG.peers.clone();

            tokio::spawn(async move {
                run_server(port, main_peers).await;
            });

            let cli_handle = tokio::task::spawn_blocking(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap();
                rt.block_on(run_interactive_mode());
            });

            // Waits for the CLI to finish
            let _ = cli_handle.await;
        }
    }
}
