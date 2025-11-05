-- Add migration script here
CREATE TABLE trade_history (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    
    -- Trade specifics
    trade_amount BIGINT NOT NULL,               -- Trade size in lamports
    trade_fee BIGINT NOT NULL,                  -- Fee paid for trade
    signature VARCHAR(88) NOT NULL UNIQUE,
    timestamp BIGINT NOT NULL,
    
    -- Market details (extend for your DEX)
    side VARCHAR(10),                           -- 'BUY' or 'SELL'
    market VARCHAR(50),                         -- Trading pair (e.g., 'SOL/USDC')
    price NUMERIC(20, 8),                       -- Execution price
    filled_amount BIGINT,                       -- Amount filled
    
    -- Status
    status VARCHAR(20) DEFAULT 'confirmed',
    slippage_bps INT,                           -- Slippage in basis points
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    CONSTRAINT valid_side CHECK (side IN ('BUY', 'SELL', NULL)),
    CONSTRAINT positive_amounts CHECK (trade_amount > 0 AND trade_fee >= 0)
);

-- Performance indexes
CREATE INDEX idx_trade_history_session_id ON trade_history(session_id);
CREATE INDEX idx_trade_history_timestamp ON trade_history(timestamp DESC);
CREATE INDEX idx_trade_history_signature ON trade_history(signature);
CREATE INDEX idx_trade_history_market ON trade_history(market) WHERE market IS NOT NULL;
CREATE INDEX idx_trade_history_status ON trade_history(status);

COMMENT ON TABLE trade_history IS 'Detailed trading activity with market data';