pub mod create2;
pub mod db;
pub mod eth;
pub mod handlers;
pub mod models;
pub mod ws_handler;

use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Clone, Serialize)]
pub struct BalanceUpdate {
    pub treasury_balance: String,
    pub balances: std::collections::HashMap<String, String>,
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub eth_client: eth::EthClient,
    pub deployer_address: [u8; 20],
    pub wallet_address: [u8; 20],
    pub fund_router_address: [u8; 20],
    pub init_code: Vec<u8>,
    pub ws_tx: broadcast::Sender<BalanceUpdate>,
}
