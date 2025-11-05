-- Add migration script here
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

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

-- ============================================================================
-- ANALYTICS: SESSION_ANALYTICS (Materialized View)
-- Aggregated session statistics for reporting
-- ============================================================================

CREATE MATERIALIZED VIEW session_analytics AS
SELECT 
    DATE_TRUNC('day', to_timestamp(created_at)) as date,
    COUNT(*) as total_sessions,
    COUNT(*) FILTER (WHERE is_active = true) as active_sessions,
    COUNT(*) FILTER (WHERE expires_at < EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)) as expired_sessions,
    AVG(total_deposited)::BIGINT as avg_deposited,
    AVG(total_spent)::BIGINT as avg_spent,
    SUM(total_deposited) as total_deposited_all,
    SUM(total_spent) as total_spent_all,
    COUNT(DISTINCT user_pubkey) as unique_users,
    
    -- Additional metrics
    MAX(total_spent) as max_spent_session,
    MIN(expires_at - created_at) as min_session_duration,
    MAX(expires_at - created_at) as max_session_duration,
    AVG(expires_at - created_at)::BIGINT as avg_session_duration
FROM sessions
GROUP BY DATE_TRUNC('day', to_timestamp(created_at))
ORDER BY date DESC;

CREATE INDEX idx_session_analytics_date ON session_analytics(date DESC);

-- Refresh function
CREATE OR REPLACE FUNCTION refresh_session_analytics()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY session_analytics;
END;
$$ LANGUAGE plpgsql;

COMMENT ON MATERIALIZED VIEW session_analytics IS 'Daily aggregated session statistics';

-- ============================================================================
-- VIEW: ACTIVE_SESSIONS_HEALTH
-- Real-time health status of active sessions
-- ============================================================================

CREATE VIEW active_sessions_health AS
SELECT 
    s.id,
    s.user_pubkey,
    s.vault_pubkey,
    s.ephemeral_pubkey,
    s.created_at,
    s.expires_at,
    (s.expires_at - EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT) as seconds_remaining,
    s.total_deposited,
    s.total_spent,
    s.max_spending_limit,
    
    -- Health metrics
    ROUND((s.total_spent::NUMERIC / NULLIF(s.max_spending_limit, 0)) * 100, 2) as spending_percentage,
    (s.total_deposited - s.total_spent) as available_balance,
    
    -- Activity metrics
    (SELECT COUNT(*) FROM trade_history WHERE session_id = s.id) as total_trades,
    (SELECT MAX(timestamp) FROM trade_history WHERE session_id = s.id) as last_trade_at,
    (SELECT COUNT(*) FROM trade_history WHERE session_id = s.id AND status = 'failed') as failed_trades,
    
    -- Health status
    CASE
        WHEN s.expires_at - EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT < 300 THEN 'EXPIRING_SOON'
        WHEN (s.total_deposited - s.total_spent) < 5000000 THEN 'LOW_BALANCE'
        WHEN s.total_spent::NUMERIC / NULLIF(s.max_spending_limit, 0) > 0.9 THEN 'HIGH_SPENDING'
        ELSE 'HEALTHY'
    END as health_status
FROM sessions s
WHERE s.is_active = true
  AND s.expires_at > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP);

COMMENT ON VIEW active_sessions_health IS 'Real-time health monitoring of active sessions';

-- ============================================================================
-- VIEW: USER_SPENDING_STATS
-- Per-user spending and activity statistics
-- ============================================================================

CREATE VIEW user_spending_stats AS
SELECT 
    user_pubkey,
    COUNT(*) as total_sessions,
    COUNT(*) FILTER (WHERE is_active = true) as active_sessions,
    SUM(total_deposited) as lifetime_deposited,
    SUM(total_spent) as lifetime_spent,
    AVG(total_spent) as avg_spent_per_session,
    MAX(total_spent) as max_spent_session,
    MIN(created_at) as first_session_at,
    MAX(created_at) as last_session_at,
    
    -- Trading activity
    (SELECT COUNT(*) 
     FROM trade_history th 
     JOIN sessions s ON th.session_id = s.id 
     WHERE s.user_pubkey = sessions.user_pubkey) as total_trades,
    
    -- Anomaly flags
    (SELECT COUNT(*) 
     FROM anomaly_logs al 
     WHERE al.user_pubkey = sessions.user_pubkey) as anomaly_count
FROM sessions
GROUP BY user_pubkey;

COMMENT ON VIEW user_spending_stats IS 'Aggregated user statistics and behavior';

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Auto-update sessions.updated_at on changes
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_sessions_updated_at
    BEFORE UPDATE ON sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Validate spending limits
CREATE OR REPLACE FUNCTION validate_spending_limit()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.total_spent > NEW.max_spending_limit THEN
        RAISE EXCEPTION 'Spending limit exceeded: % > %', NEW.total_spent, NEW.max_spending_limit;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER check_spending_limit
    BEFORE UPDATE ON sessions
    FOR EACH ROW
    WHEN (NEW.total_spent > OLD.total_spent)
    EXECUTE FUNCTION validate_spending_limit();

-- ============================================================================
-- STORED PROCEDURES
-- ============================================================================

