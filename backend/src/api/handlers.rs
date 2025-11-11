use std::str::FromStr;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey, transaction::Transaction};

use crate::api::AppState;

const DEFAULT_SESSION_DURATION_SECS: i64 = 4 * 60 * 60;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

pub async fn check_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub parent_wallet: String,
    pub approved_amount: u64,
    #[serde(default = "default_session_duration")]
    pub session_duration: i64,
    #[serde(default)]
    pub expected_transactions: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub parent_wallet: String,
    pub ephemeral_wallet: String,
    pub vault_address: String,
    pub expires_at: String,
    pub suggested_deposit: u64,
    pub instructions: Vec<EncodedInstruction>,
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(payload): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, String)> {
    let parent_wallet =
        Pubkey::from_str(&payload.parent_wallet).map_err(|_| bad_request("invalid parent wallet"))?;

    if payload.session_duration <= 0 {
        return Err(bad_request("session_duration must be positive"));
    }
    if payload.approved_amount == 0 {
        return Err(bad_request("approved_amount must be greater than zero"));
    }

    let (vault_address, _) = state.delegation_manager.derive_vault_pda(&parent_wallet);

    let session = state
        .session_manager
        .create_session(
            &payload.parent_wallet,
            &vault_address.to_string(),
            payload.approved_amount,
            payload.session_duration,
        )
        .await
        .map_err(internal)?;

    let suggested_deposit = payload
        .expected_transactions
        .map(|expected| state.deposit_calculator.calculate_deposit(expected))
        .unwrap_or_else(|| state.deposit_calculator.calculate_default_deposit());

    let create_ix = state
        .delegation_manager
        .build_create_vault_ix(
            parent_wallet,
            payload.approved_amount,
            payload.session_duration,
        )
        .map_err(internal)?;

    Ok(Json(CreateSessionResponse {
        session_id: session.session_id,
        parent_wallet: session.parent_wallet,
        ephemeral_wallet: session.ephemeral_wallet,
        vault_address: session.vault_address,
        expires_at: session.expires_at.to_rfc3339(),
        suggested_deposit,
        instructions: vec![encode_instruction(&create_ix)],
    }))
}

#[derive(Debug, Deserialize)]
pub struct ApproveDelegationRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct ApproveDelegationResponse {
    pub success: bool,
    pub message: String,
    pub instruction: EncodedInstruction,
}

pub async fn approve_delegation(
    State(state): State<AppState>,
    Json(payload): Json<ApproveDelegationRequest>,
) -> Result<Json<ApproveDelegationResponse>, (StatusCode, String)> {
    let session = state
        .session_manager
        .get_session(&payload.session_id)
        .await
        .map_err(internal)?
        .ok_or(not_found("session id not found"))?;

    let parent_wallet =
        Pubkey::from_str(&session.parent_wallet).map_err(|_| bad_request("invalid parent wallet"))?;
    let delegate =
        Pubkey::from_str(&session.ephemeral_wallet).map_err(|_| bad_request("invalid delegate"))?;

    let approve_ix = state
        .delegation_manager
        .build_approve_delegate_ix(parent_wallet, delegate)
        .map_err(internal)?;

    state
        .session_manager
        .record_delegation(
            &session.session_id,
            &session.vault_address,
            &session.ephemeral_wallet,
        )
        .await
        .map_err(internal)?;

    Ok(Json(ApproveDelegationResponse {
        success: true,
        message: format!(
            "Delegation prepared for session {} and delegate {}",
            session.session_id, session.ephemeral_wallet
        ),
        instruction: encode_instruction(&approve_ix),
    }))
}

