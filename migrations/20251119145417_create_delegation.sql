CREATE TABLE IF NOT EXISTS delegations (
    id SERIAL PRIMARY KEY,
    vault_address VARCHAR(44) NOT NULL,
    ephemeral_wallet VARCHAR(44) NOT NULL,
    approved_at TIMESTAMP NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMP,
    is_active BOOLEAN NOT NULL DEFAULT true
);

CREATE INDEX IF NOT EXISTS idx_delegations_vault ON delegations(vault_address);
