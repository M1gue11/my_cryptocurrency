// use crate::globals::NODE;
use crate::{
    model::{HDKey, Transaction},
    security_utils::{public_key_to_hex, sign_hash},
};
use ed25519_dalek::SigningKey;
use ed25519_dalek::VerifyingKey;

pub struct Wallet {
    master_hdkey: HDKey,
    current_index: u32,
}

const BASE_PATH: [u32; 4] = [111, 0, 0, 0];
impl Wallet {
    pub fn new(seed: &str) -> Self {
        // TODO: improve seed generation
        let hdkey = HDKey::new(seed.as_bytes());
        Wallet {
            master_hdkey: hdkey,
            current_index: 0,
        }
    }

    pub fn derive_path(&self, path: &[u32]) -> HDKey {
        let mut node = self.master_hdkey.clone();
        for &i in path {
            node = node.derive_child(i);
        }
        node
    }

    pub fn generate_n_keys(&self, n: u32) -> Vec<HDKey> {
        let mut keys = Vec::with_capacity(n as usize);
        for i in 0..n {
            let mut full_path = BASE_PATH.to_vec();
            full_path.push(i);
            let child_hdkey = self.derive_path(&full_path);
            keys.push(child_hdkey);
        }
        keys
    }

    pub fn owns_address(&self, address: &VerifyingKey) -> Option<u32> {
        // TODO: rethink this limit
        let limit = self.current_index + 100;
        for i in 0..limit {
            let mut full_path = BASE_PATH.to_vec();
            full_path.push(i);
            let candidate = self.derive_path(&full_path);
            if &candidate.get_public_key() == address {
                return Some(i);
            }
        }
        None
    }

    pub fn get_receive_addr(&mut self) -> VerifyingKey {
        // TODO: get a new receive address that is not already used
        let mut path = BASE_PATH.to_vec();
        path.push(self.current_index);
        let child_hdkey = self.derive_path(&path);
        self.current_index += 1;
        child_hdkey.get_public_key()
    }

    pub fn send_tx(
        &self,
        src_pk: VerifyingKey,
        dest_pk: VerifyingKey,
        amount: f64,
        message: Option<String>,
    ) -> Result<Transaction, &'static str> {
        let addr_index = self.owns_address(&src_pk);
        if addr_index.is_none() {
            return Err("Endereço de origem não pertence a esta carteira");
        }

        let mut full_path = BASE_PATH.to_vec();
        full_path.push(addr_index.unwrap());
        let child_hdkey = self.derive_path(&full_path);

        let mut tx = Transaction::new(
            amount,
            public_key_to_hex(&dest_pk),
            public_key_to_hex(&child_hdkey.get_public_key()),
            message,
        );
        let signature = sign_hash(
            &mut SigningKey::from_bytes(&child_hdkey.private_key),
            tx.to_string().as_bytes(),
        );
        tx.signature = hex::encode(signature.to_bytes());

        Ok(tx)
    }

    // TODO: the ideia here is fetch the origin address with balance from the node
    // pub fn send_payment(&self, dest_pk: VerifyingKey, amount: f64) {
    //     let node = NODE.lock().unwrap();
    //     // let origin_addr = node.
    // }
}
