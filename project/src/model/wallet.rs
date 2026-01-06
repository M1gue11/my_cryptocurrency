use crate::db::repository::LedgerRepository;
use crate::model::MempoolTx;
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

#[derive(Clone)]
pub enum DerivationType {
    Receive = 0,
    Change = 1,
}

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

    fn get_base_path(&self, d_type: &DerivationType) -> Vec<u32> {
        let mut base = BASE_PATH.to_vec();
        base[2] = d_type.clone() as u32;
        base
    }

    pub fn derive_path(&self, path: &[u32]) -> HDKey {
        let mut node = self.master_hdkey.clone();
        for &i in path {
            node = node.derive_child(i);
        }
        node
    }

    pub fn generate_n_keys(
        &self,
        n: u32,
        offset: Option<u32>,
        d_type: Option<DerivationType>,
    ) -> Vec<HDKey> {
        let derivation_type = d_type.unwrap_or(DerivationType::Receive);
        let mut keys = Vec::with_capacity(n as usize);
        let start_index = offset.unwrap_or(0);
        for i in 0..n {
            let mut full_path = self.get_base_path(&derivation_type);
            full_path.push(i + start_index);
            let child_hdkey = self.derive_path(&full_path);
            keys.push(child_hdkey);
        }
        keys
    }

    fn generate_used_keys_for_type(&self, d_type: DerivationType) -> Vec<HDKey> {
        let repo = LedgerRepository::new();
        let mut gap_count = 0;
        let mut keys: Vec<HDKey> = Vec::with_capacity(GAP_LIMIT as usize);

        loop {
            let mut batch = Vec::new();
            for i in 0..GAP_LIMIT {
                let mut path = self.get_base_path(&d_type);
                path.push(i + GAP_LIMIT * gap_count);
                batch.push(self.derive_path(&path));
            }

            let addresses: Vec<String> = batch.iter().map(|k| k.get_address()).collect();
            let any_address_in_bc = repo.has_any_address_been_used(&addresses);

            // If no address was used, stop iterating
            if any_address_in_bc.is_err() || !any_address_in_bc.unwrap() {
                break;
            }

            keys.extend(batch);
            gap_count += 1;
        }

        keys
    }

    /// Generates all keys (both receive and change) derived from used addresses using the gap limit strategy.
    fn generate_all_used_keys(&self) -> Vec<HDKey> {
        let mut keys = self.generate_used_keys_for_type(DerivationType::Receive);
        keys.extend(self.generate_used_keys_for_type(DerivationType::Change));
        keys
    }

    /// Checks if the wallet owns the given address using the gap limit strategy.
    /// # Returns
    /// * `Some(u32)` - The derivation index if the wallet owns this address
    /// * `None` - If the address is not owned by this wallet
    pub fn owns_address(&self, address: &str) -> Option<u32> {
        let keys = self.generate_all_used_keys();
        keys.iter()
            .position(|k| k.get_address() == address)
            .map(|i| i as u32)
    }

    /// Lists all keys derived from addresses that have been used in the blockchain.
    /// # Returns
    /// A vector of HDKey objects for all addresses detected as used in the blockchain
    fn list_used_gaps(&self) -> Vec<HDKey> {
        self.generate_all_used_keys()
    }

    /// Gets a new receiving address from the wallet, incrementing the current index.
    /// # Returns
    /// A new receiving address as a String
    pub fn get_receive_addr(&mut self) -> String {
        let mut path = BASE_PATH.to_vec();
        path.push(self.current_index);
        let child_hdkey = self.derive_path(&path);
        self.current_index += 1;
        child_hdkey.get_address()
    }

    pub fn get_curr_addr(&self) -> String {
        let mut path = BASE_PATH.to_vec();
        path.push(self.current_index);
        let child_hdkey = self.derive_path(&path);
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
        let repo = LedgerRepository::new();
        let keys = self.list_used_gaps();
        let addresses = keys
            .iter()
            .map(|k| k.get_address())
            .collect::<Vec<String>>();
        let utxos = repo.get_utxos_for_addresses(&addresses).unwrap_or_default();
        utxos
    }

    /// Selects UTXOs from the wallet to cover the specified amount.
    ///
    /// This function implements a greedy coin selection algorithm:
    /// 1. Retrieves all available UTXOs from the wallet
    /// 2. Sorts UTXOs in descending order by value (largest first)
    /// 3. Accumulates UTXOs until the total meets or exceeds the required amount
    ///
    /// # Arguments
    /// * `amount` - The minimum value required to be covered by selected UTXOs
    ///
    /// # Returns
    /// * `Some(Vec<UTXO>)` - A vector of selected UTXOs if sufficient funds are available
    /// * `None` - If the wallet doesn't have enough funds to cover the amount
    pub fn select_utxos(&self, amount: f64) -> Option<Vec<UTXO>> {
        let mut utxos = self.get_wallet_utxos();
        // Sort UTXOs in descending order by value (largest first)
        utxos.sort_by(|a, b| {
            b.output
                .value
                .partial_cmp(&a.output.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut selected = Vec::new();
        let mut total = 0.0;
        for utxo in utxos {
            total += utxo.output.value;
            selected.push(utxo);
            if total >= amount {
                return Some(selected);
            }
        }
        // Insufficient funds
        None
    }

    pub fn calculate_balance(&self) -> f64 {
        let utxos = self.get_wallet_utxos();
        utxos.iter().map(|u| u.output.value).sum()
    }

    pub fn send_tx(
        &mut self,
        mut outputs: Vec<TxOutput>,
        fee: Option<f64>,
        message: Option<String>,
    ) -> Result<MempoolTx, &'static str> {
        // Validate output addresses
        let is_outputs_valid = outputs
            .iter()
            .map(|o| &o.address)
            .all(|addr| HDKey::validate_address(&addr));
        if !is_outputs_valid {
            return Err("One or more output addresses are invalid");
        }

        // Calculate total needed and select UTXOs
        let total_needed: f64 = outputs.iter().map(|o| o.value).sum::<f64>() + fee.unwrap_or(0.0);
        let utxos_to_spend = match self.select_utxos(total_needed) {
            Some(ref utxos) => utxos.clone(),
            None => return Err("Insufficient funds"),
        };

        // Calculate change and add change output if necessary
        let change = utxos_to_spend.iter().map(|u| u.output.value).sum::<f64>() - total_needed;
        if change > 0.0 {
            let change_address = self.get_change_addr();
            let change_output = TxOutput {
                address: change_address,
                value: change,
            };
            outputs.push(change_output);
        }

        // Create inputs from selected UTXOs
        let mut inputs = Vec::new();
        for utxo in &utxos_to_spend {
            // Use a reference to avoid moving
            let input = TxInput {
                prev_tx_id: utxo.tx_id,
                output_index: utxo.index,
                signature: "".to_string(), // will be signed later
                public_key: String::new(), // will be filled later
            };
            inputs.push((input, utxo.output.address.clone(), utxo.index));
        }

        // Create the transaction and sign inputs
        let mut mem_tx = MempoolTx::new(
            Transaction::new(
                inputs.iter().map(|(inp, _, _)| inp.clone()).collect(),
                outputs,
                message,
            ),
            utxos_to_spend,
        );
        let tx_bytes = &mem_tx.tx.as_bytes();
        for (i, (_, addr, _)) in inputs.into_iter().enumerate() {
            if let Some(derivation_index) = self.owns_address(&addr) {
                let mut path = BASE_PATH.to_vec();
                path.push(derivation_index);
                let child_hdkey = self.derive_path(&path);
                let sig = child_hdkey.sign(tx_bytes);
                mem_tx.tx.inputs[i].signature = hex::encode(sig.to_bytes());
                mem_tx.tx.inputs[i].public_key = public_key_to_hex(&child_hdkey.get_public_key());
            } else {
                return Err("Address not owned by wallet");
            }
        }

        Ok(mem_tx)
    }
}
