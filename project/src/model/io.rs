use serde::{Deserialize, Serialize};

use crate::security_utils::digest_to_hex_string;

#[derive(Clone, Serialize, Deserialize)]
pub struct TxInput {
    pub prev_tx_id: [u8; 32], // hash da transação anterior
    pub output_index: usize,  // índice do output gasto
    pub signature: String,    // assinatura do dono
    pub public_key: String,   // chave pública do dono
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TxOutput {
    pub value: f64,
    pub address: String, // endereço destino (ex: Base58Check)
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UTXO {
    pub tx_id: [u8; 32],
    pub index: usize,
    pub output: TxOutput,
}