-- Get active sessions for a user
CREATE OR REPLACE FUNCTION get_user_active_sessions(p_user_pubkey VARCHAR)
RETURNS TABLE (
    session_id UUID,
    ephemeral_pubkey VARCHAR,
    vault_pubkey VARCHAR,
    expires_at BIGINT,
    total_deposited BIGINT,
    total_spent BIGINT,
    time_remaining BIGINT,
    health_status TEXT
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        s.id,
        s.ephemeral_pubkey,
        s.vault_pubkey,
        s.expires_at,
        s.total_deposited,
        s.total_spent,
        GREATEST(0, s.expires_at - EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT) as time_remaining,
        CASE
            WHEN s.expires_at - EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT < 300 THEN 'EXPIRING_SOON'
            WHEN (s.total_deposited - s.total_spent) < 5000000 THEN 'LOW_BALANCE'
            WHEN s.total_spent::NUMERIC / NULLIF(s.max_spending_limit, 0) > 0.9 THEN 'HIGH_SPENDING'
            ELSE 'HEALTHY'
        END as health_status
    FROM sessions s
    WHERE s.user_pubkey = p_user_pubkey
      AND s.is_active = true
      AND s.expires_at > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)
    ORDER BY s.created_at DESC;
END;
$$ LANGUAGE plpgsql;

-- Get session trading summary
CREATE OR REPLACE FUNCTION get_session_trading_summary(p_session_id UUID)
RETURNS TABLE (
    total_trades BIGINT,
    total_volume BIGINT,
    total_fees BIGINT,
    avg_trade_size NUMERIC,
    first_trade_at BIGINT,
    last_trade_at BIGINT,
    success_rate NUMERIC
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        COUNT(*)::BIGINT,
        SUM(trade_amount)::BIGINT,
        SUM(trade_fee)::BIGINT,
        AVG(trade_amount),
        MIN(timestamp),
        MAX(timestamp),
        ROUND(
            (COUNT(*) FILTER (WHERE status = 'confirmed')::NUMERIC / NULLIF(COUNT(*), 0)) * 100,
            2
        ) as success_rate
    FROM trade_history
    WHERE session_id = p_session_id;
END;
$$ LANGUAGE plpgsql;

-- Batch cleanup expired sessions
CREATE OR REPLACE FUNCTION cleanup_expired_sessions_batch(batch_size INT DEFAULT 100)
RETURNS TABLE (
    session_id UUID,
    vault_pubkey VARCHAR,
    refund_eligible BOOLEAN,
    refund_amount BIGINT
) AS $$
BEGIN
    RETURN QUERY
    WITH expired AS (
        SELECT id, vault_pubkey, total_deposited, total_spent
        FROM sessions
        WHERE is_active = true
          AND expires_at < EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)
        LIMIT batch_size
        FOR UPDATE SKIP LOCKED
    )
    SELECT 
        e.id,
        e.vault_pubkey,
        (e.total_deposited > e.total_spent) as refund_eligible,
        GREATEST(0, e.total_deposited - e.total_spent) as refund_amount
    FROM expired e;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- MAINTENANCE FUNCTIONS
-- ============================================================================

-- Archive old sessions
CREATE TABLE IF NOT EXISTS sessions_archive (LIKE sessions INCLUDING ALL);

CREATE OR REPLACE FUNCTION archive_old_sessions(days_old INT DEFAULT 30)
RETURNS INTEGER AS $$
DECLARE
    archived_count INTEGER;
BEGIN
    WITH moved AS (
        DELETE FROM sessions
        WHERE is_active = false
          AND updated_at < CURRENT_TIMESTAMP - INTERVAL '1 day' * days_old
        RETURNING *
    )
    INSERT INTO sessions_archive
    SELECT * FROM moved;
    
    GET DIAGNOSTICS archived_count = ROW_COUNT;
    
    RAISE NOTICE 'Archived % old sessions', archived_count;
    RETURN archived_count;
END;
$$ LANGUAGE plpgsql;

-- Vacuum and analyze
CREATE OR REPLACE FUNCTION maintenance_vacuum()
RETURNS void AS $$
BEGIN
    VACUUM ANALYZE sessions;
    VACUUM ANALYZE vault_transactions;
    VACUUM ANALYZE trade_history;
    VACUUM ANALYZE vault_deposits;
    VACUUM ANALYZE delegation_events;
    VACUUM ANALYZE cleanup_events;
    VACUUM ANALYZE rate_limits;
    VACUUM ANALYZE anomaly_logs;
    
    RAISE NOTICE 'Vacuum completed on all tables';
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- GRANTS (Configure for your application user)
-- ============================================================================

-- Example grants (uncomment and adjust for your setup)
-- CREATE USER ephemeral_vault_app WITH PASSWORD 'secure_password';
-- GRANT CONNECT ON DATABASE ephemeral_vault TO ephemeral_vault_app;
-- GRANT USAGE ON SCHEMA public TO ephemeral_vault_app;
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ephemeral_vault_app;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO ephemeral_vault_app;
-- GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO ephemeral_vault_app;

-- ============================================================================
-- END OF SCHEMA
-- ============================================================================
