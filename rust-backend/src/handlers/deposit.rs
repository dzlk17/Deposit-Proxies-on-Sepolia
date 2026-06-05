use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::create2;
use crate::db;
use crate::models::{DepositRecord, ErrorResponse};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct DepositRequest {
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct DepositResponse {
    pub deposit_address: String,
    pub salt: String,
    pub note: String,
}

pub async fn handle_deposit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DepositRequest>,
) -> Result<Json<DepositResponse>, ErrorResponse> {
    let user_addr = req.user.strip_prefix("0x").unwrap_or(&req.user).to_string();
    let user_bytes = hex::decode(&user_addr).map_err(|_| {
        ErrorResponse::new(
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid user address",
        )
    })?;

    let mut user_arr = [0u8; 20];
    if user_bytes.len() != 20 {
        return Err(ErrorResponse::new(
            axum::http::StatusCode::BAD_REQUEST,
            "User address must be 20 bytes",
        ));
    }
    user_arr.copy_from_slice(&user_bytes);

    let counter = {
        let db = state.db.lock().unwrap();
        db::get_all_deposits(&db).map(|r| r.len()).unwrap_or(0)
    };

    let user_salt = {
        let mut salt = [0u8; 32];
        let counter_bytes = (counter as u64).to_be_bytes();
        salt[..8].copy_from_slice(&counter_bytes);
        salt[8..28].copy_from_slice(&user_arr);
        salt
    };

    let contract_salt = create2::derive_salt(&user_salt, &state.wallet_address);
    let deposit_addr =
        create2::compute_create2_address(&state.deployer_address, &contract_salt, &state.init_code);

    let deposit_hex = format!("0x{}", hex::encode(deposit_addr));
    let salt_hex = format!("0x{}", hex::encode(user_salt));

    let record = DepositRecord {
        id: 0,
        user_address: format!("0x{}", hex::encode(user_arr)),
        deposit_address: deposit_hex.clone(),
        salt: salt_hex.clone(),
        status: "pending".to_string(),
        created_at: String::new(),
    };

    {
        let db = state.db.lock().unwrap();
        if let Err(e) = db::insert_deposit(&db, &record) {
            eprintln!("DB insert warning: {}", e);
        }
    }

    Ok(Json(DepositResponse {
        deposit_address: deposit_hex,
        salt: salt_hex,
        note: "Send Sepolia ETH to this address.".to_string(),
    }))
}
