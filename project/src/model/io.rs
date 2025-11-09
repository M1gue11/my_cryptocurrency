use serde::{Deserialize, Serialize};

use crate::security_utils::digest_to_hex_string;

#[derive(Clone, Serialize, Deserialize)]
pub struct TxInput {
    pub prev_tx_id: [u8; 32], // previous transaction ID
    pub output_index: usize,  // spent output index
    pub signature: String,    // owner's signature
    pub public_key: String,   // owner's public key
}
impl TxInput {
    pub fn get_partial(&self) -> TxInput {
        TxInput {
            prev_tx_id: self.prev_tx_id,
            output_index: self.output_index,
            signature: String::new(),
            public_key: String::new(),
        }
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.prev_tx_id);
        out.extend_from_slice(&self.output_index.to_be_bytes());
        out.extend_from_slice(self.signature.as_bytes());
        out.extend_from_slice(self.public_key.as_bytes());
        out
    }
}

impl std::fmt::Debug for TxInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TxInput")
            .field("prev_tx_id", &digest_to_hex_string(&self.prev_tx_id))
            .field("output_index", &self.output_index)
            .field("signature", &self.signature)
            .field("public_key", &self.public_key)
            .finish()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct TxOutput {
    pub value: f64,
    pub address: String, // endereço destino (ex: Base58Check)
}

impl TxOutput {
    pub fn as_bytes(o: &TxOutput) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&o.value.to_be_bytes());
        out.extend_from_slice(o.address.as_bytes());
        out
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UTXO {
    pub tx_id: [u8; 32],
    pub index: usize,
    pub output: TxOutput,
}
