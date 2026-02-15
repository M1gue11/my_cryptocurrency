# Caramuru - Educational Cryptocurrency

Blockchain cryptocurrency implementation developed as an undergraduate thesis in Computer Science at PUC-Rio. Complete system with custom blockchain, P2P network, HD wallet, mining capabilities, and web interface.

## Features

### Blockchain
- **Proof-of-Work consensus** with configurable difficulty (default: 8 leading zero bits)
- **UTXO model** for transaction management
- **Merkle root verification** for block integrity
- **Double-spending detection** within blocks
- **Block size limit**: 1 KB
- **Block reward**: 1 COIN (1,000,000 satoshis) + transaction fees
- **Mempool** with fee-rate prioritization
- **Persistence**: JSON and SQLite database

### Wallet
- **HD Wallet** (Hierarchical Deterministic) with BIP32-like derivation
- **Ed25519 signatures** for transaction signing
- **Base58Check address encoding** (SHA256 + RIPEMD160)
- **UTXO selection** with greedy coin selection algorithm
- **Gap limit strategy** (limit: 20) for address discovery
- **Encrypted keystore** (PBKDF2 + AES-GCM)
- **Multi-wallet support**

### Network
- **P2P network** with TCP connections (port 6000)
- **Message broadcasting** for blocks and transactions
- **Inventory protocol** for synchronization
- **Version exchange** between nodes
- **Fork detection** and handling

### Mining
- **Mempool-based mining** with fee prioritization
- **Coinbase transactions** for mining rewards
- **Nonce-based Proof-of-Work**
- **Configurable max attempts** (default: 3)

### Interface
- **CLI** with interactive commands
- **JSON-RPC API** (port 7000) for programmatic access
- **HTTP API** (port 7001) for frontend
- **Web interface** (React) for blockchain explorer and wallet management

## Tech Stack

**Backend (Rust)**
- `tokio` - Async runtime
- `axum` - HTTP framework
- `ed25519-dalek` - Digital signatures
- `sha2` & `ripemd` - Cryptographic hashing
- `rusqlite` + `r2d2` - SQLite with connection pooling
- `clap` - CLI framework

**Frontend (React/TypeScript)**
- `react` 19 - UI framework
- `react-router-dom` - Routing
- `vite` - Build tool
- `tailwindcss` - Styling

## Prerequisites

