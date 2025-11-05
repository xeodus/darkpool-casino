-- Add migration script here
CREATE TABLE vault_deposits (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    
    amount BIGINT NOT NULL,
    signature VARCHAR(88) NOT NULL UNIQUE,
    timestamp BIGINT NOT NULL,
    
    -- Deposit classification
    deposit_type VARCHAR(20) NOT NULL,          -- 'initial', 'auto_topup', 'manual'
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT valid_deposit_type CHECK (
        deposit_type IN ('initial', 'auto_topup', 'manual')
    ),
    CONSTRAINT positive_deposit CHECK (amount > 0)
);

CREATE INDEX idx_vault_deposits_session_id ON vault_deposits(session_id);
CREATE INDEX idx_vault_deposits_timestamp ON vault_deposits(timestamp DESC);
CREATE INDEX idx_vault_deposits_type ON vault_deposits(deposit_type);

COMMENT ON TABLE vault_deposits IS 'SOL deposits for transaction fees';