use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DepositRecord {
    pub id: i64,
    pub user_address: String,
    pub deposit_address: String,
    pub salt: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug)]
pub struct ErrorResponse {
    pub code: StatusCode,
    pub error: String,
}

impl ErrorResponse {
    pub fn new(code: StatusCode, error: impl Into<String>) -> Self {
        Self {
            code,
            error: error.into(),
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.error }));
        (self.code, body).into_response()
    }
}