- **Rust** 1.80+ ([install](https://rustup.rs/))
- **Node.js** 18+ ([install](https://nodejs.org/))
- **SQLite** 3.x (usually pre-installed on Linux/Mac)

## Installation

### Backend (Rust Node)

```bash
cd project
cargo build --release
```

Binary will be at `project/target/release/caramuru` (or `caramuru.exe` on Windows).

### Frontend (React)

```bash
cd frontend
npm install
```

## Environment Variables

Create a `.env` file in the `project/` directory:

```bash
# Copy the example
cp .env.example .env
```

### Configuration Options

```env
# Blockchain Configuration
PERSISTED_CHAIN_PATH=saved_files/              # Where to save blockchain JSON
DB_PATH=saved_files/bd/caramuru_main_db.db    # SQLite database path
MAX_MINING_ATTEMPTS=3                          # Mining attempts before giving up

# Wallet Configuration
MINER_WALLET_SEED_PATH=keys/miner_wallet.json # Miner wallet keystore
MINER_WALLET_PASSWORD=miner123                 # Miner wallet password

# Network Configuration
P2P_PORT=6000                                  # P2P network port
PEERS=18.116.162.147:6000                     # Comma-separated peer addresses

# RPC Configuration
RPC_PORT=7000                                  # JSON-RPC server port
```

### Important Notes

- `PERSISTED_CHAIN_PATH`: Directory will be created automatically
- `DB_PATH`: Database file will be created on first run
- `MINER_WALLET_SEED_PATH`: Must exist before mining (create with `wallet new` command)
- `PEERS`: Leave empty for standalone node, or list peer addresses for network sync
- Ensure directories exist: `mkdir -p saved_files/bd keys`

## Running Locally

### Option 1: CLI Mode (Interactive)

Start node with CLI interface:

```bash
cd project
cargo run --release
```

Or if already built:

```bash
./target/release/caramuru
```

This will:
1. Start P2P server on port 6000
2. Start RPC server on port 7000 (background)
3. Open interactive CLI

### Option 2: HTTP Mode (For Frontend)

Start node with HTTP API:

```bash
cd project
cargo run --release -- daemon http
```

This will:
1. Start P2P server on port 6000
2. Start HTTP server on port 7001

Then start the frontend:

```bash
cd frontend
npm run dev
```

Frontend will be available at `http://localhost:5173`

### Option 3: Daemon + Attach

Start daemon in background:

```bash
cd project
cargo run --release -- daemon rpc &
```

Attach CLI to running daemon:

```bash
cargo run --release -- attach
```

## CLI Usage

### Initialize Blockchain

```bash
node init
```

Creates genesis block and initializes database.

### Wallet Management

```bash
# Create new wallet
wallet new --password mypassword --path keys/my_wallet.json

# Import existing wallet
wallet import --password mypassword --path keys/my_wallet.json

# Get new address
wallet address --name my_wallet

# Check balance
wallet balance --name my_wallet

# Send transaction
wallet send --name my_wallet --to <recipient_address> --amount 500000
```

### Mining

```bash
# Mine a single block (requires miner wallet configured in .env)
mine block
```

### Blockchain Operations

```bash
# Show entire blockchain
chain show

# Validate blockchain integrity
chain validate

# Show blockchain status (height, total difficulty)
chain status

# Show all UTXOs
chain utxos
```

### Node Operations

```bash
# Show mempool transactions
node mempool

# Clear mempool
node clear-mempool

# Show node status
node status

# Force save blockchain to disk
node save
```

### Transaction Operations

```bash
# View transaction details
tx view <transaction_id>
```

## Frontend Pages

- **Dashboard**: Node status, blockchain height, peer count
- **Blocks**: Blockchain explorer with search functionality
- **Transactions**: Transaction viewer with details
- **Wallet**: Import/create wallets, send transactions, view balance
- **Network**: Connected peers information

## Docker Deployment

Run multi-node network with Docker Compose:

```bash
cd project
docker-compose up
```

This creates:
- **root-node**: Port 6000 (P2P), 7001 (HTTP)
- **node-1**: Port 6001 (P2P), 7002 (HTTP)
- **node-2**: Port 6002 (P2P), 7003 (HTTP)

All nodes are automatically connected and will sync blocks/transactions.

### Access Docker Nodes

```bash
# View logs
docker-compose logs -f root-node

# Execute CLI commands
docker-compose exec root-node /usr/local/bin/node attach
```

## JSON-RPC API

The RPC server (port 7000) accepts JSON-RPC 2.0 requests:

```bash
# Example: Get blockchain status
curl -X POST http://localhost:7000/rpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"chain_status","params":[],"id":1}'
```

### Available RPC Methods

**Node**
- `node_status`, `node_init`, `node_mempool`, `node_clear_mempool`, `node_save`

**Chain**
- `chain_status`, `chain_show`, `chain_validate`, `chain_utxos`

**Mining**
- `mine_block`

**Wallet**
- `wallet_import`, `wallet_new`, `wallet_address`, `wallet_balance`, `wallet_send`, `wallet_generate_keys`

**Transactions**
- `transaction_view`

## Project Structure

```
project/                        # Rust backend
  src/
    main.rs                     # Entry point
    cli/                        # CLI interface
    daemon/                     # HTTP & RPC servers
    model/                      # Blockchain core (Block, Transaction, Wallet)
    db/                         # SQLite database layer
    network/                    # P2P network implementation
    security_utils/             # Cryptography (hashing, signing, encryption)
    utils/                      # Helper functions
  Cargo.toml                    # Rust dependencies
  .env.example                  # Environment variables template
  docker-compose.yml            # Multi-node setup

frontend/                       # React frontend
  src/
    App.tsx                     # Main app router
    pages/                      # Page components
    components/                 # Reusable UI components
    contexts/                   # React context (wallet state)
    services/                   # API clients
    types/                      # TypeScript types
  package.json                  # Node dependencies
```

## Consensus Rules

- **Difficulty**: 8 leading zero bits in block hash
- **Block Reward**: 1 COIN (1,000,000 satoshis)
- **Max Block Size**: 1 KB (1000 bytes)
- **HD Wallet Path**: `purpose/account/change/index` (custom: `111/0/0-1/index`)
- **Gap Limit**: 20 unused addresses

## Database Schema

**SQLite tables:**
- `block_headers` - Block metadata (hash, height, timestamp)
- `transactions` - Transaction data and block association
- `utxos` - Unspent transaction outputs
- `used_addresses` - Address tracking for gap limit
- `mempool_txs` - Pending transactions

## Common Workflows

### First Time Setup

```bash
# 1. Build the node
cd project
cargo build --release

# 2. Setup environment
cp .env.example .env
mkdir -p saved_files/bd keys

# 3. Start node
cargo run --release

# 4. Initialize blockchain
node init

# 5. Create miner wallet
wallet new --password miner123 --path keys/miner_wallet.json

# 6. Mine first block
mine block
```

### Send Transaction

```bash
# 1. Get recipient address (from another wallet)
wallet address --name recipient_wallet

# 2. Send coins
wallet send --name sender_wallet --to <address> --amount 100000

# 3. Mine block to confirm transaction
mine block

# 4. Verify balance
wallet balance --name recipient_wallet
```

### Run With Frontend

```bash
# Terminal 1: Start backend
cd project
cargo run --release -- daemon http

# Terminal 2: Start frontend
cd frontend
npm run dev

# Open browser: http://localhost:5173
```

## Troubleshooting

**"No UTXOs available"**
- Mine blocks to generate coins: `mine block`
- Check balance: `wallet balance --name <wallet>`

**"Wallet not found"**
- Import wallet first: `wallet import --password <pwd> --path <path>`
- Create new wallet: `wallet new --password <pwd> --path <path>`

**"Port already in use"**
- Change ports in `.env`: `P2P_PORT`, `RPC_PORT`
- Kill existing process: `lsof -ti:6000 | xargs kill` (Linux/Mac)

**"Database locked"**
- Stop other node instances
- Remove lock: `rm saved_files/bd/*.db-shm saved_files/bd/*.db-wal`

**Frontend can't connect**
- Ensure backend is running with `daemon http` mode
- Check HTTP server is on port 7001: `lsof -i:7001`

## License

Educational project for PUC-Rio Computer Science undergraduate thesis.

## Author

Miguel - PUC-Rio Computer Science
