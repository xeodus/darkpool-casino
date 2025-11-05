-- Add migration script here
CREATE TABLE vault_transactions (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    
    -- Transaction details
    transaction_type VARCHAR(20) NOT NULL,      -- 'deposit', 'trade', 'withdrawal', 'cleanup'
    amount BIGINT NOT NULL,                     -- Amount in lamports
    signature VARCHAR(88) NOT NULL UNIQUE,      -- Solana transaction signature
    timestamp BIGINT NOT NULL,                  -- Unix timestamp
    
    -- Status and metadata
    status VARCHAR(20) DEFAULT 'confirmed',     -- 'pending', 'confirmed', 'failed'
    error_message TEXT,                         -- Error details if failed
    metadata JSONB,                             -- Additional transaction data
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    CONSTRAINT valid_transaction_type CHECK (
        transaction_type IN ('deposit', 'trade', 'withdrawal', 'cleanup', 'refund')
    ),
    CONSTRAINT valid_status CHECK (
        status IN ('pending', 'confirmed', 'failed', 'timeout')
    )
);

-- Performance indexes
CREATE INDEX idx_vault_transactions_session_id ON vault_transactions(session_id);
CREATE INDEX idx_vault_transactions_timestamp ON vault_transactions(timestamp DESC);
CREATE INDEX idx_vault_transactions_type ON vault_transactions(transaction_type);
CREATE INDEX idx_vault_transactions_signature ON vault_transactions(signature);
CREATE INDEX idx_vault_transactions_status ON vault_transactions(status) WHERE status != 'confirmed';

COMMENT ON TABLE vault_transactions IS 'Comprehensive log of all vault transactions';
COMMENT ON COLUMN vault_transactions.transaction_type IS 'Type of transaction performed';
COMMENT ON COLUMN vault_transactions.metadata IS 'JSON metadata for extensibility';