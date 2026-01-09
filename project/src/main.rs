mod db;
mod front;
mod globals;
mod model;
mod network;
mod security_utils;
mod utils;

#[cfg(test)]
mod tests;

use crate::db::db::init_db;
use front::run_interactive_mode;

#[tokio::main]
async fn main() {
    init_db();
    run_interactive_mode().await;
}
