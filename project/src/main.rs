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
use crate::daemon::http_server::HttpServer;
use crate::daemon::rpc_server::RpcServer;
use crate::db::db::init_db;
use crate::globals::CONFIG;
use crate::network::server::run_server;
use crate::utils::PidFile;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "caramuru")]
#[command(about = "Caramuru cryptocurrency node", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start daemon in a specific mode
    Daemon {
        #[command(subcommand)]
        mode: DaemonMode,
    },

    /// Attach CLI to a running daemon (requires RPC mode)
    Attach,
}

#[derive(Subcommand)]
enum DaemonMode {
    /// HTTP mode - for frontend communication (port 7001)
    Http,

    /// RPC mode - for CLI communication (port 7000)
    Rpc,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        // daemon http mode
        Some(Command::Daemon {
            mode: DaemonMode::Http,
        }) => {
            run_daemon_http().await;
        }

        // daemon rpc mode
        Some(Command::Daemon {
            mode: DaemonMode::Rpc,
        }) => {
            run_daemon_rpc().await;
        }

        // attach CLI to a running daemon
        Some(Command::Attach) => {
            run_cli_attached().await;
        }

        // default: daemon + CLI (RPC only, no HTTP)
        None => {
            run_daemon_with_cli().await;
        }
    }
}

/// Daemon com HTTP server (para frontend)
async fn run_daemon_http() {
    let _pid_file = PidFile::create_or_exit("caramuru.pid");
    init_db();

    let p2p_port = CONFIG.p2p_port;
    let main_peers = CONFIG.peers.clone();
    let http_port = CONFIG.http_port;

    // P2P server
    tokio::spawn(async move {
        run_server(p2p_port, main_peers).await;
    });

    println!(
        "Daemon started (HTTP mode). P2P: {}, HTTP: {}",
        p2p_port, http_port
    );
    println!("Frontend can connect at http://localhost:{}/rpc", http_port);

    // HTTP server (blocking)
    let http_server = HttpServer::new(http_port);
    if let Err(e) = http_server.start().await {
        eprintln!("[HTTP] Server error: {}", e);
    }
}

/// Daemon com RPC server (para CLI)
async fn run_daemon_rpc() {
    let _pid_file = PidFile::create_or_exit("caramuru.pid");
    init_db();

    let p2p_port = CONFIG.p2p_port;
    let main_peers = CONFIG.peers.clone();
    let rpc_port = CONFIG.rpc_port;

    // P2P server
    tokio::spawn(async move {
        run_server(p2p_port, main_peers).await;
    });

    println!(
        "Daemon started (RPC mode). P2P: {}, RPC: {}",
        p2p_port, rpc_port
    );
    println!("Use 'caramuru attach' to connect CLI");

    // RPC server (blocking)
    let rpc_server = RpcServer::new(rpc_port).await;
    if let Err(e) = rpc_server.start().await {
        eprintln!("[RPC] Server error: {}", e);
    }
}

/// Attach CLI to running daemon
async fn run_cli_attached() {
    let rpc_port = CONFIG.rpc_port;
    let client = RpcClient::new("127.0.0.1", rpc_port);

    if !client.ping().await {
        eprintln!("Error: Could not connect to daemon on port {}", rpc_port);
        eprintln!("Make sure the daemon is running with 'caramuru daemon rpc'");
        return;
    }
    println!("Connected to daemon on port {}", rpc_port);

    run_cli(client).await;
}

/// Daemon + CLI integrada (modo desenvolvimento, RPC only)
async fn run_daemon_with_cli() {
    let _pid_file = PidFile::create_or_exit("caramuru.pid");
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

    // CLI interativa
    let cli_handle = tokio::task::spawn_blocking(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        rt.block_on(run_cli_attached());
    });

    let _ = cli_handle.await;
}
