CREATE TABLE IF NOT EXISTS cleanup_events (
    id SERIAL PRIMARY KEY,
    vault_address VARCHAR(44) NOT NULL,
    returned_amount BIGINT NOT NULL,
    cleanup_caller VARCHAR(44) NOT NULL,
    signature VARCHAR(88),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_cleanup_events_vault ON cleanup_events(vault_address);
