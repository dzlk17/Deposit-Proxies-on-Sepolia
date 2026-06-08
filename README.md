# Deterministic Deposit Proxies on Sepolia

Precomputes and deploys CREATE2 minimal proxy contracts (EIP-1167) that forward ETH to a `FundRouter` contract, which routes funds to a configurable treasury address.

## How to Run

### Backend (Rust Axum)
```bash
cp .env.example .env   # fill in env vars
cargo run --manifest-path rust-backend/Cargo.toml
```
Server starts on `http://0.0.0.0:3001`.

### Frontend (Next.js)
```bash
cd app
pnpm install
pnpm dev
```
Frontend starts on `http://localhost:3000`.

## Deployment (Sepolia)

| Contract | Address |
|----------|---------|
| DeterministicProxyDeployer | `0xCdF6E4419FdDCBca86C00448ed369EF21453A4D9` |
| FundRouter | `0xA798a7e85FDDef5eA0c71b06987D3d149B506804` |
| FundRouterStorage | `0x53eB9b75B175636584B12fAcb5152e4b78e2511f` |
| Treasury | `0x3367436E1D23f6e562924D69A7785848b2A0348c` |
| Deployer | `0x83613e3B88e380fE43f8D7A911B67a791372a9dC` |

## Assumptions

- Minimal proxy forwards calls & ETH to `FUND_ROUTER_ADDRESS` via `delegatecall` (EIP-1167).
- ETH lands in `FundRouter` via proxy forwarding; `FundRouter` forwards from its own balance.
- ERC20 tokens are held by `FundRouter` when `transferFunds` is called.
- `CHAIN_ID` defaults to Sepolia (`11155111`).
- SQLite for local persistence.

## TODOs Implemented

| TODO | File | Description |
|------|------|-------------|
| `_proxyInitCode()` | `contracts/DeterministicProxyDeployer.sol` | EIP-1167 minimal proxy init code using `abi.encodePacked` |
| `_isAllowedCaller()` | `contracts/FundRouter.sol` | `staticcall` to `FundRouterStorage.isAllowedCaller(address)` |
| `_isAllowedTreasury()` | `contracts/FundRouter.sol` | `staticcall` to `FundRouterStorage.isAllowedTreasury(address)` |
| `transferFunds()` ERC20 | `contracts/FundRouter.sol` | `require(IERC20(token).transfer(treasuryAddress, amt))` |

## Screenshot

<p align="center">
  <img width="1052" alt="Deposit Proxies Dashboard" src="https://github.com/user-attachments/assets/d2a24abe-a3d3-48c6-a37b-70cc3875f684" />
  <br/>
  <em>Frontend dashboard showing deposit addresses, balances, and treasury status.</em>
</p>

## Potential Improvements

### Balance Fetching — Unnecessary RPC Load

**Problem:** The background polling loop (`main.rs:96-105`) and the WebSocket connection handler (`ws_handler.rs:24-33`) fetch ETH balances for **all** deposit addresses in the database every 15 seconds — including addresses whose status is already `"routed"`. Once a deposit has been routed, its proxy is deployed and its ETH has been forwarded to the treasury, so its balance will always be 0. These RPC calls are wasted and compound linearly as more deposits are processed.

**Proposed fix:** Filter out routed deposits when fetching balances:

1. Add a `get_pending_deposits()` function in `db.rs` that queries `SELECT ... WHERE status != 'routed'`.
2. Replace `get_all_deposits()` with `get_pending_deposits()` in `ws_handler.rs`.