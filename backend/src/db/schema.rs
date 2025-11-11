pub const CREATE_SESSIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id SERIAL PRIMARY KEY,
    session_id VARCHAR(64) UNIQUE NOT NULL,
    parent_wallet VARCHAR(44) NOT NULL,
    ephemeral_wallet VARCHAR(44) NOT NULL,
    encrypted_keypair TEXT NOT NULL,
    vault_address VARCHAR(44) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_activity TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sessions_parent_wallet ON sessions(parent_wallet);
CREATE INDEX IF NOT EXISTS idx_sessions_session_id ON sessions(session_id);
CREATE INDEX IF NOT EXISTS idx_sessions_is_active ON sessions(is_active);
"#;

pub const CREATE_VAULT_TRANSACTIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS vault_transactions (
    id SERIAL PRIMARY KEY,
    vault_address VARCHAR(44) NOT NULL,
    transaction_type VARCHAR(20) NOT NULL,
    amount BIGINT NOT NULL,
    signature VARCHAR(88),
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_vault_transactions_vault ON vault_transactions(vault_address);
CREATE INDEX IF NOT EXISTS idx_vault_transactions_type ON vault_transactions(transaction_type);
"#;

pub const CREATE_DELEGATIONS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS delegations (
    id SERIAL PRIMARY KEY,
    vault_address VARCHAR(44) NOT NULL,
    ephemeral_wallet VARCHAR(44) NOT NULL,
    approved_at TIMESTAMP NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE INDEX IF NOT EXISTS idx_delegations_vault ON delegations(vault_address);
"#;

pub const CREATE_CLEANUP_EVENTS_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS cleanup_events (
    id SERIAL PRIMARY KEY,
    vault_address VARCHAR(44) NOT NULL,
    returned_amount BIGINT NOT NULL,
    cleanup_caller VARCHAR(44) NOT NULL,
    signature VARCHAR(88),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cleanup_events_vault ON cleanup_events(vault_address);
"#;
