use std::sync::Arc;

use axum::{extract::State, Json};
use serde::Serialize;

use crate::db;
use crate::models::DepositRecord;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct DepositsResponse {
    pub deposits: Vec<DepositRecord>,
}

pub async fn handle_get_deposits(
    State(state): State<Arc<AppState>>,
) -> Json<DepositsResponse> {
    let db = state.db.lock().unwrap();
    let deposits = db::get_all_deposits(&db).unwrap_or_default();
    Json(DepositsResponse { deposits })
}
