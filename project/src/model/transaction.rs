use crate::{
    globals::CONFIG,
    model::{TxInput, TxOutput},
    security_utils::{
        digest_to_hex_string, load_public_key_from_hex, load_signature_from_hex, sha256,
        verify_signature,
    },
};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub date: NaiveDateTime,
    pub message: Option<String>,
}

impl Transaction {
    pub fn new(inputs: Vec<TxInput>, outputs: Vec<TxOutput>, message: Option<String>) -> Self {
        let date = Utc::now().naive_utc();
        Transaction {
            inputs,
            outputs,
            date,
            message,
        }
    }

    pub fn id(&self) -> [u8; 32] {
        sha256(&self.as_bytes())
    }

    pub fn new_coinbase(miner_address: String) -> Self {
        let date = Utc::now().naive_utc();
        let inputs = Vec::new();
        let reward_amount = CONFIG.block_reward;
        let outputs = vec![TxOutput {
            value: reward_amount,
            address: miner_address,
        }];
        let message = Some("Coinbase transaction".to_string());

        Transaction {
            date,
            inputs,
            outputs,
            message,
        }
    }

    pub fn validate(&self) -> bool {
        let partial_tx = Transaction {
            inputs: self.inputs.iter().map(|i| i.get_partial()).collect(),
            outputs: self.outputs.clone(),
            date: self.date,
            message: self.message.clone(),
        };
        let partial_tx_bytes = partial_tx.as_bytes();
        for input in &self.inputs {
            let pubkey = load_public_key_from_hex(&input.public_key);
            let sig = load_signature_from_hex(&input.signature);

            if !verify_signature(&pubkey, &partial_tx_bytes, sig) {
                return false;
            }
        }
        true
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let data = format!(
            "{:?}{:?}{}{:?}",
            self.inputs, self.outputs, self.date, self.message
        );
        data.as_bytes().to_vec()
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Transaction {{ id: {}, inputs: {:?}, outputs: {:?}, date: {}, message: {:?} }}",
            digest_to_hex_string(&self.id()),
            self.inputs,
            self.outputs,
            self.date,
            self.message
        )
    }
}

impl std::fmt::Debug for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Transaction")
            .field("id", &digest_to_hex_string(&self.id()))
            .field("inputs", &self.inputs)
            .field("outputs", &self.outputs)
            .field("date", &self.date)
            .field("message", &self.message)
            .finish()
    }
}
