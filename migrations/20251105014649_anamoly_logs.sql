-- Add migration script here
CREATE TABLE anomaly_logs (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID REFERENCES sessions(id) ON DELETE SET NULL,
    user_pubkey VARCHAR(44) NOT NULL,
    
    -- Anomaly details
    anomaly_type VARCHAR(50) NOT NULL,          -- 'excessive_spending', 'rapid_trades', etc.
    severity VARCHAR(20) NOT NULL,              -- 'low', 'medium', 'high', 'critical'
    description TEXT,
    metadata JSONB,                             -- Additional anomaly data
    
    -- Response
    action_taken VARCHAR(50),                   -- 'none', 'rate_limited', 'alerted', 'suspended'
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    CONSTRAINT valid_severity CHECK (
        severity IN ('low', 'medium', 'high', 'critical')
    ),
    CONSTRAINT valid_action CHECK (
        action_taken IN ('none', 'rate_limited', 'session_suspended', 'alerted', 'blocked')
    )
);

CREATE INDEX idx_anomaly_logs_session_id ON anomaly_logs(session_id);
CREATE INDEX idx_anomaly_logs_user_pubkey ON anomaly_logs(user_pubkey);
CREATE INDEX idx_anomaly_logs_created_at ON anomaly_logs(created_at DESC);
CREATE INDEX idx_anomaly_logs_severity ON anomaly_logs(severity);
CREATE INDEX idx_anomaly_logs_type ON anomaly_logs(anomaly_type);

COMMENT ON TABLE anomaly_logs IS 'Security anomaly detection and response log';