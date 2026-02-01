use super::cli::{ChainCommands, Commands, MineCommands, TransactionCommands, WalletCommands};
use crate::{
    cli::{RpcClient, cli::NodeCommands},
    daemon::types::WalletAccessParams,
    globals::CONFIG,
};
use std::io::{self, Write};

pub async fn run_cli(client: RpcClient) {
    print_welcome(&client).await;

    let mut loaded_wallets: Vec<(String, WalletAccessParams)> = {
        vec![(
            "miner_wallet".to_string(),
            WalletAccessParams {
                key_path: CONFIG.miner_wallet_seed_path.clone(),
                password: CONFIG.miner_wallet_password.clone(),
            },
        )]
    };

    loop {
        print!("\n> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();

                if input.is_empty() {
                    continue;
                }

                if input == "exit" || input == "quit" || input == "q" {
                    println!("Goodbye!");
                    break;
                }

                if input == "help" || input == "?" {
                    print_help();
                    continue;
                }

                match parse_command(input) {
                    Ok(command) => {
                        if let Err(e) = execute_command(command, &client, &mut loaded_wallets).await
                        {
                            println!("Error: {}", e);
                        }
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                        println!("  Type 'help' for available commands");
                    }
                }
            }
            Err(e) => {
                println!("Error reading input: {}", e);
            }
        }
    }
}

fn resolve_wallet_by_name<'a>(
    name: Option<String>,
    loaded_wallets: &'a mut Vec<(String, WalletAccessParams)>,
) -> &'a mut WalletAccessParams {
    let name = match name {
        Some(n) => n,
        None => "miner_wallet".to_string(),
    };
    for (loaded_name, wallet) in loaded_wallets {
        if *loaded_name == name {
            return wallet;
        }
    }
    panic!("Wallet with name '{}' not found", name);
}

async fn startup_helper(client: &RpcClient) {
    let response = match client.node_status().await {
        Ok(status) => status,
        Err(e) => {
            println!("Error: Could not retrieve node status: {}", e);
            return;
        }
    };

    if response.block_height == 0 {
        println!("⚠  Blockchain is empty. Use 'mine block' to create the genesis block.");
    } else {
        println!("✓  Loaded blockchain with {} blocks", response.block_height);
    }
}

