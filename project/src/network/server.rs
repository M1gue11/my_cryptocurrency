use crate::model::{get_node, get_node_mut};
use crate::network::NetworkMessage;
use crate::network::peer_manager::{
    PEER_MANAGER, PeerDirection, PeerHandshakeState, get_peer_count,
};
use crate::security_utils::bytes_to_hex_string;
use crate::utils;
use once_cell::sync::Lazy;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

#[derive(Clone)]
pub enum Delivery {
    Broadcast { exclude_peer: Option<SocketAddr> },
    Direct { target_peer: SocketAddr },
}

type QueuedMessage = (NetworkMessage, Delivery);

pub struct Broadcast {
    pub sender: broadcast::Sender<QueuedMessage>,
    pub _receiver: broadcast::Receiver<QueuedMessage>,
}

pub static BROADCAST_CHANNEL: Lazy<Broadcast> = Lazy::new(|| {
    let (sender, receiver) = broadcast::channel(100); // 100 messages buffer
    Broadcast {
        sender,
        _receiver: receiver,
    }
});

pub async fn run_server(port: u16, peers: Vec<String>) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind P2P server to address");

    utils::log_info(
        utils::LogCategory::P2P,
        &format!("P2P Server listening on {}", addr),
    );
    utils::log_info(
        utils::LogCategory::P2P,
        &format!("Known peers: {:?}", peers),
    );

    // 1. Try to connect to known peers (seeds)
    for peer_addr in peers {
        let peer_addr_clone = peer_addr.clone();
        tokio::spawn(async move {
            connect_to_peer(peer_addr_clone).await;
        });
    }

    // 2. Loop to accept new connections
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                utils::log_info(
                    utils::LogCategory::P2P,
                    &format!("New connection received from: {}", addr),
                );
                // Spawn a task to handle this connection without blocking the rest
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket, PeerDirection::Inbound).await {
                        utils::log_warning(
                            utils::LogCategory::P2P,
                            &format!("Connection lost with {}: {}", addr, e),
                        );
                    }
                });
            }
            Err(e) => {
                utils::log_error(utils::LogCategory::P2P, &format!("Connection error: {}", e))
            }
        }
    }
}

