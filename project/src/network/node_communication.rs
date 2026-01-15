use crate::model::{Block, Transaction};
use crate::network::NetworkMessage;
use crate::network::network_message::InventoryType;
use crate::network::server::{BROADCAST_CHANNEL, Delivery};
use std::net::SocketAddr;

pub fn broadcast_new_block_hash(block_hash: [u8; 32], exclude_peer: Option<SocketAddr>) {
    let inv_msg = NetworkMessage::Inv {
        items: vec![(InventoryType::Block, block_hash)],
    };
    let _ = BROADCAST_CHANNEL
        .sender
        .send((inv_msg, Delivery::Broadcast { exclude_peer }));
}

pub fn broadcast_new_tx_hash(tx_hash: [u8; 32], exclude_peer: Option<SocketAddr>) {
    let inv_msg = NetworkMessage::Inv {
        items: vec![(InventoryType::Tx, tx_hash)],
    };
    let _ = BROADCAST_CHANNEL
        .sender
        .send((inv_msg, Delivery::Broadcast { exclude_peer }));
}

pub fn send_block(block: &Block, exclude_peer: Option<SocketAddr>) {
    let block_msg = NetworkMessage::Block(block.clone());
    let _ = BROADCAST_CHANNEL
        .sender
        .send((block_msg, Delivery::Broadcast { exclude_peer }));
}

pub fn send_block_to(block: &Block, target_peer: SocketAddr) {
    let block_msg = NetworkMessage::Block(block.clone());
    let _ = BROADCAST_CHANNEL
        .sender
        .send((block_msg, Delivery::Direct { target_peer }));
}

pub fn send_tx_to(tx: &Transaction, target_peer: SocketAddr) {
    let tx_msg = NetworkMessage::Tx(tx.clone());
    let _ = BROADCAST_CHANNEL
        .sender
        .send((tx_msg, Delivery::Direct { target_peer }));
}

pub fn ask_for_block(block_hash: [u8; 32]) {
    let get_data_msg = NetworkMessage::GetData {
        item_type: InventoryType::Block,
        item_id: block_hash,
    };
    let _ = BROADCAST_CHANNEL
        .sender
        .send((get_data_msg, Delivery::Broadcast { exclude_peer: None }));
}

pub fn ask_for_tx(tx_hash: [u8; 32]) {
    let get_data_msg = NetworkMessage::GetData {
        item_type: InventoryType::Tx,
        item_id: tx_hash,
    };
    let _ = BROADCAST_CHANNEL
        .sender
        .send((get_data_msg, Delivery::Broadcast { exclude_peer: None }));
}

pub fn ask_for_blocks(last_known_hash: [u8; 32]) {
    let get_blocks_msg = NetworkMessage::GetBlocks { last_known_hash };
    let _ = BROADCAST_CHANNEL
        .sender
        .send((get_blocks_msg, Delivery::Broadcast { exclude_peer: None }));
}