async fn print_welcome(client: &RpcClient) {
    println!("\n╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║              🔗 CARAMURU Node Interactive CLI 🔗                     ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!("\nWelcome! Type 'help' for available commands or 'exit' to quit.\n");

    startup_helper(client).await;
}

fn print_help() {
    println!("\n╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                        Available Commands                            ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!("\n📋 General:");
    println!("  help, ?                    - Show this help message");
    println!("  exit, quit, q              - Exit the program");

    println!("\n★  Node:");
    println!("  node init                       - Reinitialize the node");
    println!("  node mempool                    - Display transactions in the mempool");
    println!("  node clear-mempool              - Clear all transactions in the mempool");
    println!("  node status                     - Show current node status");

    println!("\n⛏  Mining:");
    println!("  mine block                 - Mine a new block with pending transactions");

    println!("\n🔗 Blockchain:");
    println!("  chain show                 - Display the entire blockchain");
    println!("  chain status               - Show blockchain status");
    println!("  chain validate             - Validate blockchain integrity");
    println!("  chain utxos [--limit <n>]  - Show at most <n> UTXOs");
    println!("    - Limit is optional, default is 10");

    println!("\n💰 Wallet:");
    println!("  wallet new --password <password> --path <keystore_path> [--name <name>]");
    println!("    - Create a new wallet. If name is provided, wallet is stored in session.");

    println!("\n  wallet list");
    println!("    - List all loaded wallets in the current session.");

    println!("\n  wallet address [--name <name>]");
    println!("    - Get new receive address (miner's wallet by default)");

    println!("\n  wallet balance [--name <name>]");
    println!("    - Check wallet balance");
    println!("    - Defaults to miner's wallet if no name is provided");

    println!(
        "\n  wallet send [--from <name>] --to <addr> --amount <n> [--fee <fee>] [--message <msg>]"
    );
    println!("    - Send transaction (miner's wallet will send by default)");

    println!("\n  wallet generate-keys [--count <n>] [--name <name>] [--type <0|1>]");
    println!("    - Generate n keys (default: 5). ");
    println!("    - If name is provided, uses that wallet. ");
    println!("    - Type 0 = receive, 1 = change. ");

    println!("\n📄 Transaction:");
    println!("  transaction view --id <hex_id>     - View transaction details");
    println!();
}

fn parse_command(input: &str) -> Result<Commands, String> {
    let parts: Vec<&str> = input.split_whitespace().collect();

    if parts.is_empty() {
        return Err("Empty command".to_string());
    }

    match parts[0] {
        "node" => {
            if parts.len() < 2 {
                return Err("Usage: node <init|mempool|clear-mempool>".to_string());
            }
            match parts[1] {
                "init" => Ok(Commands::Node(NodeCommands::Init)),
                "mempool" => Ok(Commands::Node(NodeCommands::Mempool)),
                "clear-mempool" => Ok(Commands::Node(NodeCommands::ClearMempool)),
                "status" => Ok(Commands::Node(NodeCommands::Status)),
                _ => Err(format!("Unknown node command: {}", parts[1])),
            }
        }

        "mine" => {
            if parts.len() < 2 {
                return Err("Usage: mine block".to_string());
            }
            match parts[1] {
                "block" => Ok(Commands::Mine(MineCommands::Block)),
                _ => Err(format!("Unknown mine command: {}", parts[1])),
            }
        }

        "chain" => {
            if parts.len() < 2 {
                return Err("Usage: chain <show|status|validate|rollback>".to_string());
            }
            match parts[1] {
                "show" => Ok(Commands::Chain(ChainCommands::Show)),
                "status" => Ok(Commands::Chain(ChainCommands::Status)),
                "validate" => Ok(Commands::Chain(ChainCommands::Validate)),
                "utxos" => {
                    let limit = if let Ok(limit_str) = parse_flag_value(&parts, "--limit") {
                        limit_str.parse::<u32>().map_err(|_| {
                            "Invalid limit format. Must be a positive number".to_string()
                        })?
                    } else {
                        20
                    };

                    Ok(Commands::Chain(ChainCommands::Utxos { limit }))
                }
                _ => Err(format!("Unknown chain command: {}", parts[1])),
            }
        }

        "wallet" => {
            if parts.len() < 2 {
                return Err("Usage: wallet <new|address|balance|send|generate-keys>".to_string());
            }

            match parts[1] {
                "new" => {
                    let password = parse_flag_value(&parts, "--password")?;
                    if password.is_empty() {
                        return Err("Password cannot be empty".to_string());
                    }

                    let path = parse_flag_value(&parts, "--path")?;
                    if path.is_empty() {
                        return Err("Path cannot be empty".to_string());
                    }

                    let name_result = parse_flag_value(&parts, "--name");
                    let name = match name_result {
                        Ok(n) if !n.is_empty() => Some(n),
                        _ => None,
                    };
                    Ok(Commands::Wallet(WalletCommands::New {
                        name,
                        path,
                        password,
                    }))
                }

                "list" => Ok(Commands::Wallet(WalletCommands::List)),

                "address" => {
                    let name_result = parse_flag_value(&parts, "--name");
                    let name = match name_result {
                        Ok(n) if !n.is_empty() => Some(n),
                        _ => None,
                    };
                    Ok(Commands::Wallet(WalletCommands::Address { name }))
                }

                "balance" => {
                    let wallet_name = match parse_flag_value(&parts, "--name") {
                        Ok(n) if !n.is_empty() => Some(n),
                        _ => None,
                    };
                    Ok(Commands::Wallet(WalletCommands::Balance {
                        name: wallet_name,
                    }))
                }

                "send" => {
                    let to = parse_flag_value(&parts, "--to")?;
                    let amount_str = parse_flag_value(&parts, "--amount")?;
                    let from = match parse_flag_value(&parts, "--from") {
                        Ok(n) if !n.is_empty() => Some(n),
                        _ => None,
                    };

                    if to.is_empty() {
                        return Err("Recipient address cannot be empty".to_string());
                    }

                    let amount = amount_str
                        .parse::<i64>()
                        .map_err(|_| "Invalid amount format. Must be a number".to_string())?;

                    if amount <= 0 {
                        return Err("Amount must be greater than zero".to_string());
                    }

                    let fee = match parse_flag_value(&parts, "--fee") {
                        Ok(fee_str) => {
                            let f = fee_str
                                .parse::<i64>()
                                .map_err(|_| "Invalid fee format. Must be a number".to_string())?;
                            if f < 0 {
                                return Err("Fee cannot be negative".to_string());
                            }
                            Some(f)
                        }
                        Err(_) => None,
                    };
                    let message = parse_flag_value(&parts, "--message").ok();
                    Ok(Commands::Wallet(WalletCommands::Send {
                        from,
                        to,
                        amount,
                        fee,
                        message,
                    }))
                }

                "generate-keys" => {
                    let count = if let Ok(count_str) = parse_flag_value(&parts, "--count") {
                        count_str.parse::<u32>().map_err(|_| {
                            "Invalid count format. Must be a positive number".to_string()
                        })?
                    } else {
                        5
                    };

                    // wallet name (optional)
                    let name = match parse_flag_value(&parts, "--name") {
                        Ok(n) if !n.is_empty() => Some(n),
                        _ => None,
                    };

                    if count == 0 {
                        return Err("Count must be greater than zero".to_string());
                    }

                    if count > 100 {
                        return Err("Count cannot exceed 100".to_string());
                    }

                    let type_ = if let Ok(type_str) = parse_flag_value(&parts, "--type") {
                        let t = type_str.parse::<u32>().map_err(|_| {
                            "Invalid type format. Must be 0 (receive) or 1 (change)".to_string()
                        })?;
                        if t > 1 {
                            return Err("Type must be 0 (receive) or 1 (change)".to_string());
                        }
                        Some(t)
                    } else {
                        None
                    };

                    Ok(Commands::Wallet(WalletCommands::GenerateKeys {
                        count,
                        name,
                        type_,
                    }))
                }

                _ => Err(format!("Unknown wallet command: {}", parts[1])),
            }
        }

        "transaction" | "tx" => {
            if parts.len() < 2 {
                return Err("Usage: transaction view --id <hex_id>".to_string());
            }

            match parts[1] {
                "view" => {
                    let id = parse_flag_value(&parts, "--id")?;
                    if id.len() != 64 {
                        return Err("Transaction ID must be 64 hexadecimal characters".to_string());
                    }
                    if !id.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Err(
                            "Transaction ID must contain only hexadecimal characters".to_string()
                        );
                    }
                    Ok(Commands::Transaction(TransactionCommands::View { id }))
                }
                _ => Err(format!("Unknown transaction command: {}", parts[1])),
            }
        }

        _ => Err(format!("Unknown command: {}", parts[0])),
    }
}

fn parse_flag_value(parts: &[&str], flag: &str) -> Result<String, String> {
    for i in 0..parts.len() {
        if parts[i] == flag && i + 1 < parts.len() {
            // Collect all parts until the next flag or end
            let mut value = String::new();
            let mut j = i + 1;
            while j < parts.len() && !parts[j].starts_with("--") {
                if !value.is_empty() {
                    value.push(' ');
                }
                value.push_str(parts[j]);
                j += 1;
            }
            return Ok(value);
        }
    }
    Err(format!("Missing required flag: {}", flag))
}

async fn execute_command(
    command: Commands,
    client: &RpcClient,
    loaded_wallets: &mut Vec<(String, WalletAccessParams)>,
) -> Result<(), String> {
    match command {
        Commands::Node(node_cmd) => {
            handle_node(node_cmd, client).await;
            Ok(())
        }
        Commands::Mine(mine_cmd) => {
            handle_mine(mine_cmd, client).await;
            Ok(())
        }
        Commands::Chain(chain_cmd) => {
            handle_chain(chain_cmd, client).await;
            Ok(())
        }
        Commands::Wallet(wallet_cmd) => {
            handle_wallet(wallet_cmd, client, loaded_wallets).await;
            Ok(())
        }
        Commands::Transaction(tx_cmd) => {
            handle_transaction(tx_cmd, client).await;
            Ok(())
        }
    }
}

async fn handle_node(command: NodeCommands, client: &RpcClient) {
    match command {
        NodeCommands::Init => {
            match client.node_init().await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Node reinitialization failed: {}", e);
                    return;
                }
            };
            println!("✓ Node reinitialized successfully");
            startup_helper(client).await;
        }
        NodeCommands::Mempool => {
            let mempool_response = match client.node_mempool().await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not retrieve mempool: {}", e);
                    return;
                }
            };

            if mempool_response.count == 0 {
                println!("⚠  Mempool is empty");
                return;
            }

            for tx in mempool_response.transactions {
                println!("{:?}\n", tx);
            }
        }

        NodeCommands::ClearMempool => {
            match client.node_clear_mempool().await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not clear mempool: {}", e);
                    return;
                }
            };
            match client.node_save().await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not save node after clearing mempool: {}", e);
                    return;
                }
            };
            println!("✓ Mempool cleared");
        }

        NodeCommands::Status => {
            let status_response = match client.node_status().await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not retrieve node status: {}", e);
                    return;
                }
            };
            println!("\n=== Node Status ===");
            println!("  Version: {}", status_response.version);
            println!("  Peers Connected: {}", status_response.peers_connected);
            println!("  Current Block Height: {}", status_response.block_height);
            println!("  Current Block Hash: {}", status_response.top_block_hash);
        }
    }
}

