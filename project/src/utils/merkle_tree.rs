use crate::security_utils::sha256;

fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let data = [*left, *right].concat();
    let hash = sha256(&data);
    hash
}

fn hash_leaf(data: &[u8]) -> [u8; 32] {
    sha256(data)
}

#[derive(Debug, Clone)]
pub struct ProofNode {
    pub hash: [u8; 32],
    pub is_left: bool,
}

#[derive(Debug, Clone)]
pub struct MerkleTree {
    levels: Vec<Vec<[u8; 32]>>,
}
impl MerkleTree {
    pub fn from_leaves(leaves: Vec<[u8; 32]>) -> Self {
        if leaves.is_empty() {
            let empty = hash_leaf(b"");
            return MerkleTree {
                levels: vec![vec![empty]],
            };
        }

        let mut levels: Vec<Vec<[u8; 32]>> = Vec::new();
        levels.push(leaves);

        while levels.last().unwrap().len() > 1 {
            let curr_lv = levels.last().unwrap();
            let mut next_level: Vec<[u8; 32]> = Vec::with_capacity((curr_lv.len() + 1) / 2);

            for i in (0..curr_lv.len()).step_by(2) {
                let left = curr_lv[i];
                let right = if i + 1 < curr_lv.len() {
                    curr_lv[i + 1]
                } else {
                    // odd number of nodes: use the last node again as right child
                    curr_lv[i]
                };
                let parent = hash_pair(&left, &right);
                next_level.push(parent);
            }

            levels.push(next_level);
        }

        MerkleTree { levels }
    }

    pub fn root(&self) -> [u8; 32] {
        self.levels.last().unwrap()[0]
    }

    pub fn get_proof(&self, index: usize) -> Option<Vec<ProofNode>> {
        if self.levels.is_empty() {
            return None;
        }
        if index >= self.levels[0].len() {
            return None;
        }

        let mut proof: Vec<ProofNode> = Vec::new();
        let mut idx = index;

        for level in 0..(self.levels.len() - 1) {
            let nodes = &self.levels[level];
            let is_right = (idx % 2) == 1;
            let sibling_index = if is_right { idx - 1 } else { idx + 1 };

            if sibling_index < nodes.len() {
                let sibling = nodes[sibling_index];
                let sibling_is_left = sibling_index < idx;
                proof.push(ProofNode {
                    hash: sibling,
                    is_left: sibling_is_left,
                });
            } else {
                let sibling = nodes[idx];
                let sibling_is_left = false;
                proof.push(ProofNode {
                    hash: sibling,
                    is_left: sibling_is_left,
                });
            }

            idx = idx / 2;
        }

        Some(proof)
    }

    pub fn verify_proof(
        leaf_hash: &[u8; 32],
        proof: &Vec<ProofNode>,
        expected_root: &[u8; 32],
    ) -> bool {
        let mut cur = *leaf_hash;
        for node in proof {
            if node.is_left {
                cur = hash_pair(&node.hash, &cur);
            } else {
                cur = hash_pair(&cur, &node.hash);
            }
        }
        &cur == expected_root
    }
}
