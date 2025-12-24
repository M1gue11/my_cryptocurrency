use super::cli::{ChainCommands, Commands, MineCommands, TransactionCommands, WalletCommands};
use crate::{
    db::repository::LedgerRepository,
    front::cli::NodeCommands,
    model::{TxOutput, Wallet, get_node, get_node_mut, init_node, wallet::DerivationType},
};
use std::io::{self, Write};

pub fn run_interactive_mode() {
    init_node();

    print_welcome();

    let mut loaded_wallets: Vec<(String, Wallet)> = {
        let node = get_node();
        vec![("miner_wallet".to_string(), node.miner.wallet.clone())]
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
                        if let Err(e) = execute_command(command, &mut loaded_wallets) {
                            println!("✗ Error: {}", e);
                        }
                    }
                    Err(e) => {
                        println!("✗ {}", e);
                        println!("  Type 'help' for available commands");
                    }
                }
            }
            Err(e) => {
                println!("✗ Error reading input: {}", e);
            }
        }
    }
}

fn resolve_wallet_by_name<'a>(
    name: Option<String>,
    loaded_wallets: &'a mut Vec<(String, Wallet)>,
) -> &'a mut Wallet {
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

fn print_welcome() {
    println!("\n╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                      ║");
    println!("║              🔗 CARAMURU Node Interactive CLI 🔗                     ║");
    println!("║                                                                      ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!("\nWelcome! Type 'help' for available commands or 'exit' to quit.\n");

    let node = get_node();
    if node.is_chain_empty() {
        println!("⚠  Blockchain is empty. Use 'mine block' to create the genesis block.");
    } else {
        println!(
            "✓  Loaded blockchain with {} blocks",
            node.blockchain.chain.len()
        );
    }
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

    println!("\n⛏  Mining:");
    println!("  mine block                 - Mine a new block with pending transactions");

    println!("\n🔗 Blockchain:");
    println!("  chain show                 - Display the entire blockchain");
    println!("  chain status               - Show blockchain status");
    println!("  chain validate             - Validate blockchain integrity");
    println!("  chain save                 - Save blockchain to disk");
    println!("  chain rollback --count <n> - Rollback N blocks (for debugging)");
    println!("  chain utxos [--limit <n>]  - Show at most <n> UTXOs");
    println!("    - Limit is optional, default is 10");

    println!("\n💰 Wallet:");
    println!("  wallet new --seed <seed> [--name <name>]");
    println!("    - Create a new wallet. If name is provided, wallet is stored in session.");

    println!("\n  wallet list");
    println!("    - List all loaded wallets in the current session.");

    println!("\n  wallet address [--name <name>]");
    println!("    - Get new receive address (miner's wallet by default)");

    println!("\n  wallet balance --seed <seed>");
    println!("    - Check wallet balance");

    println!("\n  wallet send [--from <name>] --to <addr> --amount <n> [--message <msg>]");
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
                return Err("Usage: node <init|mempool>".to_string());
            }
            match parts[1] {
                "init" => Ok(Commands::Node(NodeCommands::Init)),
                "mempool" => Ok(Commands::Node(NodeCommands::Mempool)),
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
                return Err("Usage: chain <show|status|validate|save|rollback>".to_string());
            }
            match parts[1] {
                "show" => Ok(Commands::Chain(ChainCommands::Show)),
                "status" => Ok(Commands::Chain(ChainCommands::Status)),
                "validate" => Ok(Commands::Chain(ChainCommands::Validate)),
                "save" => Ok(Commands::Chain(ChainCommands::Save)),
                "rollback" => {
                    let count_str = parse_flag_value(&parts, "--count")?;
                    let count = count_str.parse::<u32>().map_err(|_| {
                        "Invalid count format. Must be a positive number".to_string()
                    })?;
                    if count == 0 {
                        return Err("Count must be greater than zero".to_string());
                    }
                    Ok(Commands::Chain(ChainCommands::Rollback { count }))
                }
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
                    let seed = parse_flag_value(&parts, "--seed")?;
                    if seed.is_empty() {
                        return Err("Seed cannot be empty".to_string());
                    }
                    let name_result = parse_flag_value(&parts, "--name");
                    let name = match name_result {
                        Ok(n) if !n.is_empty() => Some(n),
                        _ => None,
                    };
                    Ok(Commands::Wallet(WalletCommands::New { seed, name }))
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
                    let seed = parse_flag_value(&parts, "--seed")?;
                    if seed.is_empty() {
                        return Err("Seed cannot be empty".to_string());
                    }
                    Ok(Commands::Wallet(WalletCommands::Balance { seed }))
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
                        .parse::<f64>()
                        .map_err(|_| "Invalid amount format. Must be a number".to_string())?;

                    if amount <= 0.0 {
                        return Err("Amount must be greater than zero".to_string());
                    }

                    let message = parse_flag_value(&parts, "--message").ok();

                    Ok(Commands::Wallet(WalletCommands::Send {
                        from,
                        to,
                        amount,
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

fn execute_command(
    command: Commands,
    loaded_wallets: &mut Vec<(String, Wallet)>,
) -> Result<(), String> {
    match command {
        Commands::Node(node_cmd) => {
            handle_node(node_cmd);
            Ok(())
        }
        Commands::Mine(mine_cmd) => {
            handle_mine(mine_cmd);
            Ok(())
        }
        Commands::Chain(chain_cmd) => {
            handle_chain(chain_cmd);
            Ok(())
        }
        Commands::Wallet(wallet_cmd) => {
            handle_wallet(wallet_cmd, loaded_wallets);
            Ok(())
        }
        Commands::Transaction(tx_cmd) => {
            handle_transaction(tx_cmd);
            Ok(())
        }
    }
}

fn handle_node(command: NodeCommands) {
    match command {
        NodeCommands::Init => {
            init_node();
            let node = get_node();

            println!("✓ Node reinitialized successfully");

            if node.is_chain_empty() {
                println!("⚠ Blockchain is empty. Use 'mine block' to create the genesis block.");
            } else {
                println!(
                    "✓ Loaded blockchain with {} blocks",
                    node.blockchain.chain.len()
                );
            }
        }
        NodeCommands::Mempool => {
            let node = get_node();

            if node.is_mempool_empty() {
                println!("⚠  Mempool is empty");
                return;
            }

            node.print_mempool();
        }
    }
}

fn handle_mine(command: MineCommands) {
    match command {
        MineCommands::Block => {
            let node = get_node_mut();

            println!("⛏  Mining new block...");
            let block = match node.mine() {
                Err(e) => {
                    println!("✗ Mining failed: {}", e);
                    return;
                }
                Ok(block) => block,
            };

            println!("✓ Block mined successfully!");
            println!("  Block hash: {}", hex::encode(block.header_hash()));
            println!("  Transactions: {}", block.transactions.len());
            println!("  Nonce: {}", block.header.nonce);

            // Save blockchain after mining
            node.save_node();
            println!("✓ Blockchain saved");
        }
    }
}

fn handle_chain(command: ChainCommands) {
    match command {
        ChainCommands::Show => {
            let node = get_node();
            if node.is_chain_empty() {
                println!("⚠  Blockchain is empty");
                return;
            }

            println!("\n=== Blockchain ===\n");
            for (i, block) in node.blockchain.chain.iter().enumerate() {
                println!("Block #{}", i);
                println!("  Hash: {}", hex::encode(block.header_hash()));
                println!(
                    "  Previous Hash: {}",
                    hex::encode(block.header.prev_block_hash)
                );
                println!("  Merkle Root: {}", hex::encode(block.header.merkle_root));
                println!("  Nonce: {}", block.header.nonce);
                println!("  Date: {}", block.header.timestamp);
                println!("  Transactions: {}", block.transactions.len());

                for (j, tx) in block.transactions.iter().enumerate() {
                    println!("\n    Transaction #{}", j);
                    println!("      ID: {}", hex::encode(tx.id()));
                    println!("      Amount: {}", tx.amount());
                    if let Some(msg) = &tx.message {
                        println!("      Message: {}", msg);
                    }
                    println!("      Inputs:");
                    for input in &tx.inputs {
                        println!(
                            "            Input Prev TX: {}",
                            hex::encode(input.prev_tx_id)
                        );
                        println!("            Input Output Index: {}\n", input.output_index);
                    }
                    println!("      Outputs:");
                    for output in &tx.outputs {
                        println!("            Output Value: {} coins", output.value);
                        println!("            Output Address: {}\n", output.address);
                    }
                }
                println!();
            }
        }

        ChainCommands::Validate => {
            let node = get_node();
            let validation = node.validate_bc();

            match validation {
                Ok(is_valid) => {
                    if is_valid {
                        println!("✓ Blockchain is valid");
                    } else {
                        println!("✗ Blockchain is invalid!");
                    }
                }
                Err(e) => {
                    println!("✗ Blockchain validation failed: {}", e);
                }
            }
        }

        ChainCommands::Save => {
            let node = get_node();
            node.save_node();
            println!("✓ Blockchain saved to disk");
        }

        ChainCommands::Status => {
            let node = get_node();
            let block_count = node.blockchain.chain.len();
            let validation = node.validate_bc();

            println!("\n=== Blockchain Status ===");
            println!("  Blocks: {}", block_count);
            println!("  Valid: {}", if validation.is_ok() { "Yes" } else { "No" });

            if block_count > 0 {
                let last_block = node.blockchain.chain.last().unwrap();
                println!(
                    "  Last Block Hash: {}",
                    hex::encode(last_block.header_hash())
                );
                println!("  Last Block Date: {}", last_block.header.timestamp);
            }
            println!();
        }

        ChainCommands::Rollback { count } => {
            let node = get_node_mut();
            let initial_blocks = node.blockchain.chain.len();

            match node.rollback_blocks(count) {
                Ok(()) => {
                    let final_blocks = node.blockchain.chain.len();
                    let removed = initial_blocks - final_blocks;

                    println!("✓ Successfully rolled back {} block(s)", removed);
                    println!("  Previous block count: {}", initial_blocks);
                    println!("  Current block count: {}", final_blocks);
                    println!("  Transactions restored to mempool: check with 'node mempool'");

                    // Auto-save after rollback
                    node.save_node();
                    println!("✓ Blockchain saved");
                }
                Err(e) => {
                    println!("✗ Rollback failed: {}", e);
                }
            }
        }

        ChainCommands::Utxos { limit } => {
            let repo = LedgerRepository::new();
            let utxos = repo.get_all_utxos(Some(limit as usize));

            if utxos.is_err() {
                println!("⚠  No UTXOs found in the blockchain");
                return;
            }

            let utxo_list = utxos.unwrap();
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

            for (i, utxo) in utxo_list.iter().take(limit as usize).enumerate() {
                let tx_id_hex = hex::encode(utxo.tx_id);
                let tx_id_short = format!("{}...{} ", &tx_id_hex[..6], &tx_id_hex[58..]);

                println!(
                    "│ {:>4} │ {} │ {:>5} │ {:>12.2} │ {:46} │",
                    i + 1,
                    tx_id_short,
                    utxo.index,
                    utxo.output.value,
                    utxo.output.address
                );
            }

            println!(
                "└──────┴──────────────────┴───────┴──────────────┴────────────────────────────────────────────────┘"
            );

            let total: f64 = utxo_list
                .iter()
                .take(limit as usize)
                .map(|u| u.output.value)
                .sum();
            println!(
                "\nTotal: {:.2} coins across {} UTXOs",
                total,
                utxo_list.len().min(limit as usize)
            );
            println!();
        }
    }
}

fn handle_wallet(command: WalletCommands, loaded_wallets: &mut Vec<(String, Wallet)>) {
    let node = get_node_mut();

    match command {
        WalletCommands::New { seed, name } => {
            let mut wallet = Wallet::new(&seed);
            let address = wallet.get_receive_addr();

            if name.is_some() {
                let name = name.unwrap();
                loaded_wallets.push((name.clone(), wallet));
            }

            println!("✓ Wallet created successfully");
            println!("  First address: {}", address);
        }

        WalletCommands::List => {
            println!("\n=== Loaded Wallets ===\n");
            for (name, w) in loaded_wallets.iter() {
                println!("Wallet: {} - Balance: {}", name, w.calculate_balance());
            }
        }

        WalletCommands::Address { name } => {
            // Select wallet, default to miner's wallet
            let wallet = resolve_wallet_by_name(name, loaded_wallets);
            let address = wallet.get_receive_addr();
            println!("✓ New receive address: {}", address);
        }

        WalletCommands::Balance { seed } => {
            let wallet = Wallet::new(&seed);
            let utxos = wallet.get_wallet_utxos();

            let total: f64 = utxos.iter().map(|u| u.output.value).sum();

            println!("\n=== Wallet Balance ===");
            println!("  UTXOs: {}", utxos.len());
            println!("  Total Balance: {} coins", total);

            if !utxos.is_empty() {
                println!("\n  Details:");
                for (i, utxo) in utxos.iter().enumerate() {
                    println!(
                        "    UTXO #{}: {} coins to {}",
                        i, utxo.output.value, utxo.output.address
                    );
                }
            }
            println!();
        }

        WalletCommands::Send {
            from,
            to,
            amount,
            message,
        } => {
            let outputs = vec![TxOutput {
                value: amount,
                address: to.clone(),
            }];
            let wallet = resolve_wallet_by_name(from, loaded_wallets);

            match wallet.send_tx(outputs, message.clone()) {
                Ok(tx) => match node.receive_transaction(tx) {
                    Ok(_) => {
                        println!("✓ Transaction created and added to mempool");
                        println!("  To: {}", to);
                        println!("  Amount: {} coins", amount);
                        if let Some(msg) = message {
                            println!("  Message: {}", msg);
                        }
                        println!("\n  Use 'mine block' to include it in the blockchain");
                    }
                    Err(e) => {
                        println!("✗ Error receiving transaction: {}", e);
                    }
                },
                Err(e) => {
                    println!("✗ Error creating transaction: {}", e);
                }
            }
        }

        WalletCommands::GenerateKeys { count, name, type_ } => {
            let wallet = resolve_wallet_by_name(name, loaded_wallets);
            let keys = wallet.generate_n_keys(
                count,
                None,
                type_.map(|t| {
                    if t == 0 {
                        DerivationType::Receive
                    } else {
                        DerivationType::Change
                    }
                }),
            );

            println!("✓ Generated {} keys:\n", count);
            for (i, key) in keys.iter().enumerate() {
                println!("Key #{}", i + 1);
                println!("  Address: {}", key.get_address());
                println!(
                    "  Public Key: {}",
                    hex::encode(key.get_public_key().as_bytes())
                );
                println!();
            }
        }
    }
}

fn handle_transaction(command: TransactionCommands) {
    match command {
        TransactionCommands::View { id } => {
            let tx_id_bytes = match hex::decode(&id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut array = [0u8; 32];
                    array.copy_from_slice(&bytes);
                    array
                }
                _ => {
                    println!("✗ Invalid transaction ID format. Must be 64 hex characters.");
                    return;
                }
            };

            let repo = LedgerRepository::new();
            match repo.get_transaction(&tx_id_bytes) {
                Ok(tx) => {
                    println!("\n=== Transaction Details ===");
                    println!("  ID: {}", hex::encode(tx.id()));
                    println!("  Date: {}", tx.date);

                    if let Some(msg) = &tx.message {
                        println!("  Message: {}", msg);
                    }

                    println!("\n  Inputs ({}): ", tx.inputs.len());
                    for (i, input) in tx.inputs.iter().enumerate() {
                        println!("    Input #{}", i);
                        println!("      Previous TX: {}", hex::encode(input.prev_tx_id));
                        println!("      Output Index: {}", input.output_index);
                        println!("      Public Key: {}", input.public_key);
                    }

                    println!("\n  Outputs ({}):", tx.outputs.len());
                    for (i, output) in tx.outputs.iter().enumerate() {
                        println!("    Output #{}", i);
                        println!("      Value: {} coins", output.value);
                        println!("      Address: {}", output.address);
                    }
                    println!();
                }
                Err(_) => {
                    println!("✗ Transaction not found");
                }
            }
        }
    }
}
