-- Add migration script here
CREATE TABLE system_config (
    key VARCHAR(100) PRIMARY KEY,
    value TEXT NOT NULL,
    value_type VARCHAR(20) DEFAULT 'string',    -- 'string', 'integer', 'boolean', 'json'
    description TEXT,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_by VARCHAR(100),
    
    CONSTRAINT valid_value_type CHECK (
        value_type IN ('string', 'integer', 'boolean', 'json')
    )
);

-- Insert default configurations
INSERT INTO system_config (key, value, value_type, description) VALUES
    ('max_session_duration', '86400', 'integer', 'Maximum session duration in seconds (24 hours)'),
    ('min_session_duration', '300', 'integer', 'Minimum session duration in seconds (5 minutes)'),
    ('max_spending_limit', '1000000000', 'integer', 'Maximum spending limit per session (1 SOL in lamports)'),
    ('rate_limit_sessions_per_hour', '5', 'integer', 'Max sessions per user per hour'),
    ('cleanup_reward_percentage', '1', 'integer', 'Percentage reward for cleanup callers'),
    ('auto_cleanup_interval', '300', 'integer', 'Cleanup worker interval in seconds'),
    ('min_vault_balance_alert', '5000000', 'integer', 'Alert threshold for low vault balance (lamports)'),
    ('max_vault_balance_alert', '100000000', 'integer', 'Alert threshold for high vault balance (lamports)')
ON CONFLICT (key) DO NOTHING;

COMMENT ON TABLE system_config IS 'Runtime configuration parameters';