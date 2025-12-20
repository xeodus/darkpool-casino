use aes_gcm::Nonce;
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use anyhow::anyhow;
use anyhow::Result;
use base64::Engine;
use base64::engine::general_purpose;
use solana_sdk::signature::Keypair;

pub struct KeyManager {
    pub cipher: Aes256Gcm
}

impl KeyManager {
    pub fn new(encryption_key: &str) -> Result<Self> {
        let key_bytes = hex::decode(encryption_key)
            .or_else(
                |_| {
                    if encryption_key.len() >= 32 {
                        Ok::<Vec<u8>, anyhow::Error>(encryption_key.as_bytes()[..32].to_vec())
                    }
                    else {
                        let mut padded = encryption_key.as_bytes().to_vec();
                        padded.resize(32, 0);
                        Ok(padded)
                    }
                }
        )?;

        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|e| anyhow!("Failed to generate cipher: {}", e))?;

        Ok(KeyManager { cipher })
    }

    pub fn generate_keypair(&self) -> Result<Keypair> {
        let keypair = Keypair::new();
        Ok(keypair)
    }

    pub async fn encrypt_keypair(&self, keypair: &Keypair) -> Result<String> {
        let keypair_bytes = keypair.to_bytes();
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from(nonce_bytes);
        let cipher_text = self.cipher.encrypt(&nonce, keypair_bytes.as_ref())
            .map_err(|e| anyhow!("Failed to generate cipher text while encrypting the keypair: {}", e))?;

        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&cipher_text);

        Ok(general_purpose::STANDARD.encode(result))
    }

    pub fn decrypt_keypair(&self, encrypted_key: &str) -> Result<Keypair> {
        let data = general_purpose::STANDARD.decode(encrypted_key)
            .map_err(|e| anyhow!("Base64 decode failed: {}", e))?;

        if data.len() < 12 {
            return Err(anyhow!("Invalid encrypted data.."));
        }

        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce_arr: [u8; 12] = nonce_bytes.try_into()?;
        let nonce = nonce_arr.into();
        let plaintext = self.cipher.decrypt(&nonce, ciphertext)
            .map_err(|e| anyhow!("failed to convert ciphertext into plaintext: {}", e))?;

        if plaintext.len() != 64 {
            return Err(anyhow!("Invalid plaintext length.."));
        }

        let bytes = plaintext.as_slice();
        let keypair = Keypair::try_from(bytes).map_err(|e| anyhow!("Failed to decrypt keypair: {}", e))?;

        Ok(keypair)
    }
}
