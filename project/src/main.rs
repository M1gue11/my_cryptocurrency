mod config;
mod model;

use crate::model::Transaction;
use model::Node;

fn main() {
    let mut node = Node::new();

    println!("\nReceiving transactions...");
    node.receive_transaction(Transaction::new(10.5, "dest1".into(), "origA".into()));
    node.receive_transaction(Transaction::new(5.0, "dest2".into(), "origB".into()));

    node.mine();

    println!("\n--- Final Blockchain State ---");
    node.print_chain();
    node.save_node();
}
