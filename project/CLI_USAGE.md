# Cryptocurrency Node Interactive CLI

## Overview
This is an **interactive** CLI that provides a command-line interface to interact with the cryptocurrency node functionalities, including blockchain management, mining, wallet operations, and transaction handling.

The CLI runs in **interactive mode** (REPL-style), meaning the program starts once and waits for your commands. You can type multiple commands without restarting the program.

## Installation

Build the project:
```bash
cd project
cargo build --release
```

The binary will be available at `target/release/project`

## Quick Start

1. **Start the interactive CLI**
   ```bash
   cargo run
   ```

2. **The program welcomes you and shows the prompt**
   ```
   🔗 Cryptocurrency Node Interactive CLI 🔗
   
   Welcome! Type 'help' for available commands or 'exit' to quit.
   
   > 
   ```

3. **Type commands at the prompt**
   ```
   > help
   > mine block
   > chain status
   > exit
   ```

## Interactive Mode

The CLI operates in **interactive mode**:
- The program starts once and waits for your input
- Type commands one at a time at the `>` prompt
- The node persists across all commands (no reinitialization needed)
- Type `exit`, `quit`, or `q` to exit the program
- Type `help` or `?` to see available commands

**Example session:**
```
$ cargo run

> mine block
⛏  Mining new block...
✓ Block mined successfully!

> chain status
=== Blockchain Status ===
  Blocks: 1

> wallet send --to <address> --amount 50
✓ Transaction created

> exit
Goodbye!
```

## Available Commands

### General Commands

#### Help
```
help
```
or
```
?
```
Shows all available commands with descriptions and categories.

#### Exit
```
exit
```
or
```
quit
```
or
```
q
```
Exits the interactive CLI gracefully.

#### Reinitialize Node
```
init
```
Reinitializes and reloads the node. Loads existing blockchain if available.

---

### Mining Operations

#### Mine Block
```
mine block
```
Mines a new block with pending transactions from the mempool. Automatically saves the blockchain after mining.

**Example:**
```
> mine block
⛏  Mining new block...
✓ Block mined successfully!
  Block hash: 006538cc...
  Transactions: 1
  Nonce: 225
✓ Blockchain saved
```

---

### Blockchain Operations

#### Show Blockchain
```
chain show
```
Displays the complete blockchain with all blocks and transactions.

**Example:**
```
> chain show

=== Blockchain ===

Block #0
  Hash: 006538cc...
  Transactions: 1
    Transaction #0
      ID: affb544d...
```

#### Validate Blockchain
```
chain validate
```
Validates the integrity of the blockchain by checking block hashes and proof of work.

**Example:**
```
> chain validate
✓ Blockchain is valid
```

#### Save Blockchain
```
chain save
```
Manually saves the blockchain to disk (mining operations do this automatically).

**Example:**
```
> chain save
✓ Blockchain saved to disk
```

#### Blockchain Status
```
chain status
```
Shows blockchain statistics including number of blocks, validity status, and last block information.

**Example:**
```
> chain status

=== Blockchain Status ===
  Blocks: 2
  Valid: Yes
  Last Block Hash: 0052299...
  Last Block Date: 2025-11-09 23:57:50
```

---

### Wallet Operations

#### Create New Wallet
```
wallet new --seed "your seed phrase here"
```
Creates a new wallet from a seed phrase and displays the first address.

**Example:**
```
> wallet new --seed "my secure wallet seed"
✓ Wallet created successfully
  First address: 11Q8aHXF...
```

**Validation:** Seed cannot be empty.

#### Get New Address
```
wallet address
```
Generates a new receive address from the miner's wallet.

**Example:**
```
> wallet address
✓ New receive address: 11LsaodnPU7...
```

#### Check Balance
```
wallet balance --seed "your seed phrase"
```
Checks the balance of a wallet by scanning the blockchain for UTXOs.

**Example:**
```
> wallet balance --seed "seed do miguel!"

=== Wallet Balance ===
  UTXOs: 2
  Total Balance: 150 coins
  
  Details:
    UTXO #0: 100 coins to 112KHrbw...
    UTXO #1: 50 coins to 11Lsaodn...
```

**Validation:** Seed cannot be empty.

#### Send Transaction
```
wallet send --to <address> --amount <value> [--message <text>]
```
Creates and sends a transaction from the miner's wallet to a recipient address.

