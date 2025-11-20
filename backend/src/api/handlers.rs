use std::{env, str::FromStr};
use anyhow::Result;
use base64::prelude::{BASE64_STANDARD, Engine};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    message::Instruction, pubkey::Pubkey, signer::Signer, transaction::Transaction
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

#[derive(Debug, Serialize)]
pub struct EncodedInstruction {
    pub program_id: String,
    pub accounts: Vec<EncodedAccountMeta>,
    pub data: String
}

#[derive(Debug, Serialize)]
pub struct EncodedAccountMeta {
    pub pubkey: String,
    pub is_signed: bool,
    pub is_writable: bool
}

pub fn encoded_instruction(ix: &Instruction) -> EncodedInstruction {
    EncodedInstruction {
        program_id: ix.program_id.to_string(),
        accounts: ix.accounts.iter().map(|meta| EncodedAccountMeta {
            pubkey: meta.pubkey.to_string(),
            is_signed: meta.is_signer,
            is_writable: meta.is_writable
        })
        .collect(),
        data: BASE64_STANDARD.encode(&ix.data)
    }
}

// Create session request and expect response

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub parent_wallet: String,
    pub approved_amount: u64,
    pub session_duration: i64,
    pub expected_transaction: Option<i64>
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub parent_wallet: String,
    pub ephemeral_wallet: String,
    pub vault_address: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub suggested_deposit: u64,
    pub instructions: Vec<EncodedInstruction>
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> 
{
    let parent_wallet = Pubkey::from_str(&payload.parent_wallet)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid parent wallet".to_string()))?;

    if payload.approved_amount == 0 {
        return Err((StatusCode::BAD_REQUEST, "Invalid amount, can't be approved".to_string()));
    }

    if payload.session_duration <= 0 {
        return Err((StatusCode::BAD_REQUEST, "Invalid session duration".to_string()));
    }

    let (vault_address, _) = state.delegation_manager.derive_vault_pda();

    let session = state.session_manager
        .create_sessions(&parent_wallet.to_string(), &vault_address.to_string(), payload.session_duration)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create table in the database: {}", e);
            (StatusCode::BAD_REQUEST, format!("Failed to create new session: {}", e))
        })?; 

    let suggested_deposit = payload.expected_transaction
        .map(|expected| state.deposit_calculator.calculate_deposit(expected as u64))
        .unwrap_or_else(|| state.deposit_calculator.calculate_default_deposit());

    let create_ix = state.delegation_manager
        .build_deposit_ix(vault_address, payload.approved_amount)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create delegation table in the database: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal server error: {}", e))
        })?;


    Ok(Json(CreateSessionResponse {
        session_id: session.session_id,
        parent_wallet: parent_wallet.to_string(),
        ephemeral_wallet: session.ephemeral_wallet,
        vault_address: vault_address.to_string(),
        created_at: session.created_at,
        expires_at: session.expires_at,
        suggested_deposit,
        instructions: vec![encoded_instruction(&create_ix)]
    }))
}

// Approve delegation request and expect response

#[derive(Debug, Deserialize)]
pub struct ApproveDelegationRequest {
    pub session_id: String,
    pub approved_amount: u64
}

#[derive(Debug, Serialize)]
pub struct ApproveDelegationResponse {
    pub success: bool,
    pub parent_wallet: String,
    pub ephemeral_wallet: String,
    pub approved_amount: u64,
    pub created_at: i64, 
    pub message: String,
    pub instructions: EncodedInstruction
}

pub async fn approve_delegation(
    State(state): State<AppState>,
    Json(payload): Json<ApproveDelegationRequest>
) -> Result<Json<ApproveDelegationResponse>, (StatusCode, String)>

{
    let session = state.session_manager
        .get_session(&payload.session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Session id not found".to_string()))?;

    let (vault_address, _) = state.delegation_manager.derive_vault_pda();

    let create_ix = state.delegation_manager
        .build_deposit_ix(vault_address, payload.approved_amount)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid server error!".to_string()))?;

    Ok(Json(ApproveDelegationResponse {
        success: true,
        parent_wallet: session.parent_wallet,
        ephemeral_wallet: session.ephemeral_wallet,
        created_at: session.created_at,
        message: format!("Delegation approved for ephemeral wallet for: {}", session.session_id),
        approved_amount: payload.approved_amount, 
        instructions: encoded_instruction(&create_ix)
    }))
}

// Create revoke request and expect response

#[derive(Debug, Deserialize)]
pub struct RevokeAccessRequest {
    pub session_id: String
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
    let keypair = state.key_manager.decrypt_keypair(&encrypted_keypair)
        .map_err(|e| (StatusCode::NOT_ACCEPTABLE, e.to_string()))?;

    let vault_address = Pubkey::from_str(&session.vault_address)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Cannot parse vault address".to_string()))?;

    let ix = state.delegation_manager.build_deposit_ix(vault_address, payload.amount)
    .await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut tx = Transaction::new_with_payer(&[ix], Some(&keypair.pubkey()));
    let sign = state.transaction_signer.signed_and_send_with_retry(&mut tx, &keypair, 5).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(TransactionSignatureResponse {
        success: true,
        signature: sign.to_string()
    }))
}

// Create trigger deposit request and expect response
#[derive(Debug, Deserialize)]
pub struct TriggerDepositRequest {
    pub sesssion_id: String,
    pub amount: Option<u64>,
    pub trading_fee: u64
}

#[derive(Debug, Serialize)]
pub struct TriggerDepositResponse {
    pub success: bool,
    pub amount: u64,
    pub vault_address: String,
    pub instructions: EncodedInstruction,
}

pub async fn trigger_deposit(
    State(state): State<AppState>,
    Json(payload): Json<TriggerDepositRequest>
) -> Result<Json<TriggerDepositResponse>, (StatusCode, String)> 
{
    let session = state.session_manager.get_session(&payload.sesssion_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Session id not found".to_string()))?;

    let amount = payload.amount
        .filter(|val| *val > 0)
        .unwrap_or_else(|| state.deposit_calculator.calculate_default_deposit());

    let vault_addr = Pubkey::from_str(&session.vault_address)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch the pubkey".to_string()))?;

    let create_ix = state.delegation_manager
        .build_deposit_ix(vault_addr, payload.trading_fee)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()))?;

    Ok(Json(TriggerDepositResponse {
        success: true,
        amount,
        instructions: encoded_instruction(&create_ix),
        vault_address: session.vault_address
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
