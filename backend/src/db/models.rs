use chrono::{DateTime, Utc};

pub struct Sessions {
    pub id: i32,
    pub session_id: String,
    pub parent_wallet: String,
    pub ephemeral_wallet: String,
    pub encrypted_keypair: String,
    pub vault_address: String,
    pub approved_amount: i64,
    pub session_duration: i64,
    pub total_deposited: i64,
    pub total_spent: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_active: bool,
    pub last_activity: DateTime<Utc>,
}

pub struct VaultTransaction {
    pub id: i32,
    pub session_id: Option<String>,
    pub vault_address: String,
    pub transaction_type: String,
    pub amount: u64,
    pub signature: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

pub struct Delegation {
    pub id: i32,
    pub session_id: String,
    pub ephemeral_wallet: String,
    pub vault_address: String,
    pub approved_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub transaction_signature: Option<String>,
    pub is_active: bool,
}

pub struct CleanupEvent {
    pub id: i32,
    pub vault_address: String,
    pub cleanup_caller: String,
    pub returned_amount: u64,
    pub signature: Option<String>,
    pub reward_amount: u64,
    pub created_at: DateTime<Utc>,
}

pub struct SessionAnalytics {
    pub session_id: String,
    pub total_orders: i64,
    pub total_volume: i64,
    pub avg_latency_ms: i64,
    pub last_updated: DateTime<Utc>,
}
