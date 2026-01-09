mod db;
mod front;
mod globals;
mod model;
mod network;
mod security_utils;
mod utils;

#[cfg(test)]
mod tests;

use std::env;

use crate::{db::db::init_db, globals::CONFIG, network::server::run_p2p_server};
use front::run_interactive_mode;

#[tokio::main]
async fn main() {
    init_db();
    let args: Vec<String> = env::args().collect();
    let mut port = CONFIG.p2p_port;
    let mut peers = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(6000);
                    i += 1;
                }
            }
            "--peer" => {
                if i + 1 < args.len() {
                    peers.push(args[i + 1].clone());
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let p2p_peers = peers.clone();
    tokio::spawn(async move {
        run_p2p_server(port, p2p_peers).await;
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
