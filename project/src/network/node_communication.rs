use crate::model::Block;
use crate::network::NetworkMessage;
use crate::network::network_message::InventoryType;
use crate::network::server::BROADCAST_CHANNEL;

pub fn broadcast_new_block_hash(block_hash: [u8; 32]) {
    let inv_msg = NetworkMessage::Inv {
        items: vec![(InventoryType::Block, block_hash)],
    };
    let _ = BROADCAST_CHANNEL.sender.send(inv_msg);
}

pub fn send_block(block: &Block) {
    let block_msg = NetworkMessage::Block(block.clone());
    let _ = BROADCAST_CHANNEL.sender.send(block_msg);
}

pub fn ask_for_block(block_hash: [u8; 32]) {
    let get_data_msg = NetworkMessage::GetData {
        item_type: InventoryType::Block,
        item_id: block_hash,
    };
    let _ = BROADCAST_CHANNEL.sender.send(get_data_msg);
}
