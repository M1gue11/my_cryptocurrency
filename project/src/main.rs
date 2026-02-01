mod cli;
mod daemon;
mod db;
mod globals;
mod model;
mod network;
mod security_utils;
mod utils;

#[cfg(test)]
mod tests;

use crate::cli::{RpcClient, run_cli};
use crate::daemon::rpc_server::RpcServer;
use crate::db::db::init_db;
use crate::globals::CONFIG;
use crate::network::server::run_server;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "caramuru")]
#[command(about = "Caramuru cryptocurrency node", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<RunMode>,
}

#[derive(Subcommand)]
enum RunMode {
    /// Start daemon only (no interactive CLI)
    Daemon,

    /// Attach CLI to a running daemon
    Attach,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        // daemon only
        Some(RunMode::Daemon) => {
            run_daemon_only().await;
        }

        // attach CLI to a running daemon
        Some(RunMode::Attach) => {
            run_cli_attached().await;
        }

        // daemon + CLI interativa
        None => {
            run_daemon_with_cli().await;
        }
    }
}

async fn run_daemon_only() {
    println!("Starting daemon...");

    init_db();

    let p2p_port = CONFIG.p2p_port;
    let main_peers = CONFIG.peers.clone();
    let rpc_port = CONFIG.rpc_port;

    // Inicia P2P server
    tokio::spawn(async move {
        run_server(p2p_port, main_peers).await;
    });

    // Inicia RPC server
    let rpc_server = RpcServer::new(rpc_port).await;

    println!(
        "Daemon started. P2P port: {}, RPC port: {}",
        p2p_port, rpc_port
    );
    println!("Use 'caramuru attach' to connect CLI");

    if let Err(e) = rpc_server.start().await {
        eprintln!("RPC server error: {}", e);
    }
}

async fn run_cli_attached() {
    let rpc_port = CONFIG.rpc_port;
    let client = RpcClient::new("127.0.0.1", rpc_port);

    if !client.ping().await {
        eprintln!("Error: Could not connect to daemon on port {}", rpc_port);
        eprintln!("Make sure the daemon is running with 'caramuru daemon'");
        return;
    }
    println!("Connected to daemon on port {}", rpc_port);

    run_cli(client).await;
}

async fn run_daemon_with_cli() {
    init_db();

    let p2p_port = CONFIG.p2p_port;
    let main_peers = CONFIG.peers.clone();

    // P2P server
    tokio::spawn(async move {
        run_server(p2p_port, main_peers).await;
    });

    // RPC server em background
    tokio::spawn(async move {
        let rpc_server = RpcServer::new(CONFIG.rpc_port).await;
        if let Err(e) = rpc_server.start().await {
            eprintln!("[RPC] Server error: {}", e);
        }
    });

    // CLI interativa (modo antigo, acesso direto ao node)
    let cli_handle = tokio::task::spawn_blocking(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        rt.block_on(run_cli_attached());
    });

    let _ = cli_handle.await;
}