async fn handle_mine(command: MineCommands, client: &RpcClient) {
    match command {
        MineCommands::Block => {
            println!("⛏  Mining new block...");
            let mine_response = match client.mine_block().await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Mining failed: {}", e);
                    return;
                }
            };
            println!("✓ Block mined successfully!");
            println!("  Block hash: {:?}", mine_response.block_hash);
            println!(
                "  Transactions: {}",
                mine_response
                    .transactions
                    .iter()
                    .map(|tx| format!("{:?}\n", tx))
                    .collect::<Vec<_>>()
                    .len()
            );
            println!("  Nonce: {:?}", mine_response.nonce);
        }
    }
}

async fn handle_chain(command: ChainCommands, client: &RpcClient) {
    match command {
        ChainCommands::Show => {
            let chain_show_response = match client.chain_show().await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not retrieve blockchain: {}", e);
                    return;
                }
            };

            if chain_show_response.blocks.len() == 0 {
                println!("⚠  Blockchain is empty");
                return;
            }

            println!("\n=== Blockchain ===\n");
            for (i, block) in chain_show_response.blocks.iter().enumerate() {
                println!("Block #{} Size: {} bytes", i, block.size_bytes);
                println!("  Hash: {}", block.hash);
                println!("  Previous Hash: {}", block.prev_hash);
                println!("  Merkle Root: {}", block.merkle_root);
                println!("  Nonce: {}", block.nonce);
                println!("  Date: {}", block.timestamp);
                println!("  Transactions: {}", block.transactions.len());

                for (j, tx) in block.transactions.iter().enumerate() {
                    println!("\n    Transaction #{} Size: {} bytes", j, tx.size);
                    println!("      ID: {}", tx.id);
                    println!(
                        "      Amount: {}",
                        tx.outputs.iter().map(|o| o.value).sum::<i64>()
                    );
                    if let Some(msg) = &tx.message {
                        println!("      Message: {}", msg);
                    }
                    println!("      Inputs:");
                    for input in &tx.inputs {
                        println!("            Input Prev TX: {}", input.prev_tx_id);
                        println!("            Input Output Index: {}\n", input.output_index);
                    }
                    println!("      Outputs:");
                    for output in &tx.outputs {
                        println!("            Output Value: {} units", output.value);
                        println!("            Output Address: {}\n", output.address);
                    }
                }
                println!();
            }
        }

        ChainCommands::Validate => {
            match client.chain_validate().await {
                Ok(res) => {
                    println!("✓ Blockchain validation request submitted: {:?}", res);
                }
                Err(e) => {
                    println!("✗ Could not validate blockchain: {}", e);
                    return;
                }
            };
        }

        ChainCommands::Status => {
            let status_response = match client.chain_status().await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not retrieve blockchain status: {}", e);
                    return;
                }
            };

            println!("\n=== Blockchain Status ===");
            println!("  Blocks: {}", status_response.block_count);
            println!(
                "  Valid: {}",
                if status_response.is_valid {
                    "Yes"
                } else {
                    "No"
                }
            );

            if status_response.block_count > 0 {
                let last_block_hash = status_response
                    .last_block_hash
                    .unwrap_or("Ultimo hash nao encontrado".to_string());

                let last_block_date = status_response
                    .last_block_date
                    .unwrap_or("Ultima data nao encontrada".to_string());
                println!("  Last Block Hash: {}", last_block_hash);
                println!("  Last Block Date: {}", last_block_date);
            }
            println!();
        }

        ChainCommands::Utxos { limit } => {
            let utxos_resp = match client.chain_utxos(limit).await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not retrieve UTXOs: {}", e);
                    return;
                }
            };
            if utxos_resp.utxos.len() == 0 {
                println!("⚠  No UTXOs found in the blockchain");
                return;
            }

            println!("\n=== UTXOs (showing up to {}) ===\n", limit);

            println!(
                "┌──────┬──────────────────┬───────┬──────────────┬────────────────────────────────────────────────┐"
            );
            println!(
                "│  #   │     TX ID        │ Index │    Value     │                   Address                      │"
            );
            println!(
                "├──────┼──────────────────┼───────┼──────────────┼────────────────────────────────────────────────┤"
            );

            for (i, utxo) in utxos_resp.utxos.iter().take(limit as usize).enumerate() {
                let tx_id_hex = &utxo.tx_id;
                let tx_id_short = format!("{}...{} ", &tx_id_hex[..6], &tx_id_hex[58..]);

                println!(
                    "│ {:>4} │ {} │ {:>5} │ {:>12.2} │ {:46} │",
                    i + 1,
                    tx_id_short,
                    utxo.index,
                    utxo.value,
                    utxo.address
                );
            }

            println!(
                "└──────┴──────────────────┴───────┴──────────────┴────────────────────────────────────────────────┘"
            );

            let total: i64 = utxos_resp
                .utxos
                .iter()
                .take(limit as usize)
                .map(|u| u.value)
                .sum();
            println!(
                "\nTotal: {} units across {} UTXOs",
                total,
                utxos_resp.utxos.len().min(limit as usize)
            );
            println!();
        }
    }
}

