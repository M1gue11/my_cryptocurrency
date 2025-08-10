use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

#[derive(Debug)]
pub struct Transaction {
    pub id: Uuid,
    pub amount: f64,
    pub date: NaiveDateTime,
    pub destination_addr: String,
    pub origin_addr: String,
}

impl Transaction {
    pub fn new(amount: f64, destination_addr: String, origin_addr: String) -> Self {
        let id = Uuid::new_v4();
        let date = Utc::now().naive_utc();
        return Transaction {
            id,
            amount,
            date,
            destination_addr,
            origin_addr,
        };
    }
}
