mod config;
mod model;
mod security_utils;

use crate::model::Wallet;
use model::Node;

fn main() {
    let mut node = Node::new();
    let mut w2 = Wallet::new("seed 2");

    if node.is_chain_empty() {
        println!("Blockchain is empty, starting with genesis block.");
        node.mine();
    }

    let tx1 = node.miner.wallet.send_tx(
        w2.get_new_receive_addr(),
        80.0,
        Some("Payment for services".to_string()),
    );

    println!("\nReceiving transactions...");
    match node.receive_transaction(tx1) {
        Ok(_) => println!("Transaction received!"),
        Err(e) => println!("Error: {}", e),
    }

    node.mine();

    println!("\n--- Final Blockchain State ---");
    node.print_chain();
    // node.save_node();
}