**Example:**
```
> wallet send --to 11LsaodnPU7JPi7qiBapAtiAUeG5PWiPZ59 --amount 50 --message "Payment"
✓ Transaction created and added to mempool
  To: 11LsaodnPU7JPi...
  Amount: 50 coins
  Message: Payment
  
  Use 'mine block' to include it in the blockchain
```

**Validations:**
- Address cannot be empty
- Amount must be a positive number (> 0)
- Address format is validated by the wallet module

**Note:** Transaction will be added to the mempool. Use `mine block` to include it in the blockchain.

#### Generate Keys
```
wallet generate-keys [--count <number>]
```
Generates multiple keys from the miner's wallet. Default count is 5.

**Example:**
```
> wallet generate-keys --count 3
✓ Generated 3 keys:

Key #1
  Address: 112KHrbwLp7...
  Public Key: 40295c8478...

Key #2
  Address: 11LsaodnPU7...
  Public Key: 76d2d82474...
```

**Validations:**
- Count must be between 1 and 100
- Default is 5 if not specified

---

### Transaction Operations

#### View Transaction
```
transaction view --id <hex_id>
```
Views details of a specific transaction by its ID (in hex format).

**Example:**
```
> transaction view --id affb544d36f97414f1764b98893a355338bc6cf5d8922adf108a73aad4dc3072

=== Transaction Details ===
  ID: affb544d36f9...
  Date: 2025-11-09 23:57:18
  Message: Coinbase transaction
  
  Inputs (0):
  
  Outputs (1):
    Output #0
      Value: 100 coins
      Address: 112KHrbwLp7...
```

**Validation:** Transaction ID must be exactly 64 hexadecimal characters.

---

## Typical Workflow

### Setting Up and Mining

```
> mine block
⛏  Mining new block...
✓ Block mined successfully!

> chain status
=== Blockchain Status ===
  Blocks: 1
```

### Creating and Sending Transactions

```
> wallet new --seed "recipient wallet seed"
✓ Wallet created successfully
  First address: 11Q8aHXF...

> wallet send --to 11Q8aHXF... --amount 100 --message "Initial funding"
✓ Transaction created and added to mempool

> mine block
⛏  Mining new block...
✓ Block mined successfully!

> wallet balance --seed "recipient wallet seed"
=== Wallet Balance ===
  UTXOs: 1
  Total Balance: 100 coins
```

### Viewing Blockchain Information

```
> chain show
=== Blockchain ===
[Shows all blocks and transactions]

> chain validate
✓ Blockchain is valid

> chain status
=== Blockchain Status ===
  Blocks: 2
  Valid: Yes
```

---

## Input Validations

The CLI includes comprehensive input validation:

| Command | Validation |
|---------|-----------|
| `wallet new --seed` | Seed cannot be empty |
| `wallet balance --seed` | Seed cannot be empty |
| `wallet send --amount` | Must be positive number (> 0) |
| `wallet send --to` | Address cannot be empty, format validated |
| `wallet generate-keys --count` | Must be 1-100 |
| `transaction view --id` | Must be exactly 64 hex characters |

All validation errors provide clear, actionable error messages with the ✗ symbol.

---

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
```
> wallet balance --seed "seed do miguel!"
```

---

## Notes

- The node initializes once at startup and persists across all commands
- All blockchain data is automatically saved after mining operations
- The blockchain is persisted in the `saved_files/` directory
- Transactions in the mempool are kept until the next mining operation
- All addresses use hierarchical deterministic (HD) key derivation
- Press Ctrl+C to force exit if needed (though `exit` is recommended)

---

## Troubleshooting

**Problem:** Command not recognized
- **Solution:** Type `help` to see all available commands

**Problem:** Blockchain is empty
- **Solution:** Use `mine block` to create the genesis block

**Problem:** Transaction fails
- **Solution:** Check that you have sufficient balance with `wallet balance --seed "seed do miguel!"`

**Problem:** Can't exit
- **Solution:** Type `exit`, `quit`, or `q`, or press Ctrl+C

---

## Examples of Common Tasks

### Check if blockchain exists and mine if needed
```
> chain status
> mine block
```

### Send coins to another wallet
```
> wallet new --seed "recipient"
> wallet send --to <address_from_above> --amount 50
> mine block
> wallet balance --seed "recipient"
```

### View a transaction
```
> chain show
[Copy a transaction ID]
> transaction view --id <transaction_id>
```

### Generate multiple addresses
```
> wallet generate-keys --count 10
```
