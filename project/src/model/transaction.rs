use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub amount: f64,
    pub date: NaiveDateTime,
    pub destination_addr: String,
    pub origin_addr: String,
    pub signature: String,
}

impl Transaction {
    pub fn new(amount: f64, destination_addr: String, origin_addr: String) -> Self {
        // TODO: Implement signature generation and validation
        let id = Uuid::new_v4();
        let date = Utc::now().naive_utc();
        return Transaction {
            id,
            amount,
            date,
            destination_addr,
            origin_addr,
            signature: String::new(),
        };
    }
}
