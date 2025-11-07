use anyhow::Result;
use solana_commitment_config::CommitmentConfig;
use std::time::Duration;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::{Keypair, Signature}, transaction::Transaction};

pub struct TransactionSigner {
    pub rpc_client: RpcClient
}

impl TransactionSigner {
    pub fn new(rpc_url: &str) -> Self {
        let rpc_client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
        Self { rpc_client }
    }

    pub async fn signed_and_send_transaction(&self, transaction: &mut Transaction, signer: &Keypair) -> Result<Signature> {
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        transaction.sign(&[signer], recent_blockhash);
        let signature = self.rpc_client.send_and_confirm_transaction(transaction)?;
        Ok(signature)
    }

    pub async fn signed_and_send_with_retry(&self, transaction: &mut Transaction, signer: &Keypair, max_retry: u8) -> Result<Signature> {
        let mut attempts = 0;
        loop {
            match self.signed_and_send_transaction(transaction, signer).await {
                Ok(signature) => {
                    return Ok(signature);
                },
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_retry {
                        return Err(anyhow::anyhow!("Failed to send signed transaction: {}", e));
                    }
                    tracing::warn!("Transaction attempt failed: {}, retry attempt: {}, retrying..", e, attempts);
                    tokio::time::sleep(Duration::from_millis(500 * attempts as u64)).await;
                }
            }
        }
    }

    pub async fn confirm_transaction(&self, sign: &Signature) -> Result<bool> {
        match self.rpc_client.confirm_transaction(sign) {
            Ok(done) => Ok(done),
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to confirm transaction: {}", e));
            }
        }
    }
}