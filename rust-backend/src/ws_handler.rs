use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures_util::future::join_all;
use futures_util::{SinkExt, StreamExt};

use crate::{db, AppState, BalanceUpdate};

pub async fn handle_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.ws_tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    let send_task = tokio::spawn(async move {
        let deposits = {
            let db = state.db.lock().unwrap();
            db::get_all_deposits(&db).unwrap_or_default()
        };

        let balance_futures: Vec<_> = deposits.iter().map(|d| {
            let addr = d.deposit_address.clone();
            let state = state.clone();
            async move { state.eth_client.get_balance(&addr).await }
        }).collect();

        let balance_results = join_all(balance_futures).await;

        let mut balances = HashMap::new();
        for (d, result) in deposits.iter().zip(balance_results.iter()) {
            let balance = result.as_ref().unwrap_or(&0.0);
            balances.insert(d.deposit_address.clone(), format!("{:.4}", balance));
        }

        let treasury_addr = format!("{:#x}", state.eth_client.treasury_address);
        let treasury_balance = state
            .eth_client
            .get_balance(&treasury_addr)
            .await
            .unwrap_or(0.0);

        let update = BalanceUpdate {
            treasury_balance: format!("{:.4}", treasury_balance),
            balances,
        };
        let text = serde_json::to_string(&update).unwrap();
        if sender.send(Message::Text(text.into())).await.is_err() {
            return;
        }

        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let text = serde_json::to_string(&msg).unwrap();
                    if sender.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
