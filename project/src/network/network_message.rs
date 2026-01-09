use crate::model::{Block, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InventoryType {
    Block,
    Tx,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetworkMessage {
    // --- Handshake ---
    Version {
        version: u32,
        height: u64,
        top_hash: String,
    },
    VerAck,

    // --- Keep-Alive ---
    Ping(u64),
    Pong(u64),

    Inv {
        items: Vec<(InventoryType, String)>,
    },
    GetData {
        item_type: InventoryType,
        item_id: String,
    },

    Block(Block),
    Tx(Transaction),

    GetBlocks {
        last_known_hash: String,
    },
}
