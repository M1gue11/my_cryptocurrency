mod db;
mod front;
mod globals;
mod model;
mod network;
mod security_utils;
mod utils;

#[cfg(test)]
mod tests;

use crate::{db::db::init_db, globals::CONFIG, network::server::run_server};
use front::run_interactive_mode;

#[tokio::main]
async fn main() {
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
