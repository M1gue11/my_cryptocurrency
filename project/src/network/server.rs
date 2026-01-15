use crate::model::{get_node, get_node_mut};
use crate::network::NetworkMessage;
use crate::security_utils::bytes_to_hex_string;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, broadcast};

type BroadcastMessage = (NetworkMessage, Option<SocketAddr>);

pub struct Broadcast {
    pub sender: broadcast::Sender<BroadcastMessage>,
    pub _receiver: broadcast::Receiver<BroadcastMessage>,
}

pub static BROADCAST_CHANNEL: Lazy<Broadcast> = Lazy::new(|| {
    let (sender, receiver) = broadcast::channel(100); // 100 messages buffer
    Broadcast {
        sender,
        _receiver: receiver,
    }
});

pub static CONNECTED_PEERS: Lazy<Arc<RwLock<HashSet<SocketAddr>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashSet::new())));

pub async fn get_connected_peers() -> Vec<SocketAddr> {
    CONNECTED_PEERS.read().await.iter().copied().collect()
}

pub async fn get_peer_count() -> usize {
    CONNECTED_PEERS.read().await.len()
}

pub async fn run_server(port: u16, peers: Vec<String>) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind P2P server to address");

    println!("P2P Server listening on {}", addr);
    println!("Known peers: {:?}", peers);

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
                println!("New connection received from: {}", addr);
                // Spawn a task to handle this connection without blocking the rest
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(socket).await {
                        println!("Connection lost with {}: {}", addr, e);
                    }
                });
            }
            Err(e) => println!("Connection error: {}", e),
        }
    }
}

async fn connect_to_peer(address: String) {
    println!("Trying to connect to peer: {}", address);
    match TcpStream::connect(&address).await {
        Ok(stream) => {
            println!("Connected to {}", address);
            // Initiate handshake actively
            if let Err(e) = handle_connection(stream).await {
                println!("Connection lost with {}: {}", address, e);
            }
        }
        Err(e) => println!("Failed to connect to {}: {}", address, e),
    }
}

async fn handle_connection(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let peer_addr = stream.peer_addr().ok();

    // Add peer to connected peers list
    if let Some(addr) = peer_addr {
        CONNECTED_PEERS.write().await.insert(addr);
        println!(
            "Peer connected: {}. Total peers: {}",
            addr,
            get_peer_count().await
        );
    }

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
            // Received data from the network (from another Peer)
            read_result = reader.read_line(&mut line) => {
                match read_result {
                    Ok(0) => break,
                    Ok(_) => {
                        // Process received message (same as before)
                        let message: NetworkMessage = match serde_json::from_str(line.trim()) {
                            Ok(msg) => msg,
                            Err(e) => {
                                println!("JSON Error: {}", e);
                                line.clear();
                                continue;
                            }
                        };

                        match message {
                            NetworkMessage::Version(ver) => {
                                println!(
                                    "Received VERSION: v={} height={} hash={}",
                                    ver.version, ver.height, bytes_to_hex_string(&ver.top_hash)
                                );
                                get_node().await.handle_version_message(ver).await;
                                let ack = serde_json::to_string(&NetworkMessage::VerAck)?;
                                writer.write_all(format!("{}\n", ack).as_bytes()).await?;
                            },

                            NetworkMessage::VerAck => {
                                println!("Received VERACK. Handshake complete! Ready to synchronize.");
                            },

                            NetworkMessage::Inv { items } => {
                                println!("Received Inventory with {} items.", items.len());
                                let node = get_node().await;
                                node.handle_inventory(items, peer_addr).await;
                            },

                            NetworkMessage::GetData{item_type, item_id} => {
                                let node = get_node().await;
                                node.handle_get_data_request(item_type, item_id).await;
                            },

                            NetworkMessage::Block(block) => {
                                let mut node = get_node_mut().await;
                                node.handle_received_block(block, peer_addr).await;
                            },

                            NetworkMessage::Tx(tx) => {
                                let mut node = get_node_mut().await;
                                node.handle_received_transaction(tx, peer_addr).await;
                            },

                            NetworkMessage::GetBlocks { last_known_hash } => {
                                let node = get_node().await;
                                node.handle_get_blocks_request(last_known_hash).await;
                            },

                            _ => println!("Received: {:?}", message),
                        }

                        line.clear();
                    }
                    Err(e) => return Err(Box::new(e)),
                }
            }

            Ok((msg, exclude_peer)) = broadcast_rx.recv() => {
                if let Some(excluded) = exclude_peer {
                    if peer_addr == Some(excluded) {
                        println!("Skipping message to excluded peer: {:?}", excluded);
                        continue;
                    }
                }
                let json = serde_json::to_string(&msg)?;
                writer.write_all(format!("{}\n", json).as_bytes()).await?;
            }
        }
    }

    // Remove peer from connected peers list when disconnecting
    if let Some(addr) = peer_addr {
        CONNECTED_PEERS.write().await.remove(&addr);
        println!(
            "Peer disconnected: {}. Total peers: {}",
            addr,
            get_peer_count().await
        );
    }

    Ok(())
}
