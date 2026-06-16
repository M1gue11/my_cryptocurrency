use chrono::NaiveDate;
use primitive_types::U256;

use project::model::{Block, Blockchain, Transaction, block::BlockHeader};
use project::utils::{ForkHelper, ForkUpdateStatus};

fn test_block(prev_block_hash: [u8; 32], nonce: u32) -> Block {
    let timestamp = NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, nonce)
        .unwrap();

    let mut block = Block {
        header: BlockHeader {
            prev_block_hash,
            merkle_root: [0; 32],
            nonce,
            timestamp,
            target: U256::MAX,
        },
        transactions: vec![Transaction::new_coinbase(format!("miner-{}", nonce), 0)],
    };
    block.evaluate_merkle_root();
    block
}

fn blockchain_with(chain: Vec<Block>) -> Blockchain {
    Blockchain { chain }
}

#[test]
fn stores_child_before_parent_and_connects_when_parent_arrives() {
    let ancestor = test_block([0; 32], 1);
    let parent = test_block(ancestor.id(), 2);
    let child = test_block(parent.id(), 3);
    let mut helper = ForkHelper::with_capacity_limit(1000);
    let blockchain = blockchain_with(vec![ancestor.clone()]);

    let update = helper.observe_block(&blockchain, child.clone(), None);
    assert!(matches!(update.status, ForkUpdateStatus::Stored));
    assert_eq!(update.missing_parents, vec![parent.id()]);
    assert!(update.connectable_blocks.is_empty());

    let update = helper.observe_block(&blockchain, parent.clone(), None);
    assert!(matches!(update.status, ForkUpdateStatus::Stored));
    assert_eq!(
        update
            .connectable_blocks
            .iter()
            .map(|block| block.id())
            .collect::<Vec<_>>(),
        vec![parent.id(), child.id()]
    );
    assert!(!helper.contains_block(&parent.id()));
    assert!(!helper.contains_block(&child.id()));
}

#[test]
fn does_not_auto_connect_when_parent_has_multiple_children() {
    let parent = test_block([9; 32], 1);
    let left = test_block(parent.id(), 2);
    let right = test_block(parent.id(), 3);
    let mut helper = ForkHelper::with_capacity_limit(1000);
    let blockchain = blockchain_with(Vec::new());

    helper.observe_block(&blockchain, left, None);
    helper.observe_block(&blockchain, right, None);

    let connectable_blocks = helper.take_connectable_blocks(parent.id());

    assert!(connectable_blocks.is_empty());
}

#[test]
fn selects_longer_branch_for_reorg_without_local_tie() {
    let genesis = test_block([0; 32], 1);
    let local_second = test_block(genesis.id(), 2);
    let fork_first = test_block(genesis.id(), 3);
    let fork_second = test_block(fork_first.id(), 4);
    let fork_third = test_block(fork_second.id(), 5);
    let blockchain = blockchain_with(vec![genesis.clone(), local_second]);
    let mut helper = ForkHelper::with_capacity_limit(1000);

    helper.observe_block(&blockchain, fork_first.clone(), None);
    helper.observe_block(&blockchain, fork_second.clone(), None);
    let update = helper.observe_block(&blockchain, fork_third.clone(), None);

    let candidate = update.best_reorg.expect("longer fork should win");
    assert_eq!(candidate.ancestor_hash, genesis.id());
    assert_eq!(candidate.candidate_height, 4);
    assert_eq!(
        candidate
            .blocks
            .iter()
            .map(|block| block.id())
            .collect::<Vec<_>>(),
        vec![fork_first.id(), fork_second.id(), fork_third.id()]
    );
}

#[test]
fn does_not_reorg_when_candidate_ties_local_height() {
    let genesis = test_block([0; 32], 1);
    let local_second = test_block(genesis.id(), 2);
    let fork_first = test_block(genesis.id(), 3);
    let blockchain = blockchain_with(vec![genesis, local_second]);
    let mut helper = ForkHelper::with_capacity_limit(1000);

    let update = helper.observe_block(&blockchain, fork_first, None);

    assert!(update.best_reorg.is_none());
}

#[test]
fn prunes_oldest_blocks_when_capacity_is_exceeded() {
    let genesis = test_block([0; 32], 1);
    let first = test_block(genesis.id(), 2);
    let second = test_block(first.id(), 3);
    let third = test_block(second.id(), 4);
    let blockchain = blockchain_with(vec![genesis]);
    let mut helper = ForkHelper::with_capacity_limit(2);

    helper.observe_block(&blockchain, first.clone(), None);
    helper.observe_block(&blockchain, second.clone(), None);
    helper.observe_block(&blockchain, third.clone(), None);

    assert!(!helper.contains_block(&first.id()));
}
