use crate::bd::Db;
use crate::model::get_node;
// use crate::globals::NODE;
use crate::model::io::UTXO;
use crate::{
    model::{HDKey, Transaction, TxInput, TxOutput},
    security_utils::public_key_to_hex,
};

#[derive(Clone)]
pub struct Wallet {
    master_hdkey: HDKey,
    current_index: u32,
}

const GAP_LIMIT: u32 = 20;

/** purpose / account / change / index */
const BASE_PATH: [u32; 4] = [111, 0, 0, 0];
impl Wallet {
    pub fn new(seed: &str) -> Self {
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

    pub fn generate_n_keys(&self, n: u32, offset: Option<u32>) -> Vec<HDKey> {
        let mut keys = Vec::with_capacity(n as usize);
        for i in 0..n {
            let mut full_path = BASE_PATH.to_vec();
            full_path.push(i + offset.unwrap_or(0));
            let child_hdkey = self.derive_path(&full_path);
            keys.push(child_hdkey);
        }
        keys
    }

    /** Returns the index of the address if owned by the wallet, otherwise None */
    pub fn owns_address(&self, address: &str) -> Option<u32> {
        let mut keys = self.generate_n_keys(GAP_LIMIT, None);
        let db = Db::open(None);
        if db.is_err() {
            panic!("Failed to open database at wallet address ownership check");
        }
        let mut gap_count = 1;
        loop {
            let index = keys.iter().position(|k| k.get_address() == address);
            if let Some(i) = index {
                return Some((i as u32) + GAP_LIMIT * (gap_count - 1));
            }
            let addresses = keys
                .iter()
                .map(|k| k.get_address())
                .collect::<Vec<String>>();
            let any_address_in_bc = db.as_ref().unwrap().has_any_address_been_used(&addresses);
            if any_address_in_bc.is_err() || !any_address_in_bc.unwrap() {
                break;
            }
            keys = self.generate_n_keys(GAP_LIMIT, Some(GAP_LIMIT * gap_count));
            gap_count += 1;
        }
        None
    }

    pub fn get_receive_addr(&mut self) -> String {
        let mut path = BASE_PATH.to_vec();
        path.push(self.current_index);
        let child_hdkey = self.derive_path(&path);
        self.current_index += 1;
        child_hdkey.get_address()
    }

    pub fn get_change_addr(&mut self) -> String {
        let mut path = BASE_PATH.to_vec();
        path[2] = 1; // change
        path.push(self.current_index);
        let child_hdkey = self.derive_path(&path);
        self.current_index += 1;
        child_hdkey.get_address()
    }

    pub fn get_wallet_utxos(&self) -> Vec<UTXO> {
        let node = get_node();
        let utxos = node.scan_utxos();
        let wallet_utxos = utxos
            .into_iter()
            .filter(|u| self.owns_address(&u.output.address).is_some())
            .collect::<Vec<UTXO>>();
        wallet_utxos
    }

    pub fn select_utxos(&self, amount: f64) -> Option<Vec<UTXO>> {
        let utxos = self.get_wallet_utxos();
        let mut selected = Vec::new();
        let mut total = 0.0;
        for utxo in utxos {
            total += utxo.output.value;
            selected.push(utxo);
            if total >= amount {
                return Some(selected);
            }
        }
        None
    }

    pub fn calculate_balance(&self) -> f64 {
        let utxos = self.get_wallet_utxos();
        utxos.iter().map(|u| u.output.value).sum()
    }

    pub fn send_tx(
        &self,
        outputs: Vec<TxOutput>,
        message: Option<String>,
    ) -> Result<Transaction, &'static str> {
        let is_outputs_valid = outputs
            .iter()
            .map(|o| &o.address)
            .all(|addr| HDKey::validate_address(&addr));
        if !is_outputs_valid {
            return Err("One or more output addresses are invalid");
        }

        let total_needed: f64 = outputs.iter().map(|o| o.value).sum();
        let utxos_to_spend = self.select_utxos(total_needed);

        if utxos_to_spend.is_none() {
            return Err("Insufficient funds");
        }

        let mut inputs = Vec::new();
        let node = get_node();

        for utxo in utxos_to_spend.unwrap() {
            let output_tx = node.find_transaction(&utxo.tx_id);
            if output_tx.is_none() {
                return Err("UTXO origin transaction not found");
            }

            let input = TxInput {
                prev_tx_id: utxo.tx_id,
                output_index: utxo.index,
                signature: "".to_string(), // will be signed later
                public_key: String::new(), // will be filled later
            };
            inputs.push((input, utxo.output.address, utxo.index));
        }

        let mut tx = Transaction::new(
            inputs.iter().map(|(inp, _, _)| inp.clone()).collect(),
            outputs,
            message,
        );
        let tx_bytes = &tx.as_bytes();

        for (i, (_input, addr, _index)) in inputs.into_iter().enumerate() {
            if let Some(derivation_index) = self.owns_address(&addr) {
                let mut path = BASE_PATH.to_vec();
                path.push(derivation_index);
                let child_hdkey = self.derive_path(&path);

                let sig = child_hdkey.sign(tx_bytes);

                tx.inputs[i].signature = hex::encode(sig.to_bytes());
                tx.inputs[i].public_key = public_key_to_hex(&child_hdkey.get_public_key());
            } else {
                return Err("Address not owned by wallet");
            }
        }

        Ok(tx)
    }
}
