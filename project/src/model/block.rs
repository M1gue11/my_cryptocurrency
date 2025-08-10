use super::Transaction;
use uuid::Uuid;

#[derive(Debug)]
pub struct Block {
    id: Uuid,
    nonce: u64,
    transactions: Vec<Transaction>,
    next: Option<Box<Block>>,
}

impl Block {
    pub fn new() -> Self {
        let id = Uuid::new_v4();
        return Block {
            id,
            nonce: 0,
            transactions: Vec::new(),
            next: None,
        };
    }

    pub fn add_transaction(&mut self, tx: Transaction) {
        self.transactions.push(tx);
    }
}
