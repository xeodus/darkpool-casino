# Ephemeral Vault System

A Solana-based system for gasless trading through temporary, session-based wallets on a dark pool perpetual futures DEX.

## Overview

The Ephemeral Vault System enables high-frequency trading without constant wallet signing by:
- Creating temporary ephemeral wallets for trading sessions
- Delegating limited trading authority from parent wallets
- Auto-managing SOL deposits for transaction fees
- Ensuring secure session cleanup with automatic fund returns

## Architecture

### Components

1. **Anchor Smart Contract** (`programs/`)
   - PDA-based vault accounts
   - Session-based delegation management
   - Automatic fund cleanup on expiry

2. **Rust Backend Service** (`backend/`)
   - Session management with encrypted key storage
   - Auto-deposit calculator
   - Transaction signing service
   - REST API for integration

3. **PostgreSQL Database**
   - Active session tracking
   - Vault transaction history
   - Delegation records
   - Cleanup event logs

## Smart Contract Instructions

### 1. Create Vault
```rust
create_ephemeral_vault(approved_amount: u64, session_duration: i64)
```
Creates a new ephemeral vault PDA for the parent wallet with a spending ceiling and session duration.

### 2. Approve Delegate
```rust
approve_delegate(ephemeral_wallet: Pubkey)
```
Grants trading permissions to the ephemeral wallet. Must be signed by parent wallet.

### 3. Auto Deposit
```rust
auto_deposit_for_trade(amount: u64)
```
Transfers SOL from parent wallet to vault for transaction fees.

### 4. Execute Trade
```rust
execute_trade(trade_amount: u64, trading_fee: u64)
```
Validates delegate authority, session status, and spend limits while accounting for fee usage.

### 5. Revoke Access
```rust
revoke_access()
```
Parent wallet revokes delegation and retrieves remaining funds.

### 6. Cleanup Vault
```rust
cleanup_vault()
```
Anyone can call after session expiry to return funds and close vault. Cleanup caller receives small reward.

## API Endpoints

### Session Management

```
POST   /api/session/create         # bootstrap session + create-vault ix
POST   /api/session/approve        # prepare approve-delegate ix
POST   /api/session/deposit        # prepare auto-deposit ix
POST   /api/session/sign           # sign & submit trade via ephemeral wallet
DELETE /api/session/revoke         # prepare revoke ix
GET    /api/session/status/:id     # session snapshot
GET    /api/session/stats          # operational metrics
GET    /api/health
```

### Example: Create Session

**Request:**
```json
POST /api/session/create
{
  "parent_wallet": "5XqZ...",
  "approved_amount": 1000000000,
  "session_duration": 3600,
  "expected_transactions": 120
}
```

**Response:**
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "ephemeral_wallet": "9KpR...",
  "vault_address": "8YtP...",
  "parent_wallet": "5XqZ...",
  "expires_at": "2025-11-06T15:30:00Z",
  "suggested_deposit": 1500000,
  "instructions": [
    {
      "program_id": "9N97GnZ47zpk8VXBq7wRdKQEUbJT19r7XFQK2XahJYa3",
      "accounts": [
        { "pubkey": "5XqZ...", "is_signer": true, "is_writable": true },
        { "pubkey": "8YtP...", "is_signer": false, "is_writable": true },
        { "pubkey": "11111111111111111111111111111111", "is_signer": false, "is_writable": false }
      ],
      "data": "B3fR...=="
    }
  ]
}
```

## Database Schema

### Sessions Table
- Stores active and historical sessions
- Encrypted ephemeral keypair storage
- Session expiry tracking

### Vault Transactions Table
- Transaction history per vault
- Amount tracking and status

### Delegations Table
- Delegation approval and revocation records

### Cleanup Events Table
- Automated cleanup logs
- Fund return tracking

## Security Features

1. **Encrypted Key Storage**: Ephemeral keys encrypted at rest using AES-256-GCM
2. **Session Expiry**: Automatic session termination after specified duration
3. **Limited Delegation**: Ephemeral wallets have trading-only permissions
4. **Parent Control**: Parent wallet can revoke access anytime
5. **Deposit Limits**: Maximum deposit per session enforced
6. **Secure Cleanup**: Guaranteed fund return on session expiry

## Session Lifecycle/Ambitions

```
User Connects
    ↓
Generate Ephemeral Wallet
    ↓
Create Vault (on-chain)
    ↓
Approve Delegation (parent signs)
    ↓
Auto-Deposit SOL
    ↓
Execute Trades (100+ without signing)
    ↓
Session Expires / Manual Revoke
    ↓
Cleanup & Return Funds
```

## Configuration

Environment variables (see `.env.example`):

```env
SERVER_ADDR=0.0.0.0:500
DATABASE_URL=postgresql://...
SOLANA_RPC_URL=https://api.devnet.solana.com
PROGRAM_ID=********************************************
ENCRYPTION_KEY=<64-character-hex-key>
```

## Development Setup

### Prerequisites
- Rust 1.75+
- Anchor Framework 0.29+
- PostgreSQL 14+
- Solana CLI tools

### Build Anchor Program
```bash
anchor build
```

### Run Backend Service
```bash
cd backend
cargo run
```

### Database Setup

```bash
docker rm -f pg16 2>/dev/null || true
docker run -d --name pg16 \
  -e POSTGRES_USER=ricky \
  -e POSTGRES_PASSWORD='password' \
  -e POSTGRES_DB=goquant \
  -p 5342:5432 postgres:16
# wait until ready
docker logs -f pg16 | sed -n '/ready to accept connections/q'

# check 
docker ps

psql "postgres://ricky:lucifer%40@127.0.0.1:5342/goquant"

# Then inside the interaction terminal we can fetch different tables from the DB using
\dt
# or,
\dt+ name_of_the_table

```

## Performance Targets

- Session creation: < 500ms
- Transaction signing: < 50ms
- Concurrent sessions: 1000+
- Transactions per session: 100+

## Testing

```bash
anchor test
```

```bash
cd backend
cargo test
```

## Pending work

**Security Analysis**

- Threat model
- Attack surface analysis
- Mitigation strategies

**Advanced Features**

- Multi-signature parent wallets
- Spending limits per session
- Hierarchical vault structures
- Cross-device session continuity

**UX Enhancements**

- One-click session creation
- Seamless session refresh
- Mobile wallet integration
- QR code session recovery

**Security Hardening**

- Hardware security module (HSM) integration
- Biometric verification options
- IP whitelisting
- Device fingerprinting

## Security Considerations

1. **Key Management**: Ephemeral keys are encrypted using AES-256-GCM with a secure encryption key
2. **Session Validation**: All operations validate session expiry and active status
3. **Parent Authority**: Critical operations require parent wallet signature
4. **Deposit Limits**: Per-session deposit caps prevent excessive fund exposure
5. **Automatic Cleanup**: Ensures no funds are locked in expired vaults