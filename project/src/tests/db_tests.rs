#[cfg(test)]
mod tests {
    use crate::db::Db;
    use crate::model::block::BlockHeader;
    use crate::model::{Block, Transaction, TxOutput};
    use rusqlite::params;

    #[test]
    fn test_db_creation_and_schema() {
        let db = Db::open(Some(":memory:")).unwrap();
        db.init_schema().unwrap();

        // Verify tables exist by trying to query them
        let mut stmt = db
            .get_conn()
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"block_headers".to_string()));
        assert!(tables.contains(&"transactions".to_string()));
        assert!(tables.contains(&"utxos".to_string()));
    }

    #[test]
    fn test_mempool_operations() {
        let db = Db::open(Some(":memory:")).unwrap();
        db.init_schema().unwrap();

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
        db.insert_mempool_tx(&tx).unwrap();

        // Retrieve it
        let txid = tx.id();
        let retrieved = db.get_transaction(&txid).unwrap();
        assert!(retrieved.is_some());

        // Remove from mempool
        db.remove_mempool_tx(&txid).unwrap();
        let retrieved = db.get_transaction(&txid).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_get_utxos_for_address() {
        let db = Db::open(Some(":memory:")).unwrap();
        db.init_schema().unwrap();

        let addr = "test_address";
        let txid = [1u8; 32];

        // Manually insert a UTXO
        db.get_conn()
            .execute(
                "INSERT INTO utxos (txid, vout, value, addr) VALUES (?1, ?2, ?3, ?4)",
                params![txid.as_slice(), 0i64, 50i64, addr],
            )
            .unwrap();

        let utxos = db.get_utxos_for_address(addr).unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].index, 0);
        assert_eq!(utxos[0].output.value, 50.0);
    }

    #[test]
    fn test_get_utxos_for_addresses_empty_list() {
        let db = Db::open(Some(":memory:")).unwrap();
        db.init_schema().unwrap();

        let utxos = db.get_utxos_for_addresses(&Vec::new()).unwrap();
        assert!(utxos.is_empty());
    }

    #[test]
    fn test_get_utxos_for_addresses_multiple() {
        let db = Db::open(Some(":memory:")).unwrap();
        db.init_schema().unwrap();

        let addr1 = "addr_one".to_string();
        let addr2 = "addr_two".to_string();
        let other_addr = "other".to_string();

        let txid1 = [1u8; 32];
        let txid2 = [2u8; 32];
        let txid3 = [3u8; 32];

        // Insert UTXOs for multiple addresses and one that should be filtered out
        db.get_conn()
            .execute(
                "INSERT INTO utxos (txid, vout, value, addr) VALUES (?1, ?2, ?3, ?4)",
                params![txid1.as_slice(), 0i64, 25i64, &addr1],
            )
            .unwrap();

        db.get_conn()
            .execute(
                "INSERT INTO utxos (txid, vout, value, addr) VALUES (?1, ?2, ?3, ?4)",
                params![txid2.as_slice(), 1i64, 75i64, &addr2],
            )
            .unwrap();

        db.get_conn()
            .execute(
                "INSERT INTO utxos (txid, vout, value, addr) VALUES (?1, ?2, ?3, ?4)",
                params![txid3.as_slice(), 0i64, 10i64, &other_addr],
            )
            .unwrap();

        let mut utxos = db
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
        let mut db = Db::open(Some(":memory:")).unwrap();
        db.init_schema().unwrap();

        use chrono::Utc;

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
        db.apply_block(block, &[tx]).unwrap();

        // Verify the transaction was stored
        let retrieved = db.get_transaction(&txid).unwrap();
        assert!(retrieved.is_some());

        // Verify UTXOs were created
        let utxos = db.get_utxos_for_address("miner_address").unwrap();
        assert_eq!(utxos.len(), 1);
        assert_eq!(utxos[0].output.value, 50.0);

        // Verify block header was stored
        let mut stmt = db
            .get_conn()
            .prepare("SELECT height FROM block_headers WHERE height = 0")
            .unwrap();
        let count = stmt
            .query_map([], |row| row.get::<_, i64>(0))
            .unwrap()
            .count();
        assert_eq!(count, 1);
    }
}
