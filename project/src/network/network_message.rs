use crate::model::{Block, Transaction, node::NodeVersion};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum InventoryType {
    Block,
    Tx,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetworkMessage {
    // --- Handshake ---
    Version(NodeVersion),
    VerAck,

    // --- Keep-Alive ---
    Ping(u64),
    Pong(u64),

    Inv {
        items: Vec<(InventoryType, [u8; 32])>,
    },
    GetData {
        item_type: InventoryType,
        item_id: [u8; 32],
    },

    Block(Block),
    Tx(Transaction),

    GetBlocks {
        last_known_hash: [u8; 32],
    },
}
