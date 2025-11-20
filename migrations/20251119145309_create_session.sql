CREATE TABLE IF NOT EXISTS sessions (
    session_id VARCHAR(64) UNIQUE NOT NULL,
    parent_wallet VARCHAR(44) NOT NULL,
    ephemeral_wallet VARCHAR(44) NOT NULL,
    encrypted_keypair TEXT NOT NULL,
    vault_address VARCHAR(44) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE INDEX IF NOT EXISTS idx_sessions_parent_wallet ON sessions(parent_wallet);
CREATE INDEX IF NOT EXISTS idx_sessions_session_id ON sessions(session_id);
