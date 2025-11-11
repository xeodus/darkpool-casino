use std::{env, str::FromStr};
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
use solana_program::example_mocks::solana_sdk::system_program;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::Signer,
    transaction::Transaction,
};
use axum::{Json, extract::{Path, State}, http::StatusCode};
use crate::api::AppState;

// Checking connection health
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String
}

pub async fn check_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "Healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string()
    })
}

// Create session request and expect response

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub parent_wallet: String,
    pub vault_address: String,
    pub timestamp: i64
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub ephemeral_wallet: String,
    pub vault_address: String,
    pub expires_at: String,
    pub suggested_deposit: u64
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> 
{
    let result = state.session_manager.create_sessions(
        &payload.parent_wallet, &payload.vault_address, payload.timestamp)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let suggested_deposit = state.deposit_calculator.calculate_default_deposit();

    Ok(Json(CreateSessionResponse {
        session_id: result.session_id,
        ephemeral_wallet: result.ephemeral_wallet,
        vault_address: result.vault_address,
        expires_at: result.expires_at.to_rfc3339(),
        suggested_deposit
    }))
}

// Approve delegation request and expect response

#[derive(Debug, Deserialize)]
pub struct ApproveDelegationRequest {
    pub session_id: String
}

#[derive(Debug, Serialize)]
pub struct ApproveDelegationResponse {
    pub success: bool,
    pub message: String
}

pub async fn approve_delegation(
    State(state): State<AppState>,
    Json(payload): Json<ApproveDelegationRequest>
) -> Result<Json<ApproveDelegationResponse>, (StatusCode, String)>

{
    let result = state.session_manager
        .get_session(&payload.session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Session id not found".to_string()))?;

    Ok(Json(ApproveDelegationResponse {
        success: true,
        message: format!("Delegation approved for ephemeral wallet for: {}", result.session_id)
    }))
}

// Create revoke request and expect response

#[derive(Debug, Deserialize)]
pub struct RevokeAccessRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct RevokeAccessResponse {
    pub success: bool,
    pub message: String
}

pub async fn revoke_access(
    State(state): State<AppState>,
    Json(payload): Json<RevokeAccessRequest>
) -> Result<Json<RevokeAccessResponse>, (StatusCode, String)>
{
    state.session_manager.deactive_session(&payload.session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(RevokeAccessResponse {
        success: true,
        message: "Successfully revoked session access".to_string()
    }))
}

pub async fn get_session_status(
    State(state): State<AppState>,
    Path(session_id): Path<String>
) -> Result<Json<serde_json::Value>, (StatusCode, String)>
{
    let session = state.session_manager.get_session(&session_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Session id not found".to_string()))?;
    Ok(Json(serde_json::to_value(session.is_active).unwrap()))
}


// Create transaction signature request and expect response

#[derive(Debug, Deserialize)]
pub struct TransactionSignatureRequest {
    pub session_id: String,
    pub amount: u64
}

#[derive(Debug, Serialize)]
pub struct TransactionSignatureResponse {
    pub success: bool,
    pub signature: String
}

pub async fn sign_and_send(
    State(state): State<AppState>,
    Json(payload): Json<TransactionSignatureRequest>
) -> Result<Json<TransactionSignatureResponse>, (StatusCode, String)>
{
    let session = state.session_manager.get_session(&payload.session_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Invalid session id".to_string()))?;

    let client = state.session_manager.pool.get().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let query = client.query_one(
        "SELECT encrypted_keypair FROM sessions WHERE session_id=$1",
        &[&payload.session_id]
    )
    .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let encrypted_keypair: String = query.get(0);
    let kp = state.key_manager.decrypt_keypair(&encrypted_keypair)
        .map_err(|e| (StatusCode::NOT_ACCEPTABLE, e.to_string()))?;

    /*let recent_blockhash = state.transaction_signer.rpc_client.get_latest_blockhash()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));*/
    let parent_wallet = Pubkey::from_str(&session.parent_wallet)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Cannot parse parent wallet addr".to_string()))?;
    let vault_address = Pubkey::from_str(&session.vault_address)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Cannot parse vault address".to_string()))?;

    let ix = build_deposit_ix(parent_wallet, vault_address, payload.amount)
    .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut tx = Transaction::new_with_payer(&[ix], Some(&kp.pubkey()));
    let sign = state.transaction_signer.signed_and_send_with_retry(&mut tx, &kp, 5).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(TransactionSignatureResponse {
        success: true,
        signature: sign.to_string()
    }))
}

#[inline]
fn program_id() -> Pubkey {
    let program_id = Pubkey::from_str(&env::var("PROGRAM_ID").expect("Program ID is not set..")).unwrap();
    program_id
}

// Derive the vault PDA your program expects: seeds = [b"vault", parent_wallet]
pub fn derive_vault_pda(user_wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault", user_wallet.as_ref()], &program_id())
}

// Build the auto_deposit instruction
pub async fn build_deposit_ix(
    user_wallet: Pubkey,  
    vault_address: Pubkey,
    trade_fee_estimate: u64,
) -> Result<Instruction> 
{
    let (expected, _) = derive_vault_pda(&user_wallet);
    if expected != vault_address {
        return Err(anyhow!("vault PDA mismatch: expected {}, got {}", expected, vault_address));
    }

    let accounts = vec![
        AccountMeta::new(user_wallet, true),
        AccountMeta::new(vault_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    
    let mut hasher = Sha256::new();
    hasher.update(b"global:auto_deposit");
    let disc = &hasher.finalize()[..8];

    let mut data = Vec::with_capacity(8 + 8);
    data.extend_from_slice(disc);
    data.extend_from_slice(&trade_fee_estimate.to_le_bytes()); // single u64 arg

    Ok(Instruction {
        program_id: program_id(),
        accounts,
        data,
    })
}

// Create trigger deposit request and expect response

#[derive(Debug, Deserialize)]
pub struct TriggerDepositRequest {
    pub sesssion_id: String,
    pub amount: u64
}

#[derive(Debug, Serialize)]
pub struct TriggerDepositResponse {
    pub success: bool,
    pub amount: u64,
    pub message: String
}

pub async fn trigger_deposit(
    State(state): State<AppState>,
    Json(payload): Json<TriggerDepositRequest>
) -> Result<Json<TriggerDepositResponse>, (StatusCode, String)> 
{
    let result = state.session_manager.get_session(&payload.sesssion_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Session id not found".to_string()))?;

    let amount = Some(payload.amount).unwrap_or_else(|| state.deposit_calculator.calculate_default_deposit());

    Ok(Json(TriggerDepositResponse {
        success: true,
        amount,
        message: format!("Deposit of {} lamports prepared for vault {}", amount, result.vault_address)
    }))
}

// Get the session stats
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub active_sessions: i64,
    pub total_capacity: i64,
    pub system_status: String,
}

pub async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>, (StatusCode, String)> {
    let active_sessions = state
        .session_manager
        .get_active_sessions_count()
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(StatsResponse {
        active_sessions,
        total_capacity: 1000,
        system_status: "operational".to_string(),
    }))
}
