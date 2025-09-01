use crate::{
    globals::CONFIG,
    security_utils::{
        digest_to_hex_string, load_public_key_from_hex, load_signature_from_hex, sha256,
        verify_signature,
    },
};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: [u8; 32],
    pub amount: f64,
    pub date: NaiveDateTime,
    pub destination_addr: String,
    pub origin_addr: Option<String>,
    pub signature: String,
    pub message: Option<String>,
}

impl Transaction {
    pub fn new(
        amount: f64,
        destination_addr: String,
        origin_addr: String,
        message: Option<String>,
    ) -> Self {
        let date = Utc::now().naive_utc();
        let origin_addr_opt = Some(origin_addr);
        let data = format!(
            "{}{}{:?}{:?}",
            amount, date, destination_addr, origin_addr_opt
        );
        let id = sha256(data.as_bytes());

        Transaction {
            id,
            amount,
            date,
            destination_addr,
            origin_addr: origin_addr_opt,
            signature: String::new(),
            message,
        }
    }

    pub fn new_coinbase(miner_address: String) -> Self {
        let date = Utc::now().naive_utc();
        let origin_addr_opt = None;
        let reward_amount = CONFIG.block_reward;
        let data = format!(
            "{}{}{:?}{:?}",
            reward_amount, date, miner_address, origin_addr_opt
        );
        let id = sha256(data.as_bytes());

        Transaction {
            id,
            amount: reward_amount,
            date,
            destination_addr: miner_address,
            origin_addr: origin_addr_opt,
            signature: String::new(),
            message: None,
        }
    }

    pub fn verify_tx(&self) -> bool {
        let signature = load_signature_from_hex(&self.signature);
        let origin_addr = self.origin_addr.clone().unwrap_or_default();
        let pk = load_public_key_from_hex(origin_addr);
        verify_signature(&pk, self.to_string().as_bytes(), signature)
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {:?} {:?} {}",
            digest_to_hex_string(&self.id),
            self.amount,
            self.origin_addr,
            self.destination_addr,
        )
    }
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("id", &digest_to_hex_string(&self.id))
            .field("amount", &self.amount)
            .field("date", &self.date)
            .field("destination_addr", &self.destination_addr)
            .field("origin_addr", &self.origin_addr)
            .field("signature", &self.signature)
            .field("message", &self.message)
            .finish()
    }
}
