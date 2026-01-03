use crate::{
    globals::CONFIG,
    model::{TxInput, TxOutput, UTXO},
    security_utils::{
        digest_to_hex_string, load_public_key_from_hex, load_signature_from_hex, sha256,
        verify_signature,
    },
    utils::format_date,
};
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

pub type TxId = [u8; 32];
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

    pub fn id(&self) -> TxId {
        sha256(&self.as_bytes())
    }

    pub fn new_coinbase(miner_address: String, fees: f64) -> Self {
        let date = Utc::now().naive_utc();
        let inputs = Vec::new();
        let reward_amount = CONFIG.block_reward + fees;
        let outputs = vec![TxOutput {
            value: reward_amount,
            address: miner_address,
        }];
        let message = Some("Coinbase and fees".to_string());

        Transaction {
            date,
            inputs,
            outputs,
            message,
        }
    }

    pub fn validate(&self) -> bool {
        // TODO: improve error messages
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

    pub fn amount(&self) -> f64 {
        self.outputs.iter().map(|o| o.value).sum()
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for input in &self.inputs {
            out.extend_from_slice(&input.as_bytes());
        }
        for output in &self.outputs {
            out.extend_from_slice(&TxOutput::as_bytes(output));
        }
        out.extend_from_slice(format_date(&self.date).as_bytes());
        out.extend_from_slice(self.message.as_ref().unwrap_or(&String::new()).as_bytes());
        out
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.is_empty()
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

#[derive(Serialize, Deserialize, Clone)]
pub struct MempoolTx {
    pub tx: Transaction,
    pub utxos: Vec<UTXO>,
}
impl MempoolTx {
    pub fn new(tx: Transaction, utxos: Vec<UTXO>) -> Self {
        MempoolTx { tx, utxos }
    }

    pub fn calculate_fee(&self) -> f64 {
        let input_sum: f64 = self.utxos.iter().map(|u| u.output.value).sum();
        let output_sum: f64 = self.tx.outputs.iter().map(|o| o.value).sum();
        input_sum - output_sum
    }
}
