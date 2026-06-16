use primitive_types::U256;
use std::path::Path;

use project::db::db::init_db;
use project::db::repository::LedgerRepository;
use project::globals::CONFIG;
use project::model::block::BlockHeader;
use project::model::{Block, Transaction, TxOutput};
use project::utils::get_current_timestamp;

fn reset_test_db() {
    let db_path = Path::new(&CONFIG.db_path);
    if db_path.exists() {
        let _ = std::fs::remove_file(db_path);
    }

    let wal_path = format!("{}-wal", CONFIG.db_path);
    let wal_path = Path::new(&wal_path);
    if wal_path.exists() {
        let _ = std::fs::remove_file(wal_path);
    }

    let shm_path = format!("{}-shm", CONFIG.db_path);
    let shm_path = Path::new(&shm_path);
    if shm_path.exists() {
        let _ = std::fs::remove_file(shm_path);
    }
}

#[test]
fn test_db_creation_and_schema() {
    reset_test_db();
    init_db();
    let repo = LedgerRepository::new();

    let utxos = repo.get_utxos_for_address("nonexistent").unwrap();
    assert!(utxos.is_empty());
}

#[test]
fn test_mempool_operations() {
    reset_test_db();
    init_db();
    let repo = LedgerRepository::new();

    let tx = Transaction::new(
        vec![],
        vec![TxOutput {
            value: 100,
            address: "test_address".to_string(),
        }],
        Some("test".to_string()),
    );

    repo.insert_mempool_tx(&tx).unwrap();

    let txid = tx.id();
    let retrieved = repo.get_transaction(&txid).unwrap();
    assert!(retrieved.is_some());

    repo.remove_mempool_tx(&txid).unwrap();
    let retrieved = repo.get_transaction(&txid).unwrap();
    assert!(retrieved.is_none());
}

#[test]
fn test_get_utxos_for_address() {
    reset_test_db();
    init_db();
    let mut repo = LedgerRepository::new();

    let addr = "test_address";
    let tx = Transaction::new(
        vec![],
        vec![TxOutput {
            value: 50,
            address: addr.to_string(),
        }],
        Some("utxo-test".to_string()),
    );
    let header = BlockHeader {
        prev_block_hash: [0u8; 32],
        merkle_root: [2u8; 32],
        nonce: 1,
        timestamp: get_current_timestamp(),
        target: U256::MAX,
    };
    let block = Block {
        header,
        transactions: vec![tx.clone()],
    };
    repo.apply_block(block).unwrap();

    let utxos = repo.get_utxos_for_address(addr).unwrap();
    assert_eq!(utxos.len(), 1);
    assert_eq!(utxos[0].index, 0);
    assert_eq!(utxos[0].output.value, 50);
}

#[test]
fn test_get_utxos_for_addresses_empty_list() {
    reset_test_db();
    init_db();
    let repo = LedgerRepository::new();

    let utxos = repo.get_utxos_for_addresses(&Vec::new()).unwrap();
    assert!(utxos.is_empty());
}

#[test]
fn test_get_utxos_for_addresses_multiple() {
    reset_test_db();
    init_db();
    let mut repo = LedgerRepository::new();

    let addr1 = "addr_one".to_string();
    let addr2 = "addr_two".to_string();
    let tx1 = Transaction::new(
        vec![],
        vec![TxOutput {
            value: 25,
            address: addr1.clone(),
        }],
        Some("addr1".to_string()),
    );
    let tx2 = Transaction::new(
        vec![],
        vec![TxOutput {
            value: 75,
            address: addr2.clone(),
        }],
        Some("addr2".to_string()),
    );
    let header = BlockHeader {
        prev_block_hash: [0u8; 32],
        merkle_root: [3u8; 32],
        nonce: 2,
        timestamp: get_current_timestamp(),
        target: U256::MAX,
    };
    let block = Block {
        header,
        transactions: vec![tx1, tx2],
    };
    repo.apply_block(block).unwrap();

    let mut utxos = repo
        .get_utxos_for_addresses(&vec![addr1.clone(), addr2.clone()])
        .unwrap();

    utxos.sort_by(|a, b| {
        a.output
            .address
            .cmp(&b.output.address)
            .then(a.index.cmp(&b.index))
    });

    assert_eq!(utxos.len(), 2);
    assert_eq!(utxos[0].output.address, addr1);
    assert_eq!(utxos[0].output.value, 25);
    assert_eq!(utxos[1].output.address, addr2);
    assert_eq!(utxos[1].output.value, 75);
}

#[test]
fn test_apply_block_genesis() {
    reset_test_db();
    init_db();
    let mut repo = LedgerRepository::new();

    let header = BlockHeader {
        prev_block_hash: [0u8; 32],
        merkle_root: [1u8; 32],
        nonce: 12345,
        timestamp: get_current_timestamp(),
        target: U256::MAX,
    };

    let tx = Transaction::new(
        vec![],
        vec![TxOutput {
            value: 50,
            address: "miner_address".to_string(),
        }],
        Some("Genesis block".to_string()),
    );

    let block = Block {
        header,
        transactions: vec![tx.clone()],
    };

    let txid = tx.id();

    repo.apply_block(block).unwrap();

    let retrieved = repo.get_transaction(&txid).unwrap();
    assert!(retrieved.is_some());

    let utxos = repo.get_utxos_for_address("miner_address").unwrap();
    assert_eq!(utxos.len(), 1);
    assert_eq!(utxos[0].output.value, 50);
}