async fn handle_wallet(
    command: WalletCommands,
    client: &RpcClient,
    loaded_wallets: &mut Vec<(String, WalletAccessParams)>,
) {
    match command {
        WalletCommands::New {
            password,
            path,
            name,
        } => {
            if name.is_some()
                && loaded_wallets
                    .iter()
                    .any(|(n, _)| n == name.as_ref().unwrap())
            {
                println!(
                    "✗ Wallet with name '{}' already loaded in session",
                    name.unwrap()
                );
                return;
            }

            let new_wallet_response = match client.wallet_new(&password, &path).await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Wallet creation failed: {}", e);
                    return;
                }
            };
            if name.is_some() {
                let name = name.unwrap();
                loaded_wallets.push((
                    name.clone(),
                    WalletAccessParams {
                        key_path: path.clone(),
                        password: password.clone(),
                    },
                ));
            }

            println!("✓ Wallet created successfully");
            println!("  First address: {:?}", new_wallet_response.address);
        }

        WalletCommands::List => {
            println!("\n=== Loaded Wallets ===\n");
            for (name, w) in loaded_wallets.iter() {
                let response = match client
                    .wallet_balance(w.key_path.clone(), w.password.clone())
                    .await
                {
                    Ok(res) => res,
                    Err(e) => {
                        println!("✗ Could not retrieve wallet balance for {}: {}", name, e);
                        continue;
                    }
                };
                println!("Wallet: {} - Balance: {}", name, response.balance);
            }
        }

        WalletCommands::Address { name } => {
            // Select wallet, default to miner's wallet
            let wallet = resolve_wallet_by_name(name, loaded_wallets);
            let address_response = match client
                .wallet_address(wallet.key_path.clone(), wallet.password.clone())
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not retrieve wallet address: {}", e);
                    return;
                }
            };
            println!("✓ New receive address: {}", address_response.address);
        }

        WalletCommands::Balance { name } => {
            let wallet = resolve_wallet_by_name(name, loaded_wallets);
            let balance_response = match client
                .wallet_balance(wallet.key_path.clone(), wallet.password.clone())
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not retrieve wallet balance: {}", e);
                    return;
                }
            };

            let total: i64 = balance_response.utxos.iter().map(|u| u.value).sum();
            println!("\n=== Wallet Balance ===");
            println!("  UTXOs: {}", balance_response.utxos.len());
            println!("  Total Balance: {} coins", total);

            if !balance_response.utxos.is_empty() {
                println!("\n  Details:");
                for (i, utxo) in balance_response.utxos.iter().enumerate() {
                    println!("    UTXO #{}: {} coins to {}", i, utxo.value, utxo.address);
                }
            }
            println!();
        }

        WalletCommands::Send {
            from,
            to,
            amount,
            fee,
            message,
        } => {
            let wallet = resolve_wallet_by_name(from, loaded_wallets);

            let send_response = match client.wallet_send(wallet, &to, amount, fee, message).await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not prepare transaction: {}", e);
                    return;
                }
            };

            if !send_response.success {
                println!(
                    "✗ Transaction failed: {}",
                    send_response.error.unwrap_or("Unknown error".to_string())
                );
                return;
            }

            println!(
                "✓ Transaction {} created and added to mempool",
                send_response.tx_id.unwrap_or("Unknown".to_string())
            );

            println!("\n  Use 'mine block' to include it in the blockchain");
        }

        WalletCommands::GenerateKeys { count, name, type_ } => {
            let wallet = resolve_wallet_by_name(name, loaded_wallets);
            let gen_keys_response = match client
                .wallet_generate_keys(
                    count,
                    wallet.key_path.clone(),
                    wallet.password.clone(),
                    type_,
                )
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not generate keys: {}", e);
                    return;
                }
            };
            println!("✓ Generated {} keys:\n", gen_keys_response.keys.len());
            for (i, key) in gen_keys_response.keys.iter().enumerate() {
                println!("Key #{}", i + 1);
                println!("  Address: {}", key.address);
                println!("  Public Key: {}", key.public_key);
                println!();
            }
        }
    }
}

async fn handle_transaction(command: TransactionCommands, client: &RpcClient) {
    match command {
        TransactionCommands::View { id } => {
            let get_tx_response = match client.transaction_view(&id).await {
                Ok(res) => res,
                Err(e) => {
                    println!("✗ Could not retrieve transaction: {}", e);
                    return;
                }
            };
            println!("\n=== Transaction Details ===");
            println!("  ID: {}", get_tx_response.id);
            println!("  Date: {}", get_tx_response.date);

            if let Some(msg) = &get_tx_response.message {
                println!("  Message: {}", msg);
            }

            println!("\n  Inputs ({}): ", get_tx_response.inputs.len());
            for (i, input) in get_tx_response.inputs.iter().enumerate() {
                println!("    Input #{}", i);
                println!("      Previous TX: {}", input.prev_tx_id);
                println!("      Output Index: {}", input.output_index);
                println!("      Public Key: {}", input.public_key);
            }

            println!("\n  Outputs ({}):", get_tx_response.outputs.len());
            for (i, output) in get_tx_response.outputs.iter().enumerate() {
                println!("    Output #{}", i);
                println!("      Value: {} units", output.value);
                println!("      Address: {}", output.address);
            }
            println!();
        }
    }
}
