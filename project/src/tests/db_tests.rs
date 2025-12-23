#[cfg(test)]
mod tests {
    use crate::db::db::init_db;
    use crate::db::repository::LedgerRepository;
    use crate::model::block::BlockHeader;
    use crate::model::{Block, Transaction, TxOutput};
    use chrono::Utc;

    #[test]
    fn test_db_creation_and_schema() {
        init_db();
        let repo = LedgerRepository::new();

        // Basic queries via repository should succeed on fresh schema
        let utxos = repo.get_utxos_for_address("nonexistent").unwrap();
        assert!(utxos.is_empty());
    }

    #[test]
    fn test_mempool_operations() {
        init_db();
        let repo = LedgerRepository::new();

        // Create a simple transaction
        let tx = Transaction::new(
            vec![],
            vec![TxOutput {
                value: 100.0,
                address: "test_address".to_string(),
            }],
            Some("test".to_string()),
        );

        // Insert into mempool
        repo.insert_mempool_tx(&tx).unwrap();

        // Retrieve it
        let txid = tx.id();
        let retrieved = repo.get_transaction(&txid).unwrap();
        assert!(retrieved.is_some());

        // Remove from mempool
        repo.remove_mempool_tx(&txid).unwrap();
        let retrieved = repo.get_transaction(&txid).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_utxos_for_address() {
        init_db();
        let mut repo = LedgerRepository::new();

        let addr = "test_address";
        let tx = Transaction::new(
            vec![],
            vec![TxOutput {
                value: 50.0,
                address: addr.to_string(),
            }],
            Some("utxo-test".to_string()),
        );
        let header = BlockHeader {
            prev_block_hash: [0u8; 32],
            merkle_root: [2u8; 32],
            nonce: 1,
            timestamp: Utc::now().naive_utc(),
        };
        let block = Block {
            header,
            transactions: vec![tx.clone()],
        };
        repo.apply_block(block, &[tx]).unwrap();

        let utxos = repo.get_utxos_for_address(addr).unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].index, 0);
        assert_eq!(utxos[0].output.value, 50.0);
    }

    #[test]
    fn test_get_utxos_for_addresses_empty_list() {
        init_db();
        let repo = LedgerRepository::new();

        let utxos = repo.get_utxos_for_addresses(&Vec::new()).unwrap();
        assert!(utxos.is_empty());
    }

    #[test]
    fn test_get_utxos_for_addresses_multiple() {
        init_db();
        let mut repo = LedgerRepository::new();

        let addr1 = "addr_one".to_string();
        let addr2 = "addr_two".to_string();
        let tx1 = Transaction::new(
            vec![],
            vec![TxOutput {
                value: 25.0,
                address: addr1.clone(),
            }],
            Some("addr1".to_string()),
        );
        let tx2 = Transaction::new(
            vec![],
            vec![TxOutput {
                value: 75.0,
                address: addr2.clone(),
            }],
            Some("addr2".to_string()),
        );
        let header = BlockHeader {
            prev_block_hash: [0u8; 32],
            merkle_root: [3u8; 32],
            nonce: 2,
            timestamp: Utc::now().naive_utc(),
        };
        let block = Block {
            header,
            transactions: vec![tx1.clone(), tx2.clone()],
        };
        repo.apply_block(block, &[tx1, tx2]).unwrap();

        let mut utxos = repo
            .get_utxos_for_addresses(&vec![addr1.clone(), addr2.clone()])
            .unwrap();

        // Sort to make assertions deterministic
        utxos.sort_by(|a, b| {
            a.output
                .address
                .cmp(&b.output.address)
                .then(a.index.cmp(&b.index))
        });

        assert_eq!(utxos.len(), 2);
        assert_eq!(utxos[0].output.address, addr1);
        assert_eq!(utxos[0].output.value, 25.0);
        assert_eq!(utxos[1].output.address, addr2);
        assert_eq!(utxos[1].output.value, 75.0);
    }

    #[test]
    fn test_apply_block_genesis() {
        init_db();
        let mut repo = LedgerRepository::new();

        // Create a genesis block header
        let header = BlockHeader {
            prev_block_hash: [0u8; 32],
            merkle_root: [1u8; 32],
            nonce: 12345,
            timestamp: Utc::now().naive_utc(),
        };

        // Create a coinbase transaction
        let tx = Transaction::new(
            vec![],
            vec![TxOutput {
                value: 50.0,
                address: "miner_address".to_string(),
            }],
            Some("Genesis block".to_string()),
        );

        let block = Block {
            header,
            transactions: vec![tx.clone()],
        };

        let txid = tx.id();

        // Apply the genesis block
        repo.apply_block(block, &[tx]).unwrap();

        // Verify the transaction was stored
        let retrieved = repo.get_transaction(&txid).unwrap();
        assert!(retrieved.is_some());

        // Verify UTXOs were created
        let utxos = repo.get_utxos_for_address("miner_address").unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].output.value, 50.0);
    }
}
