use crate::model::get_node;
use crate::network::NetworkMessage;
use serde_json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

pub async fn run_p2p_server(port: u16, peers: Vec<String>) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind P2P server to address");

    println!("P2P Server listening on {}", addr);

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
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);

    {
        let node = get_node().await;
        let v = node.get_node_version_info();

        let json = serde_json::to_string(&NetworkMessage::Version(v))?;
        writer.write_all(format!("{}\n", json).as_bytes()).await?;
    }

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let message: NetworkMessage = match serde_json::from_str(line.trim()) {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error deserializing message: {}", e);
                line.clear();
                continue;
            }
        };

        match message {
            NetworkMessage::Version(ver) => {
                println!(
                    "Received VERSION: v={} height={} hash={}",
                    ver.version, ver.height, ver.top_hash
                );

                let ack = serde_json::to_string(&NetworkMessage::VerAck)?;
                writer.write_all(format!("{}\n", ack).as_bytes()).await?;
            }

            NetworkMessage::VerAck => {
                println!("Received VERACK. Handshake complete! Ready to synchronize.");
            }

            NetworkMessage::Ping(nonce) => {
                println!("Received Ping: {}", nonce);
                let pong = serde_json::to_string(&NetworkMessage::Pong(nonce))?;
                writer.write_all(format!("{}\n", pong).as_bytes()).await?;
            }

            _ => println!("Message not yet implemented: {:?}", message),
        }

        line.clear();
    }
    Ok(())
}
