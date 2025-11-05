# Cryptocurrency Node CLI

## Overview
This CLI provides a command-line interface to interact with the cryptocurrency node functionalities, including blockchain management, mining, wallet operations, and transaction handling.

## Installation

Build the project:
```bash
cd project
cargo build --release
```

The binary will be available at `target/release/project`

## Quick Start

1. **Initialize the node**
   ```bash
   cargo run -- init
   ```

2. **Mine the genesis block**
   ```bash
   cargo run -- mine block
   ```

3. **Check blockchain status**
   ```bash
   cargo run -- chain status
   ```

## Available Commands

### General Commands

```bash
cargo run -- --help
```

Shows all available commands and options.

### Node Operations

#### Initialize Node
```bash
cargo run -- init
```
Initializes and starts the node. Loads existing blockchain if available.

### Mining Operations

#### Mine Block
```bash
cargo run -- mine block
```
Mines a new block with pending transactions from the mempool. Automatically saves the blockchain after mining.

### Blockchain Operations

#### Show Blockchain
```bash
cargo run -- chain show
```
Displays the complete blockchain with all blocks and transactions.

#### Validate Blockchain
```bash
cargo run -- chain validate
```
Validates the integrity of the blockchain by checking block hashes and proof of work.

#### Save Blockchain
```bash
cargo run -- chain save
```
Manually saves the blockchain to disk (mining operations do this automatically).

#### Blockchain Status
```bash
cargo run -- chain status
```
Shows blockchain statistics including number of blocks, validity status, and last block information.

### Wallet Operations

#### Create New Wallet
```bash
cargo run -- wallet new --seed "your seed phrase here"
```
Creates a new wallet from a seed phrase and displays the first address.

**Example:**
```bash
cargo run -- wallet new --seed "my secure wallet seed"
```

#### Get New Address
```bash
cargo run -- wallet address
```
Generates a new receive address from the miner's wallet.

#### Check Balance
```bash
cargo run -- wallet balance --seed "your seed phrase"
```
Checks the balance of a wallet by scanning the blockchain for UTXOs.

**Example:**
```bash
cargo run -- wallet balance --seed "seed do miguel!"
```

#### Send Transaction
```bash
cargo run -- wallet send --to <address> --amount <value> [--message <text>]
```
Creates and sends a transaction from the miner's wallet to a recipient address.

**Example:**
```bash
cargo run -- wallet send --to 118YR7eQT932ijiSkCFy88YhPFHm8iWp7Rm --amount 50 --message "Payment for services"
```

Note: Transaction will be added to the mempool. Use `mine block` to include it in the blockchain.

#### Generate Keys
```bash
cargo run -- wallet generate-keys [--count <number>]
```
Generates multiple keys from the miner's wallet. Default count is 5.

**Example:**
```bash
cargo run -- wallet generate-keys --count 10
```

### Transaction Operations

#### View Transaction
```bash
cargo run -- transaction view --id <transaction_id_hex>
```
Views details of a specific transaction by its ID (in hex format).

**Example:**
```bash
cargo run -- transaction view --id affb544d36f97414f1764b98893a355338bc6cf5d8922adf108a73aad4dc3072
```

## Typical Workflow

### Setting Up and Mining

```bash
# 1. Initialize the node
cargo run -- init

# 2. Mine the genesis block
cargo run -- mine block

# 3. Check the status
cargo run -- chain status
```

### Creating and Sending Transactions

```bash
# 1. Create a new recipient wallet
cargo run -- wallet new --seed "recipient wallet seed"
# Note the address from output

# 2. Send coins from miner wallet
cargo run -- wallet send --to <recipient_address> --amount 100 --message "Initial funding"

# 3. Mine the block to include the transaction
cargo run -- mine block

# 4. Verify the transaction
cargo run -- chain show

# 5. Check recipient balance
cargo run -- wallet balance --seed "recipient wallet seed"
```

### Viewing Blockchain Information

```bash
# View entire blockchain
cargo run -- chain show

# Validate blockchain
cargo run -- chain validate

# Check status
cargo run -- chain status
```

## Configuration

The node configuration is stored in `Settings.toml`:

```toml
difficulty = 8                      # Mining difficulty (leading zero bits)
persisted_chain_path = "saved_files/"  # Blockchain storage location
block_reward = 100                  # Mining reward in coins
```

## Miner Wallet

The default miner wallet uses the seed: `"seed do miguel!"`

To check the miner's balance:
```bash
cargo run -- wallet balance --seed "seed do miguel!"
```

## Notes

- All blockchain data is automatically saved after mining operations
- The blockchain is persisted in the `saved_files/` directory
- Each command initializes the node independently, loading the saved blockchain state
- Transactions in the mempool are not persisted between runs
- All addresses use hierarchical deterministic (HD) key derivation
