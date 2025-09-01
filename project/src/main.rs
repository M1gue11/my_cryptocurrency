mod globals;
mod model;
mod security_utils;

use crate::globals::NODE;
use crate::model::Wallet;

fn main() {
    let mut node = NODE.lock().unwrap();
    let mut w2 = Wallet::new("seed 3");

    let keys = node.miner.wallet.generate_n_keys(10);
    println!("Generated keys:");
    for key in keys {
        println!("{}", key);
    }

    if node.is_chain_empty() {
        println!("Blockchain is empty, starting with genesis block.");
        node.mine();
    }

    let origin_addr = node.miner.wallet.derive_path(&[111, 0, 0, 0, 0]);
    let tx1 = node.miner.wallet.send_tx(
        origin_addr.get_public_key(),
        w2.get_receive_addr(),
        80.0,
        Some("Payment for services".to_string()),
    );

    println!("\nReceiving transactions...");
    match node.receive_transaction(tx1.unwrap()) {
        Ok(_) => println!("Transaction received!"),
        Err(e) => println!("Error: {}", e),
    }

    node.mine();

    println!("\n--- Final Blockchain State ---");
    node.print_chain();
    // node.save_node();
}
