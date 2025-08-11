mod model;

use model::Blockchain;
use model::Miner;

use crate::model::Transaction;

fn main() {
    let mut bc = Blockchain::new();
    let miner = Miner::new("MinerAddress".into());
    let difficulty = 4;

    println!("Blockchain initialized with genesis block");

    let t1 = Transaction::new(100.0, "Alice".into(), "Bob".into());
    println!("Transaction added to mempool: {:?}", t1);
    bc.add_transaction_to_mempool(t1);

    println!("Mining block...");
    miner.mine(bc.get_mempool(), bc.get_last_block_hash(), difficulty);
    bc.add_block(mined_block, difficulty);

    println!("Updated state of blockchain: {:?}", bc);
}
