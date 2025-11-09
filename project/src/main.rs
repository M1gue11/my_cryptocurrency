mod globals;
mod model;
mod security_utils;
mod utils;

use crate::model::{TxOutput, Wallet, get_node_mut, init_node};

fn main() {
    init_node();
    let node = get_node_mut();
    let mut w2 = Wallet::new("seed 3");

    if node.is_chain_empty() {
        println!("Blockchain is empty, starting with genesis block.");
        node.mine();
    }

    let outputs = vec![TxOutput {
        value: 100.0,
        address: w2.get_receive_addr(),
    }];
    let tx = node
        .miner
        .wallet
        .send_tx(outputs, Some("Test transaction".to_string()));

    match node.receive_transaction(tx.unwrap()) {
        Ok(_) => println!("Transaction received!"),
        Err(e) => println!("Error: {}", e),
    }

    node.mine();

    println!("\n--- Final Blockchain State ---");
    node.print_chain();
    node.save_node();
}
