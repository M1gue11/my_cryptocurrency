use crate::cli::client::RpcClient;
use crate::CliCommand;

pub async fn execute_command(cmd: CliCommand) -> Result<(), String> {
    let client = RpcClient::new();

    // Check if daemon is running
    if client.ping().await.is_err() {
        return Err(
            "Daemon is not running. Start it with: caramuru daemon start".to_string(),
        );
    }

    match cmd {
        CliCommand::Node(node_cmd) => handle_node_command(&client, node_cmd).await,
        CliCommand::Mine(mine_cmd) => handle_mine_command(&client, mine_cmd).await,
        CliCommand::Chain(chain_cmd) => handle_chain_command(&client, chain_cmd).await,
        CliCommand::Wallet(wallet_cmd) => handle_wallet_command(&client, wallet_cmd).await,
    }
}

async fn handle_node_command(
    client: &RpcClient,
    cmd: crate::NodeSubcommands,
) -> Result<(), String> {
    match cmd {
        crate::NodeSubcommands::Status => {
            let status: serde_json::Value = client.call("node.status", serde_json::json!({})).await?;

            println!("\n=== Node Status ===");
            println!(
                "  Version: {}",
                status["version"]["version"].as_u64().unwrap_or(0)
            );
            println!(
                "  Peers Connected: {}",
                status["peers_connected"].as_u64().unwrap_or(0)
            );
            println!(
                "  Current Block Height: {}",
                status["version"]["height"].as_u64().unwrap_or(0)
            );
            if let Some(hash_array) = status["version"]["top_hash"].as_array() {
                let hash_bytes: Vec<u8> = hash_array
                    .iter()
                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                    .collect();
                println!("  Current Block Hash: {}", hex::encode(hash_bytes));
            }
            println!();
            Ok(())
        }
    }
}

async fn handle_mine_command(
    client: &RpcClient,
    cmd: crate::MineSubcommands,
) -> Result<(), String> {
    match cmd {
        crate::MineSubcommands::Block => {
            println!("Mining new block...");
            let result: serde_json::Value =
                client.call("mine.block", serde_json::json!({})).await?;

            println!("Block mined successfully!");
            println!(
                "  Block hash: {}",
                result["hash"].as_str().unwrap_or("N/A")
            );
            println!(
                "  Transactions: {}",
                result["transactions"].as_u64().unwrap_or(0)
            );
            println!("  Nonce: {}", result["nonce"].as_u64().unwrap_or(0));
            println!(
                "  Timestamp: {}",
                result["timestamp"].as_str().unwrap_or("N/A")
            );
            println!();
            Ok(())
        }
    }
}

async fn handle_chain_command(
    client: &RpcClient,
    cmd: crate::ChainSubcommands,
) -> Result<(), String> {
    match cmd {
        crate::ChainSubcommands::Status => {
            let status: serde_json::Value =
                client.call("chain.status", serde_json::json!({})).await?;

            println!("\n=== Blockchain Status ===");
            println!("  Blocks: {}", status["blocks"].as_u64().unwrap_or(0));
            println!(
                "  Valid: {}",
                if status["valid"].as_bool().unwrap_or(false) {
                    "Yes"
                } else {
                    "No"
                }
            );

            if let Some(hash) = status["last_block_hash"].as_str() {
                println!("  Last Block Hash: {}", hash);
            }
            if let Some(date) = status["last_block_date"].as_str() {
                println!("  Last Block Date: {}", date);
            }
            println!();
            Ok(())
        }
    }
}

async fn handle_wallet_command(
    client: &RpcClient,
    cmd: crate::WalletSubcommands,
) -> Result<(), String> {
    match cmd {
        crate::WalletSubcommands::Balance { name } => {
            let params = if let Some(n) = name {
                serde_json::json!({ "name": n })
            } else {
                serde_json::json!({})
            };

            let balance: serde_json::Value = client.call("wallet.balance", params).await?;

            println!("\n=== Wallet Balance ===");
            println!("  UTXOs: {}", balance["utxos"].as_u64().unwrap_or(0));
            println!(
                "  Total Balance: {} coins",
                balance["total_balance"].as_i64().unwrap_or(0)
            );
            println!();
            Ok(())
        }
    }
}
