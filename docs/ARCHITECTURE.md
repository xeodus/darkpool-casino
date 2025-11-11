# Ephemeral Vault System Architecture

## Overview
- Enable session-based, gasless trading for a dark pool perpetual futures DEX.
- Parent wallets remain custodial authority while delegating trading permissions to ephemeral session wallets.
- Anchor program manages vault lifecycle, delegation state, and SOL fee floats.
- Rust backend orchestrates session lifecycle, Solana transactions, and persistence in PostgreSQL.

## On-Chain Program (Anchor)
- **Program ID**: `ephemeral_vault`.
- **PDA Strategy**
  - `EphemeralVault`: `seeds = ["vault", parent_wallet]`.
  - `VaultDelegation`: `seeds = ["delegation", vault]`.
  - `CleanupRewardVault` (optional future optimisation): `seeds = ["cleanup", parent_wallet]`.
- **Accounts**
  - `EphemeralVault`
    - `parent_wallet` – owner of trading funds.
    - `ephemeral_wallet` – delegated signer (set during approval).
    - `session_start`, `session_expiry`, `last_activity` – timestamps for lifecycle management.
    - `total_deposited`, `total_spent`, `available_buffer` – SOL accounting.
    - `approved_amount` – cap on session spending.
    - `status` – active / revoked / expired.
    - `bump`.
  - `VaultDelegation`
    - `vault` – linked vault PDA.
    - `delegate` – delegated ephemeral wallet.
    - `approved_at`, `revoked_at`.
    - `is_active`.
    - `bump`.
- **Instructions**
  - `create_ephemeral_vault(parent_wallet, session_duration, approved_amount)`
    - Initialise vault PDA, session metadata, approved spend ceiling.
  - `approve_delegate(parent_wallet, delegate)`
    - Verifies caller, records delegate, creates/updates `VaultDelegation`.
  - `auto_deposit_for_trade(parent_wallet, vault, fee_estimate)`
    - Transfers SOL from parent to vault, enforces approved cap, updates accounting.
  - `execute_trade(ephemeral_wallet, vault, trade_amount, fee_paid)`
    - Requires delegate signer, checks active session, increments spend totals.
  - `revoke_access(parent_wallet, vault)`
    - Marks vault inactive, refunds SOL to parent, updates delegation.
  - `cleanup_vault(cleaner, parent_wallet, vault)`
    - Callable post-expiry; refunds parent, pays bounty to cleaner, closes PDA.
- **Events**
  - `VaultCreated`, `DelegateApproved`, `FundDeposited`, `TradeExecuted`, `AccessRevoked`, `VaultCleaned`.
- **Guards & Constraints**
  - Session duration bounds, per-session SOL caps, clock-based expiry enforcement.
  - Arithmetic overflow protection, rent-safe closures, delegation authority checks.

## Backend Services (Rust, Axum)
- **SessionManager**
  - Generates ephemeral keypairs (in-memory + encrypted storage).
  - Persists session metadata, expiry, and activity timestamps.
  - Exposes lifecycle helpers: create, fetch, revoke, refresh, expire.
- **DelegationManager**
  - Builds Anchor-compliant instructions for create/approve/revoke flows.
  - Orchestrates signature collection from parent wallets (off-chain) and submission to Solana.
- **DepositCalculator**
  - Estimates SOL requirements per trading session based on transaction volume/fee heuristics.
  - Determines top-up thresholds and amounts for auto-deposits.
- **VaultMonitor**
  - Periodic tasks to detect expired sessions, low balances, anomalous spending.
  - Triggers cleanups/revocations and emits alerts via logging/metrics.
- **TransactionSigner**
  - Signs with ephemeral keypairs, manages priority fees, retries, and confirmation polling.

## Session Workflow
1. **Connect**: Backend validates parent wallet, generates session UUID + ephemeral keypair.
2. **Create Vault**: Backend submits `create_ephemeral_vault` instruction signed by parent.
3. **Approve Delegate**: Parent signs delegation transaction; backend records delegation.
4. **Auto-Deposit**: Backend estimates SOL fees and calls `auto_deposit_for_trade`.
5. **Execute Trades**: Client routes unsigned trade payloads via ephemeral wallet signer.
6. **Monitor & Refresh**: Backend monitors balances/session expiry, triggers top-ups as needed.
7. **Revoke / Cleanup**: On logout/timeout, parent or cleaner returns funds and closes vault.

## PostgreSQL Schema
- `sessions`
  - `session_id`, `parent_wallet`, `ephemeral_wallet`, `vault_address`, `created_at`, `expires_at`, `last_activity`, `is_active`.
  - `encrypted_keypair`, `approved_amount`, `total_deposited`, `total_spent`.
- `vault_transactions`
  - `vault_address`, `transaction_type` (deposit/trade/cleanup), `amount`, `signature`, `status`.
- `delegations`
  - `vault_address`, `ephemeral_wallet`, `approved_at`, `revoked_at`, `is_active`, `transaction_signature`.
- `cleanup_events`
  - `vault_address`, `cleanup_caller`, `returned_amount`, `reward_amount`, `signature`.
- `session_analytics`
  - Aggregates per-user/session metrics for monitoring (P95 order latency, SOL usage, etc.).

## API Surface
- `POST /api/session/create`
  - Payload: parent wallet, approved_amount, session_duration.
  - Response: session metadata, vault PDA, ephemeral wallet pubkey, suggested deposit.
- `POST /api/session/approve`
  - Payload: session_id, delegate_signature blob.
  - Orchestrates on-chain approve flow.
- `POST /api/session/deposit`
  - Payload: session_id, override_amount?; triggers auto-deposit transaction.
- `GET /api/session/status/{session_id}`
  - Returns session + vault status snapshot.
- `DELETE /api/session/revoke`
  - Payload: session_id, optional force flag.
- `GET /api/session/stream` (WebSocket)
  - Pushes state changes, balance updates, expiry warnings, anomaly alerts.

## Security & Compliance
- AES-256-GCM encrypted storage for ephemeral keypairs (per ENCRYPTION_ID).
- Parent wallet verification for create/approve/revoke flows.
- Rate limiting + IP heuristics on session creation APIs.
- Anomaly detection hooks in `VaultMonitor` (spend velocity, multi-IP access).
- Emergency kill switch toggles via config/feature flag.

## Performance Considerations
- Session bootstrap target **< 500 ms** (pre-warmed RPC clients + cached blockhashes).
- Transaction signing target **< 50 ms** with in-memory keypairs.
- Tokio tasks for batch cleanup + monitoring to support 1000+ concurrent sessions.
- Efficient PDA derivations and compute unit budgeting in Anchor handlers.

## Testing Strategy
- Anchor integration tests for instruction flow + edge cases (expiry, over-deposit, revocation).
- Backend unit tests for calculators + key management.
- Integration tests using `solana-program-test` or localnet for end-to-end session lifecycle.
- Security regression tests simulating unauthorized delegate usage and replay attacks.

## Future Enhancements
- Multi-signature parent approvals & spending limits per session.
- Hardware Security Module (HSM) integration for key storage.
- Device fingerprinting and IP whitelisting.
- Batched cleanup transactions and compute-efficient trade batching.
