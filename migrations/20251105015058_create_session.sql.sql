-- Add migration script here
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    
    -- Wallet identifiers
    user_pubkey VARCHAR(44) NOT NULL,           -- Parent wallet (main user wallet)
    ephemeral_pubkey VARCHAR(44) NOT NULL,      -- Temporary trading wallet
    vault_pubkey VARCHAR(44) NOT NULL,          -- On-chain vault PDA
    
    -- Session timing
    created_at BIGINT NOT NULL,                 -- Unix timestamp
    expires_at BIGINT NOT NULL,                 -- Session expiration
    is_active BOOLEAN NOT NULL DEFAULT true,
    
    -- Financial tracking
    total_deposited BIGINT NOT NULL DEFAULT 0,  -- Total SOL deposited (lamports)
    total_spent BIGINT NOT NULL DEFAULT 0,      -- Total SOL spent (lamports)
    max_spending_limit BIGINT NOT NULL,         -- Maximum allowed spending
    
    -- Metadata
    ip_address INET,                            -- User IP for security
    user_agent TEXT,                            -- Browser/client info
    
    -- Timestamps
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    CONSTRAINT unique_active_vault UNIQUE (vault_pubkey, is_active),
    CONSTRAINT valid_expiry CHECK (expires_at > created_at),
    CONSTRAINT valid_spending CHECK (total_spent <= total_deposited)
);

-- Performance indexes
CREATE INDEX idx_sessions_user_pubkey ON sessions(user_pubkey);
CREATE INDEX idx_sessions_ephemeral_pubkey ON sessions(ephemeral_pubkey);
CREATE INDEX idx_sessions_vault_pubkey ON sessions(vault_pubkey);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at) WHERE is_active = true;
CREATE INDEX idx_sessions_created_at ON sessions(created_at DESC);
CREATE INDEX idx_sessions_active ON sessions(is_active, expires_at);

-- Partial index for expired session cleanup
CREATE INDEX idx_sessions_expired_cleanup ON sessions(expires_at) 
    WHERE is_active = true AND expires_at < EXTRACT(EPOCH FROM CURRENT_TIMESTAMP);

COMMENT ON TABLE sessions IS 'Active ephemeral wallet sessions with expiry tracking';
COMMENT ON COLUMN sessions.user_pubkey IS 'Main wallet that owns the session';
COMMENT ON COLUMN sessions.ephemeral_pubkey IS 'Temporary wallet for gasless trading';
COMMENT ON COLUMN sessions.vault_pubkey IS 'On-chain PDA vault address';