async fn connect_to_peer(address: String) {
    utils::log_info(
        utils::LogCategory::P2P,
        &format!("Trying to connect to peer: {}", address),
    );
    match TcpStream::connect(&address).await {
        Ok(stream) => {
            utils::log_info(
                utils::LogCategory::P2P,
                &format!("Connected to {}", address),
            );
            // Initiate handshake actively
            if let Err(e) = handle_connection(stream, PeerDirection::Outbound).await {
                utils::log_warning(
                    utils::LogCategory::P2P,
                    &format!("Connection lost with {}: {}", address, e),
                );
            }
        }
        Err(e) => utils::log_warning(
            utils::LogCategory::P2P,
            &format!("Failed to connect to {}: {}", address, e),
        ),
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    direction: PeerDirection,
) -> Result<(), Box<dyn std::error::Error>> {
    let peer_addr = stream.peer_addr().ok();

    let (connection_id, mut disconnect_rx) = match peer_addr {
        Some(addr) => {
            let registration = PEER_MANAGER.register_peer(addr, direction).await;
            utils::log_info(
                utils::LogCategory::P2P,
                &format!(
                    "Peer connected: {}. Total peers: {}",
                    addr,
                    get_peer_count().await
                ),
            );
            registration
        }
        None => return Err("Missing peer address".into()),
    };

    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);
    let mut broadcast_rx = BROADCAST_CHANNEL.sender.subscribe();

    {
        let node = get_node().await;
        let v = node.get_node_version_info();

        let json = serde_json::to_string(&NetworkMessage::Version(v))?;
        writer.write_all(format!("{}\n", json).as_bytes()).await?;
    }

    let mut line = String::new();
    loop {
        tokio::select! {
            disconnect_signal = disconnect_rx.changed() => {
                match disconnect_signal {
                    Ok(_) if *disconnect_rx.borrow() => {
                        PEER_MANAGER
                            .mark_disconnecting(
                                peer_addr.unwrap(),
                                connection_id,
                                "Disconnect signal received",
                            )
                            .await;
                        break;
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            // Received data from the network (from another Peer)
            read_result = reader.read_line(&mut line) => {
                match read_result {
                    Ok(0) => break,
                    Ok(_) => {
                        // Process received message (same as before)
                        let message: NetworkMessage = match serde_json::from_str(line.trim()) {
                            Ok(msg) => msg,
                            Err(e) => {
                                utils::log_error(utils::LogCategory::P2P, &format!("JSON Error: {}", e));
                                line.clear();
                                continue;
                            }
                        };

                        match message {
                            NetworkMessage::Version(ver) => {
                                PEER_MANAGER
                                    .set_handshake_state(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        PeerHandshakeState::VersionReceived,
                                        "Received VERSION",
                                    )
                                    .await;
                                utils::log_info(utils::LogCategory::P2P, &format!(
                                    "Received VERSION: v={} height={} hash={}",
                                    ver.version, ver.height, bytes_to_hex_string(&ver.top_hash)
                                ));
                                get_node().await.handle_version_message(ver, peer_addr).await;
                                let ack = serde_json::to_string(&NetworkMessage::VerAck)?;
                                writer.write_all(format!("{}\n", ack).as_bytes()).await?;
                            },

                            NetworkMessage::VerAck => {
                                PEER_MANAGER
                                    .set_handshake_state(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        PeerHandshakeState::HandshakeComplete,
                                        "Received VERACK",
                                    )
                                    .await;
                                utils::log_info(utils::LogCategory::P2P, "Received VERACK. Handshake complete! Ready to synchronize.");
                            },

                            NetworkMessage::Inv { items } => {
                                PEER_MANAGER
                                    .update_last_event(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        format!("Received INV ({} items)", items.len()),
                                    )
                                    .await;
                                utils::log_info(utils::LogCategory::P2P, &format!("Received Inventory with {} items.", items.len()));
                                let node = get_node().await;
                                node.handle_inventory(items, peer_addr).await;
                            },

                            NetworkMessage::GetData{item_type, item_id} => {
                                PEER_MANAGER
                                    .update_last_event(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        format!("Received GETDATA {:?}", item_type),
                                    )
                                    .await;
                                let node = get_node().await;
                                node.handle_get_data_request(item_type, item_id, peer_addr).await;
                            },

                            NetworkMessage::Block(block) => {
                                PEER_MANAGER
                                    .update_last_event(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        format!("Received BLOCK {}", bytes_to_hex_string(&block.id())),
                                    )
                                    .await;
                                let mut node = get_node_mut().await;
                                node.handle_received_block(block, peer_addr).await;
                            },

                            NetworkMessage::Tx(tx) => {
                                PEER_MANAGER
                                    .update_last_event(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        format!("Received TX {}", bytes_to_hex_string(&tx.id())),
                                    )
                                    .await;
                                let mut node = get_node_mut().await;
                                node.handle_received_transaction(tx, peer_addr).await;
                            },

                            NetworkMessage::GetBlocks { last_known_hash } => {
                                PEER_MANAGER
                                    .update_last_event(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        format!(
                                            "Received GETBLOCKS from {}",
                                            bytes_to_hex_string(&last_known_hash)
                                        ),
                                    )
                                    .await;
                                let node = get_node().await;
                                node.handle_get_blocks_request(last_known_hash, peer_addr.unwrap()).await;
                            },

                            NetworkMessage::FindCommonAncestor { local_block_hashes } => {
                                PEER_MANAGER
                                    .update_last_event(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        format!(
                                            "Received FIND_COMMON_ANCESTOR ({} hashes)",
                                            local_block_hashes.len()
                                        ),
                                    )
                                    .await;
                                let node = get_node().await;
                                node.handle_find_common_ancestor_request(local_block_hashes, peer_addr.unwrap()).await;
                            },

                            NetworkMessage::SendCommonBlock(block) => {
                                PEER_MANAGER
                                    .update_last_event(
                                        peer_addr.unwrap(),
                                        connection_id,
                                        format!(
                                            "Received COMMON_BLOCK {}",
                                            bytes_to_hex_string(&block.id())
                                        ),
                                    )
                                    .await;
                                let mut node = get_node_mut().await;
                                node.handle_received_common_block(block, peer_addr).await;
                            },

                            _ => utils::log_info(utils::LogCategory::P2P, &format!("Received: {:?}", message)),
                        }

                        line.clear();
                    }
                    Err(e) => return Err(Box::new(e)),
                }
            }

            Ok((msg, delivery)) = broadcast_rx.recv() => {
                match delivery {
                    Delivery::Broadcast { exclude_peer } => {
                        if exclude_peer.is_some() && peer_addr == exclude_peer {
                            utils::log_info(utils::LogCategory::P2P, &format!("Skipping message to excluded peer: {:?}", exclude_peer.unwrap()));
                            continue;
                        }
                        let json = serde_json::to_string(&msg)?;
                        writer.write_all(format!("{}\n", json).as_bytes()).await?;
                    }
                    Delivery::Direct { target_peer } => {
                        if peer_addr != Some(target_peer) {
                            continue;
                        }
                        let json = serde_json::to_string(&msg)?;
                        writer.write_all(format!("{}\n", json).as_bytes()).await?;
                    }
                }
            }
        }
    }

    // Remove peer from connected peers list when disconnecting
    let addr = peer_addr.unwrap();
    PEER_MANAGER.remove_peer(addr, connection_id).await;
    utils::log_info(
        utils::LogCategory::P2P,
        &format!(
            "Peer disconnected: {}. Total peers: {}",
            addr,
            get_peer_count().await
        ),
    );

    Ok(())
}
