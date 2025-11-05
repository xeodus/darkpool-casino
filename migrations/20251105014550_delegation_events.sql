-- Add migration script here
CREATE TABLE delegation_events (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    
    event_type VARCHAR(20) NOT NULL,            -- 'approved', 'revoked'
    signature VARCHAR(88) NOT NULL,             -- On-chain transaction signature
    timestamp BIGINT NOT NULL,
    
    -- Additional context
    revocation_reason TEXT,                     -- Why delegation was revoked
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT valid_event_type CHECK (
        event_type IN ('approved', 'revoked')
    )
);

CREATE INDEX idx_delegation_events_session_id ON delegation_events(session_id);
CREATE INDEX idx_delegation_events_timestamp ON delegation_events(timestamp DESC);
CREATE INDEX idx_delegation_events_type ON delegation_events(event_type);

-- Ensure only one approval per session
CREATE UNIQUE INDEX idx_delegation_one_approval 
    ON delegation_events(session_id) 
    WHERE event_type = 'approved';

COMMENT ON TABLE delegation_events IS 'Delegation approval and revocation history';