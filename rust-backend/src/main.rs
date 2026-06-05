use std::sync::{Arc, Mutex};

use axum::{Router, routing::{get, post}};
use dotenvy::dotenv;
use ethers::signers::Signer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use rust_backend::{create2, db, eth, handlers, AppState};
use handlers::{deposit::handle_deposit, deposits::handle_get_deposits, router::handle_router};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let rpc_url = std::env::var("SEPOLIA_RPC_URL").expect("SEPOLIA_RPC_URL must be set");
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY must be set");
    let treasury = std::env::var("NEXT_PUBLIC_TREASURY_ADDRESS").expect("NEXT_PUBLIC_TREASURY_ADDRESS must be set");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let fund_router_addr_str =
        std::env::var("FUND_ROUTER_ADDRESS").expect("FUND_ROUTER_ADDRESS must be set");
    let deployer_addr_str =
        std::env::var("DEPLOYER_CONTRACT_ADDRESS").expect("DEPLOYER_CONTRACT_ADDRESS must be set");
    let chain_id: u64 = std::env::var("CHAIN_ID")
        .unwrap_or_else(|_| "11155111".to_string())
        .parse()
        .expect("Invalid CHAIN_ID");

    let db_path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or(&database_url)
        .to_string();
    let conn = db::init_db(&db_path).expect("Failed to init DB");

    let fund_router_bytes = hex::decode(
        fund_router_addr_str
            .strip_prefix("0x")
            .unwrap_or(&fund_router_addr_str),
    )
    .expect("Invalid FUND_ROUTER_ADDRESS");
    let deployer_bytes = hex::decode(
        deployer_addr_str
            .strip_prefix("0x")
            .unwrap_or(&deployer_addr_str),
    )
    .expect("Invalid DEPLOYER_CONTRACT_ADDRESS");

    let mut fra = [0u8; 20];
    fra.copy_from_slice(&fund_router_bytes);
    let mut da = [0u8; 20];
    da.copy_from_slice(&deployer_bytes);

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let init_code = create2::build_init_code(&fra);

    let eth_client = eth::EthClient::new(
        &rpc_url,
        &private_key,
        &fund_router_addr_str,
        &treasury,
        chain_id,
    )
    .expect("Failed to create EthClient");

    let wallet_arr = {
        let addr: [u8; 20] = eth_client.wallet.address().into();
        addr
    };

    let state = Arc::new(AppState {
        db: Mutex::new(conn),
        eth_client,
        deployer_address: da,
        wallet_address: wallet_arr,
        fund_router_address: fra,
        init_code,
    });

    let app = Router::new()
        .route("/deposit", post(handle_deposit))
        .route("/deposits", get(handle_get_deposits))
        .route("/router", post(handle_router))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    println!("Server running on http://0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}
