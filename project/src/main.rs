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

use crate::cli::{RpcClient, run_interactive_mode};
use crate::daemon::rpc_server::{DEFAULT_RPC_PORT, RpcServer};
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

    /// RPC server port (default: 7000)
    #[arg(long, default_value_t = DEFAULT_RPC_PORT)]
    rpc_port: u16,
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
        // Modo: apenas daemon
        Some(RunMode::Daemon) => {
            run_daemon_only(args.rpc_port).await;
        }

        // Modo: attach CLI a daemon existente
        Some(RunMode::Attach) => {
            run_cli_attached(args.rpc_port).await;
        }

        // Modo padrão: daemon + CLI interativa
        None => {
            run_daemon_with_cli(args.rpc_port).await;
        }
    }
}

/// Inicia apenas o daemon (sem CLI interativa)
async fn run_daemon_only(rpc_port: u16) {
    println!("Starting daemon...");

    init_db();

    let p2p_port = CONFIG.p2p_port;
    let main_peers = CONFIG.peers.clone();

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

    // RPC server roda indefinidamente
    if let Err(e) = rpc_server.start().await {
        eprintln!("RPC server error: {}", e);
    }
}

/// Conecta CLI a um daemon já rodando
async fn run_cli_attached(rpc_port: u16) {
    let client = RpcClient::new("127.0.0.1", rpc_port);

    // Verifica se o daemon está rodando
    if !client.ping().await {
        eprintln!("Error: Could not connect to daemon on port {}", rpc_port);
        eprintln!("Make sure the daemon is running with 'caramuru daemon'");
        return;
    }

    println!("Connected to daemon on port {}", rpc_port);

    // TODO: Implementar CLI que usa RpcClient em vez de acessar o node diretamente
    // Por enquanto, apenas mostra o status
    match client.node_status().await {
        Ok(status) => {
            println!("\n=== Node Status ===");
            println!("  Version: {}", status.version);
            println!("  Peers: {}", status.peers_connected);
            println!("  Block Height: {}", status.block_height);
            println!("  Top Hash: {}", status.top_block_hash);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    println!("\n[CLI via RPC ainda em desenvolvimento]");
    println!("Use o modo padrão (sem argumentos) para a CLI completa.");
}

async fn run_daemon_with_cli(rpc_port: u16) {
    init_db();

    let p2p_port = CONFIG.p2p_port;
    let main_peers = CONFIG.peers.clone();

    // P2P server
    tokio::spawn(async move {
        run_server(p2p_port, main_peers).await;
    });

    // RPC server em background
    tokio::spawn(async move {
        let rpc_server = RpcServer::new(rpc_port).await;
        if let Err(e) = rpc_server.start().await {
            eprintln!("[RPC] Server error: {}", e);
        }
    });

    // CLI interativa (modo antigo, acesso direto ao node)
    let cli_handle = tokio::task::spawn_blocking(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        rt.block_on(run_interactive_mode());
    });

    let _ = cli_handle.await;
}
