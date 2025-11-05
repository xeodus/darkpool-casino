-- Add migration script here
CREATE TABLE rate_limits (
    id BIGSERIAL PRIMARY KEY,
    user_pubkey VARCHAR(44) NOT NULL,
    endpoint VARCHAR(100) NOT NULL,
    request_count INT NOT NULL DEFAULT 1,
    window_start TIMESTAMP WITH TIME ZONE NOT NULL,
    window_end TIMESTAMP WITH TIME ZONE NOT NULL,
    
    CONSTRAINT unique_rate_limit UNIQUE (user_pubkey, endpoint, window_start),
    CONSTRAINT valid_window CHECK (window_end > window_start)
);

CREATE INDEX idx_rate_limits_user_endpoint ON rate_limits(user_pubkey, endpoint);
CREATE INDEX idx_rate_limits_window ON rate_limits(window_end);

-- Auto-cleanup expired rate limit records
CREATE INDEX idx_rate_limits_cleanup ON rate_limits(window_end) 
    WHERE window_end < CURRENT_TIMESTAMP;

COMMENT ON TABLE rate_limits IS 'Rate limiting enforcement';