#[derive(Debug, Deserialize)]
pub struct TriggerDepositRequest {
    pub session_id: String,
    pub amount: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct TriggerDepositResponse {
    pub instruction: EncodedInstruction,
    pub amount: u64,
    pub vault_address: String,
}

pub async fn trigger_deposit(
    State(state): State<AppState>,
    Json(payload): Json<TriggerDepositRequest>,
) -> Result<Json<TriggerDepositResponse>, (StatusCode, String)> {
    let session = state
        .session_manager
        .get_session(&payload.session_id)
        .await
        .map_err(internal)?
        .ok_or(not_found("session id not found"))?;

    let parent_wallet =
        Pubkey::from_str(&session.parent_wallet).map_err(|_| bad_request("invalid parent wallet"))?;

    let amount = payload
        .amount
        .filter(|value| *value > 0)
        .unwrap_or_else(|| state.deposit_calculator.calculate_default_deposit());

    if amount > session.approved_amount.saturating_sub(session.total_deposited) {
        return Err(bad_request("amount exceeds remaining session allowance"));
    }

    let deposit_ix = state
        .delegation_manager
        .build_auto_deposit_ix(parent_wallet, amount)
        .map_err(internal)?;

    Ok(Json(TriggerDepositResponse {
        instruction: encode_instruction(&deposit_ix),
        amount,
        vault_address: session.vault_address,
    }))
}

#[derive(Debug, Deserialize)]
pub struct TransactionSignatureRequest {
    pub session_id: String,
    pub trade_amount: u64,
    pub trading_fee: u64,
}

#[derive(Debug, Serialize)]
pub struct TransactionSignatureResponse {
    pub success: bool,
    pub signature: String,
}

pub async fn sign_and_send(
    State(state): State<AppState>,
    Json(payload): Json<TransactionSignatureRequest>,
) -> Result<Json<TransactionSignatureResponse>, (StatusCode, String)> {
    if payload.trade_amount == 0 {
        return Err(bad_request("trade_amount must be greater than zero"));
    }

    let session = state
        .session_manager
        .get_session(&payload.session_id)
        .await
        .map_err(internal)?
        .ok_or(not_found("session id not found"))?;

    let encrypted_keypair = state
        .session_manager
        .fetch_encrypted_keypair(&session.session_id)
        .await
        .map_err(internal)?
        .ok_or(not_found("no keypair stored for session"))?;

    let keypair = state
        .key_manager
        .decrypt_keypair(&encrypted_keypair)
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;

    let delegate =
        Pubkey::from_str(&session.ephemeral_wallet).map_err(|_| bad_request("invalid delegate"))?;
    let parent_wallet =
        Pubkey::from_str(&session.parent_wallet).map_err(|_| bad_request("invalid parent wallet"))?;

    let ix = state
        .delegation_manager
        .build_execute_trade_ix(
            parent_wallet,
            delegate,
            payload.trade_amount,
            payload.trading_fee,
        )
        .map_err(internal)?;

    let mut tx = Transaction::new_with_payer(&[ix], Some(&delegate));
    let signature = state
        .transaction_signer
        .signed_and_send_with_retry(&mut tx, &keypair, 5)
        .await
        .map_err(internal)?;

    state
        .session_manager
        .record_trade(
            &session.session_id,
            payload.trade_amount.saturating_add(payload.trading_fee),
        )
        .await
        .map_err(internal)?;

    Ok(Json(TransactionSignatureResponse {
        success: true,
        signature: signature.to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct RevokeAccessRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
pub struct RevokeAccessResponse {
    pub success: bool,
    pub instruction: EncodedInstruction,
}

pub async fn revoke_access(
    State(state): State<AppState>,
    Json(payload): Json<RevokeAccessRequest>,
) -> Result<Json<RevokeAccessResponse>, (StatusCode, String)> {
    let session = state
        .session_manager
        .get_session(&payload.session_id)
        .await
        .map_err(internal)?
        .ok_or(not_found("session id not found"))?;

    let parent_wallet =
        Pubkey::from_str(&session.parent_wallet).map_err(|_| bad_request("invalid parent wallet"))?;

    let revoke_ix = state
        .delegation_manager
        .build_revoke_access_ix(parent_wallet)
        .map_err(internal)?;

    state
        .session_manager
        .deactivate_session(&session.session_id)
        .await
        .map_err(internal)?;

    Ok(Json(RevokeAccessResponse {
        success: true,
        instruction: encode_instruction(&revoke_ix),
    }))
}

#[derive(Debug, Serialize)]
pub struct SessionStatusResponse {
    pub session_id: String,
    pub parent_wallet: String,
    pub ephemeral_wallet: String,
    pub vault_address: String,
    pub approved_amount: u64,
    pub total_deposited: u64,
    pub total_spent: u64,
    pub expires_at: String,
    pub last_activity: String,
    pub is_active: bool,
}

pub async fn get_session_status(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<SessionStatusResponse>, (StatusCode, String)> {
    let session = state
        .session_manager
        .get_session(&session_id)
        .await
        .map_err(internal)?
        .ok_or(not_found("session id not found"))?;

    Ok(Json(SessionStatusResponse {
        session_id: session.session_id,
        parent_wallet: session.parent_wallet,
        ephemeral_wallet: session.ephemeral_wallet,
        vault_address: session.vault_address,
        approved_amount: session.approved_amount,
        total_deposited: session.total_deposited,
        total_spent: session.total_spent,
        expires_at: session.expires_at.to_rfc3339(),
        last_activity: session.last_activity.to_rfc3339(),
        is_active: session.is_active,
    }))
}

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
        .map_err(internal)?;

    Ok(Json(StatsResponse {
        active_sessions,
        total_capacity: 1_000,
        system_status: "operational".to_string(),
    }))
}

#[derive(Debug, Serialize)]
pub struct EncodedInstruction {
    pub program_id: String,
    pub accounts: Vec<EncodedAccountMeta>,
    pub data: String,
}

#[derive(Debug, Serialize)]
pub struct EncodedAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

fn encode_instruction(ix: &Instruction) -> EncodedInstruction {
    EncodedInstruction {
        program_id: ix.program_id.to_string(),
        accounts: ix
            .accounts
            .iter()
            .map(|meta| EncodedAccountMeta {
                pubkey: meta.pubkey.to_string(),
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: BASE64.encode(&ix.data),
    }
}

fn default_session_duration() -> i64 {
    DEFAULT_SESSION_DURATION_SECS
}

fn internal<E: ToString>(error: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn not_found(message: &str) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, message.to_string())
}

fn bad_request(message: &str) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, message.to_string())
}
