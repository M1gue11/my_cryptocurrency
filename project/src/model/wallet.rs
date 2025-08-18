use crate::{
    model::Transaction,
    security_utils::{generate_key_pair, public_key_to_hex, sign_hash},
};
use ed25519_dalek::SigningKey;
use ed25519_dalek::VerifyingKey;

pub struct Wallet {
    private_key: SigningKey,
    pub public_key: VerifyingKey,
}

impl Wallet {
    pub fn new() -> Self {
        let (private_key, public_key) = generate_key_pair();
        Wallet {
            private_key,
            public_key,
        }
    }

    pub fn send_tx(
        &mut self,
        dest_pk: VerifyingKey,
        amount: f64,
        message: Option<String>,
    ) -> Transaction {
        let mut tx = Transaction::new(
            amount,
            public_key_to_hex(&dest_pk),
            public_key_to_hex(&self.public_key),
            message,
        );
        let signature = sign_hash(&mut self.private_key, tx.to_string().as_bytes());
        tx.signature = hex::encode(signature.to_bytes());
        tx
    }
}
