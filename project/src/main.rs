mod config;
mod model;
mod security_utils;

use crate::model::Wallet;
use model::Node;

fn main() {
    let mut node = Node::new();
    let mut w1 = Wallet::new();
    let w2 = Wallet::new();

    if node.is_chain_empty() {
        println!("Blockchain is empty, starting with genesis block.");
        node.mine();
    }

    let tx1 = w1.send_tx(
        w2.public_key,
        50.0,
        Some("Payment for services".to_string()),
    );

    println!("\nReceiving transactions...");
    node.receive_transaction(tx1);

    node.mine();

    println!("\n--- Final Blockchain State ---");
    node.print_chain();
    node.save_node();
}
