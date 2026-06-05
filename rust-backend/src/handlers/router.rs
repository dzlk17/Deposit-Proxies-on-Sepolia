use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;
use tracing::{error, info};

use crate::db;
use crate::models::ErrorResponse;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct RouterResponse {
    pub checked: usize,
    pub routed: usize,
    pub tx_hashes: Vec<String>,
}

pub async fn handle_router(
    State(state): State<Arc<AppState>>,
    _body: String,
) -> Result<Json<RouterResponse>, ErrorResponse> {
    let deposits = match {
        let db = state.db.lock().unwrap();
        db::get_all_deposits(&db)
    } {
        Ok(d) => d,
        Err(e) => {
            return Err(ErrorResponse::new(
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("DB error: {}", e),
            ));
        }
    };

    info!("Starting routing: {} deposits to check", deposits.len());

    let mut checked = 0usize;
    let mut routed = 0usize;
    let mut tx_hashes: Vec<String> = Vec::new();

    let deployer_addr = ethers::types::Address::from_slice(&state.deployer_address);

    for deposit in &deposits {
        checked += 1;
        info!("Checking deposit #{}: {}", checked, deposit.deposit_address);

        let salt_hex = deposit
            .salt
            .strip_prefix("0x")
            .unwrap_or(&deposit.salt)
            .to_string();
        let salt_bytes = match hex::decode(&salt_hex) {
            Ok(b) => b,
            Err(_) => {
                info!("Skipping {}: invalid salt", deposit.deposit_address);
                continue;
            }
        };
        if salt_bytes.len() != 32 {
            info!("Skipping {}: salt length != 32", deposit.deposit_address);
            continue;
        }
        let mut salt_arr = [0u8; 32];
        salt_arr.copy_from_slice(&salt_bytes);

        let balance = match state.eth_client.get_balance(&deposit.deposit_address).await {
            Ok(b) => b,
            Err(e) => {
                info!("Skipping {}: balance check error: {}", deposit.deposit_address, e);
                continue;
            }
        };

        if balance <= 0.0001 {
            info!("Skipping {}: low balance ({})", deposit.deposit_address, balance);
            continue;
        }

        info!("Deploying proxy for {} (balance: {})", deposit.deposit_address, balance);
        match state
            .eth_client
            .deploy_proxy(salt_arr, deployer_addr)
            .await
        {
            Ok(tx_hash) => {
                info!("Deploy tx sent: {:?}", tx_hash);

                let proxy_addr: ethers::types::Address =
                    match deposit.deposit_address.parse() {
                        Ok(a) => a,
                        Err(_) => continue,
                    };
                let balance_wei = ethers::types::U256::from((balance * 1e18) as u128);

                info!("Routing {} ETH from {} to treasury", balance, proxy_addr);
                match state
                    .eth_client
                    .route_proxy(proxy_addr, balance_wei)
                    .await
                {
                    Ok(route_tx) => {
                        info!("Route tx sent: {:?}", route_tx);
                        routed += 1;
                        tx_hashes.push(format!("{:?}", tx_hash));
                        tx_hashes.push(format!("{:?}", route_tx));

                        let db = state.db.lock().unwrap();
                        let _ = db::update_status(&db, &deposit.deposit_address, "routed");
                    }
                    Err(e) => {
                        error!("Route failed for {}: {}", deposit.deposit_address, e);
                    }
                }
            }
            Err(e) => {
                error!("Deploy failed for {}: {}", deposit.deposit_address, e);
            }
        }
    }

    info!("Routing complete: checked={}, routed={}", checked, routed);
    Ok(Json(RouterResponse {
        checked,
        routed,
        tx_hashes,
    }))
}
