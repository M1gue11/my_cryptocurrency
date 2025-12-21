mod bd;
mod front;
mod globals;
mod model;
mod security_utils;
mod utils;

#[cfg(test)]
mod tests;

use std::process::exit;

use bd::Db;
use front::run_interactive_mode;

use crate::model::node;

fn main() {
    node::init_node();
    let node = node::get_node();
    let result = node
        .miner
        .wallet
        .owns_address("116rfoz1ZV2VyD4vLmpPGHCNF661NPLTzH4");
    println!("Address ownership result: {:?}", result);
    exit(0);
    let db = Db::open(None).unwrap();
    let _ = db.init_schema();

    run_interactive_mode();
}
