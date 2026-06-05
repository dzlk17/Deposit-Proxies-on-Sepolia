pub mod create2;
pub mod db;
pub mod eth;
pub mod handlers;
pub mod models;

use std::sync::Mutex;

use rusqlite::Connection;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub eth_client: eth::EthClient,
    pub deployer_address: [u8; 20],
    pub wallet_address: [u8; 20],
    pub fund_router_address: [u8; 20],
    pub init_code: Vec<u8>,
}
