#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::Duration};

    use chrono::NaiveDate;
    use primitive_types::U256;
    use tokio::time::timeout;

    use crate::{
        db::db::init_db,
        model::{Block, block::BlockHeader, node::Node},
        network::{
            NetworkMessage,
            server::{BROADCAST_CHANNEL, Delivery},
        },
    };

    fn test_block(prev_block_hash: [u8; 32], nonce: u32) -> Block {
        let timestamp = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, nonce)
            .unwrap();

        Block {
            header: BlockHeader {
                prev_block_hash,
                merkle_root: [nonce as u8; 32],
                nonce,
                timestamp,
                target: U256::MAX,
            },
            transactions: Vec::new(),
        }
    }

    fn build_test_node(chain: Vec<Block>) -> Node {
        init_db();
        let mut node = Node::new();
        node.blockchain.chain = chain;
        node
    }

    async fn expect_direct_get_blocks(
        receiver: &mut tokio::sync::broadcast::Receiver<(NetworkMessage, Delivery)>,
        expected_hash: [u8; 32],
        expected_peer: SocketAddr,
    ) {
        timeout(Duration::from_secs(1), async {
            loop {
                let (message, delivery) = receiver.recv().await.unwrap();
                if matches!(
                    (&message, &delivery),
                    (
                        NetworkMessage::GetBlocks { last_known_hash },
                        Delivery::Direct { target_peer }
                    ) if *last_known_hash == expected_hash && *target_peer == expected_peer
                ) {
                    break;
                }
            }
        })
        .await
        .expect("expected a direct GetBlocks request");
    }

    #[tokio::test]
    async fn continues_sync_when_common_block_matches_local_tip() {
        let mut receiver = BROADCAST_CHANNEL.sender.subscribe();
        let peer: SocketAddr = "127.0.0.1:6100".parse().unwrap();

        let genesis = test_block([0; 32], 1);
        let second = test_block(genesis.id(), 2);

        let mut node = build_test_node(vec![genesis, second.clone()]);

        node.handle_received_common_block(second.clone(), Some(peer))
            .await;

        expect_direct_get_blocks(&mut receiver, second.id(), peer).await;
    }

    #[tokio::test]
    async fn keeps_fork_flow_when_common_block_is_not_local_tip() {
        let mut receiver = BROADCAST_CHANNEL.sender.subscribe();
        let peer: SocketAddr = "127.0.0.1:6101".parse().unwrap();

        let genesis = test_block([0; 32], 3);
        let second = test_block(genesis.id(), 4);
        let third = test_block(second.id(), 5);

        let mut node = build_test_node(vec![genesis, second.clone(), third]);

        node.handle_received_common_block(second.clone(), Some(peer))
            .await;

        expect_direct_get_blocks(&mut receiver, second.id(), peer).await;
    }
}
