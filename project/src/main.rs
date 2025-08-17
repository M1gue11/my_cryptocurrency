mod config;
mod model;
mod security_utils;

use crate::model::Transaction;
use model::Node;

fn main() {
    let mut node = Node::new();

    if node.is_chain_empty() {
        println!("Blockchain is empty, starting with genesis block.");
        node.mine();
    }

    let is_chain_valid = node.validate_blockchain();
    println!("Is blockchain valid? {}", is_chain_valid);

    println!("\nReceiving transactions...");
    node.receive_transaction(Transaction::new(8.0, "miguel".into(), "mauro".into(), None));

    node.mine();

    println!("\n--- Final Blockchain State ---");
    node.print_chain();
    node.save_node();
}
