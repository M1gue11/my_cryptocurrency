use crate::cli::{ChainCommands, Commands, MineCommands, TransactionCommands, WalletCommands};
use crate::model::{TxOutput, Wallet, get_node, get_node_mut, init_node};

pub fn handle_command(command: Commands) {
    match command {
        Commands::Init => handle_init(),
        Commands::Mine(mine_cmd) => handle_mine(mine_cmd),
        Commands::Chain(chain_cmd) => handle_chain(chain_cmd),
        Commands::Wallet(wallet_cmd) => handle_wallet(wallet_cmd),
        Commands::Transaction(tx_cmd) => handle_transaction(tx_cmd),
    }
}

fn handle_init() {
    init_node();
    let node = get_node();
    
    println!("✓ Node initialized successfully");
    
    if node.is_chain_empty() {
        println!("⚠ Blockchain is empty. Use 'mine block' to create the genesis block.");
    } else {
        println!("✓ Loaded blockchain with {} blocks", node.blockchain.chain.len());
    }
}

fn handle_mine(command: MineCommands) {
    match command {
        MineCommands::Block => {
            init_node();
            let node = get_node_mut();
            
            println!("⛏ Mining new block...");
            let block = node.mine();
            
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
    init_node();
    
    match command {
        ChainCommands::Show => {
            let node = get_node();
            if node.is_chain_empty() {
                println!("⚠ Blockchain is empty");
                return;
            }
            
            println!("\n=== Blockchain ===\n");
            for (i, block) in node.blockchain.chain.iter().enumerate() {
                println!("Block #{}", i);
                println!("  Hash: {}", hex::encode(block.header_hash()));
                println!("  Previous Hash: {}", hex::encode(block.header.prev_block_hash));
                println!("  Merkle Root: {}", hex::encode(block.header.merkle_root));
                println!("  Nonce: {}", block.header.nonce);
                println!("  Date: {}", block.header.timestamp);
                println!("  Transactions: {}", block.transactions.len());
                
                for (j, tx) in block.transactions.iter().enumerate() {
                    println!("    Transaction #{}", j);
                    println!("      ID: {}", hex::encode(tx.id()));
                    println!("      Inputs: {}", tx.inputs.len());
                    println!("      Outputs: {}", tx.outputs.len());
                    if let Some(msg) = &tx.message {
                        println!("      Message: {}", msg);
                    }
                }
                println!();
            }
        }
        
        ChainCommands::Validate => {
            let node = get_node();
            let is_valid = node.validate_blockchain();
            
            if is_valid {
                println!("✓ Blockchain is valid");
            } else {
                println!("✗ Blockchain is invalid!");
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
            let is_valid = node.validate_blockchain();
            
            println!("\n=== Blockchain Status ===");
            println!("  Blocks: {}", block_count);
            println!("  Valid: {}", if is_valid { "Yes" } else { "No" });
            
            if block_count > 0 {
                let last_block = node.blockchain.chain.last().unwrap();
                println!("  Last Block Hash: {}", hex::encode(last_block.header_hash()));
                println!("  Last Block Date: {}", last_block.header.timestamp);
            }
            println!();
        }
    }
}

fn handle_wallet(command: WalletCommands) {
    init_node();
    let node = get_node_mut();
    
    match command {
        WalletCommands::New { seed } => {
            let mut wallet = Wallet::new(&seed);
            let address = wallet.get_receive_addr();
            
            println!("✓ Wallet created successfully");
            println!("  First address: {}", address);
        }
        
        WalletCommands::Address => {
            let address = node.miner.wallet.get_receive_addr();
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
                    println!("    UTXO #{}: {} coins to {}", i, utxo.output.value, utxo.output.address);
                }
            }
            println!();
        }
        
        WalletCommands::Send { to, amount, message } => {
            let outputs = vec![TxOutput {
                value: amount,
                address: to.clone(),
            }];
            
            match node.miner.wallet.send_tx(outputs, message.clone()) {
                Ok(tx) => {
                    match node.receive_transaction(tx) {
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
                    }
                }
                Err(e) => {
                    println!("✗ Error creating transaction: {}", e);
                }
            }
        }
        
        WalletCommands::GenerateKeys { count } => {
            let keys = node.miner.wallet.generate_n_keys(count);
            
            println!("✓ Generated {} keys:\n", count);
            for (i, key) in keys.iter().enumerate() {
                println!("Key #{}", i + 1);
                println!("  Address: {}", key.get_address());
                println!("  Public Key: {}", hex::encode(key.get_public_key().as_bytes()));
                println!();
            }
        }
    }
}

fn handle_transaction(command: TransactionCommands) {
    init_node();
    let node = get_node();
    
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
            
            match node.find_transaction(&tx_id_bytes) {
                Some(tx) => {
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
                None => {
                    println!("✗ Transaction not found");
                }
            }
        }
        
    }
}
