use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use hmac::Hmac;
use pbkdf2::pbkdf2;
use rand::{RngCore, thread_rng};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fs::File;
use std::io::{Read, Write};

use crate::utils;

#[derive(Serialize, Deserialize, Debug)]
pub struct Keystore {
    pub salt: String,       // Hex: Salt to prevent Rainbow Tables on password
    pub nonce: String,      // Hex: Unique number for AES encryption
    pub ciphertext: String, // Hex: The encrypted seed
}

pub type Seed = [u8; 32];
const PBKDF2_ITERATIONS: u32 = 1; //TODO: this should be 600_000 for production use

impl Keystore {
    /// Creates a new seed, encrypts it with the password, and saves it to a file.
    /// Returns the plaintext seed on success.
    pub fn new_seed(password: &str, file_path: &str) -> Result<Seed, String> {
        // generate random seed
        let mut seed = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut seed);

        // generate a random Salt
        let mut salt = [0u8; 16];
        rng.fill_bytes(&mut salt);

        // derive a strong key from the password + salt
        let mut key = [0u8; 32]; // AES-256 requires 32 bytes
        match pbkdf2::<Hmac<Sha256>>(
            password.as_bytes(),
            &salt,
            // must be the same number of iterations as creation
            PBKDF2_ITERATIONS,
            &mut key,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e.to_string()),
        }

        // encrypt
        let cipher = Aes256Gcm::new(&key.into());
        let mut nonce_bytes = [0u8; 12]; // standard GCM Nonce is 96-bits (12 bytes)
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, seed.as_ref())
            .map_err(|e| format!("Encryption error: {}", e))?;

        let keystore = Keystore {
            salt: hex::encode(salt),
            nonce: hex::encode(nonce_bytes),
            ciphertext: hex::encode(ciphertext),
        };
        Keystore::save_to_file(&keystore, file_path)?;
        Ok(seed)
    }

    pub fn save_to_file(keystore: &Keystore, file_path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&keystore)
            .map_err(|e| format!("Erro ao serializar JSON: {}", e))?;

        utils::assert_parent_dir_exists(&file_path)
            .expect("Failed to create parent directories for blockchain file");
        let mut file =
            File::create(file_path).map_err(|e| format!("Erro ao criar arquivo: {}", e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Erro ao escrever arquivo: {}", e))?;
        Ok(())
    }

    pub fn load_from_file(file_path: &str) -> Result<Self, String> {
        let mut file = File::open(file_path).map_err(|_| "Arquivo não encontrado".to_string())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|_| "Falha ao ler arquivo".to_string())?;

        let keystore: Keystore = serde_json::from_str(&contents)
            .map_err(|_| "Formato de arquivo inválido".to_string())?;
        Ok(keystore)
    }

    pub fn decrypt_seed(&self, password: &str) -> Result<Seed, String> {
        let salt = hex::decode(&self.salt).map_err(|_| "Salt inválido")?;
        let nonce_bytes = hex::decode(&self.nonce).map_err(|_| "Nonce inválido")?;
        let ciphertext = hex::decode(&self.ciphertext).map_err(|_| "Ciphertext inválido")?;

        let mut key = [0u8; 32];
        match pbkdf2::<Hmac<Sha256>>(password.as_bytes(), &salt, PBKDF2_ITERATIONS, &mut key) {
            Ok(_) => {}
            Err(e) => return Err(e.to_string()),
        }

        // decrypt
        let cipher = Aes256Gcm::new(&key.into());
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext_seed = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| "Senha incorreta ou arquivo corrompido")?;
        if plaintext_seed.len() != 32 {
            return Err("Seed descriptografada tem tamanho inválido".to_string());
        }
        Ok(plaintext_seed.try_into().unwrap())
    }
}
