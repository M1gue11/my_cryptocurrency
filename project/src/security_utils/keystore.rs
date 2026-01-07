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

#[derive(Serialize, Deserialize, Debug)]
pub struct Keystore {
    pub salt: String,       // Hex: Salt to prevent Rainbow Tables on password
    pub nonce: String,      // Hex: Unique number for AES encryption
    pub ciphertext: String, // Hex: The encrypted seed
}

pub type Seed = [u8; 32];

impl Keystore {
    /// Creates a new seed, encrypts it with the password, and saves it to a file.
    /// Returns the plaintext seed on success.
    pub fn new_seed(password: &str, file_path: &str) -> Result<Seed, String> {
        // 1. Gerar Entropia (A Seed Real - 32 Bytes)
        let mut seed = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut seed);

        // 2. Preparar Criptografia
        // Gerar um Salt aleatório (para fortalecer a senha)
        let mut salt = [0u8; 16];
        rng.fill_bytes(&mut salt);

        // Derivar uma chave forte a partir da senha + salt (PBKDF2)
        // Isso impede que senhas fracas sejam quebradas instantaneamente
        let mut key = [0u8; 32]; // AES-256 precisa de 32 bytes
        match pbkdf2::<Hmac<Sha256>>(
            password.as_bytes(),
            &salt,
            600_000, // Tem que ser o mesmo número de iterações da criação
            &mut key,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e.to_string()),
        }

        // 3. Criptografar (AES-GCM)
        let cipher = Aes256Gcm::new(&key.into());
        let mut nonce_bytes = [0u8; 12]; // Nonce padrão do GCM é 96-bits (12 bytes)
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt retorna o texto cifrado + tag de autenticação
        let ciphertext = cipher
            .encrypt(nonce, seed.as_ref())
            .map_err(|e| format!("Erro na criptografia: {}", e))?;

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

        let mut file =
            File::create(file_path).map_err(|e| format!("Erro ao criar arquivo: {}", e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| format!("Erro ao escrever arquivo: {}", e))?;
        Ok(())
    }

    pub fn load_from_file(password: &str, file_path: &str) -> Result<Seed, String> {
        // 1. Ler o arquivo
        let mut file = File::open(file_path).map_err(|_| "Arquivo não encontrado".to_string())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|_| "Falha ao ler arquivo".to_string())?;

        // 2. Parse do JSON
        let keystore: Keystore = serde_json::from_str(&contents)
            .map_err(|_| "Formato de arquivo inválido".to_string())?;

        // 3. Decodificar Hex para Bytes
        let salt = hex::decode(&keystore.salt).map_err(|_| "Salt inválido")?;
        let nonce_bytes = hex::decode(&keystore.nonce).map_err(|_| "Nonce inválido")?;
        let ciphertext = hex::decode(&keystore.ciphertext).map_err(|_| "Ciphertext inválido")?;

        // 4. Recriar a chave a partir da senha fornecida
        let mut key = [0u8; 32];
        match pbkdf2::<Hmac<Sha256>>(
            password.as_bytes(),
            &salt,
            600_000, // Tem que ser o mesmo número de iterações da criação
            &mut key,
        ) {
            Ok(_) => {}
            Err(e) => return Err(e.to_string()),
        }

        // 5. Descriptografar
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
