use std::path::PathBuf;
use std::sync::Arc;

use dotenvy::dotenv;
use ethers::prelude::*;
use ethers::signers::Signer;

type EthResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn artifact_path(name: &str) -> PathBuf {
    let cwd = std::env::current_dir().unwrap();
    let candidates = vec![
        cwd.join("artifacts").join("contracts").join(format!("{name}.sol")).join(format!("{name}.json")),
        cwd.join("..").join("artifacts").join("contracts").join(format!("{name}.sol")).join(format!("{name}.json")),
    ];
    for p in &candidates {
        if p.exists() {
            return p.clone();
        }
    }
    panic!("Cannot find artifact for {name}. Tried: {:?}", candidates);
}

fn load_artifact(name: &str) -> (ethers::abi::Abi, Bytes) {
    let path = artifact_path(name);
    let file = std::fs::File::open(&path).unwrap();
    let reader = std::io::BufReader::new(file);
    let json: serde_json::Value = serde_json::from_reader(reader).unwrap();
    let abi: ethers::abi::Abi = serde_json::from_value(json["abi"].clone()).unwrap();
    let hex_str = json["bytecode"].as_str().unwrap().strip_prefix("0x").unwrap();
    let bytecode: Bytes = hex::decode(hex_str).unwrap().into();
    (abi, bytecode)
}

fn encode_call<A: ethers::abi::Tokenize + std::fmt::Debug>(func: &str, args: A) -> Vec<u8> {
    let sel = &ethers::utils::keccak256(func.as_bytes())[..4];
    let mut data = sel.to_vec();
    data.extend_from_slice(&ethers::abi::encode(&args.into_tokens()));
    data
}

#[tokio::main]
async fn main() -> EthResult<()> {
    dotenv().ok();

    let rpc_url = std::env::var("SEPOLIA_RPC_URL")?;
    let private_key = std::env::var("PRIVATE_KEY")?;
    let treasury = std::env::var("NEXT_PUBLIC_TREASURY_ADDRESS")?;
    let chain_id: u64 = std::env::var("CHAIN_ID")
        .unwrap_or_else(|_| "11155111".to_string())
        .parse()?;

    let provider = Arc::new(Provider::<Http>::try_from(&rpc_url)?);
    let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
    let deployer_addr = wallet.address();

    let signer = SignerMiddleware::new(provider.clone(), wallet.clone().with_chain_id(chain_id));
    let client: Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>> = Arc::new(signer);

    println!("Deploying with account: {deployer_addr:#x}");
    let balance = provider.get_balance(deployer_addr, None).await?;
    println!("Balance: {balance}");

    // ── 1. Deploy FundRouterStorage ──────────────────────────────────
    println!("\n1. Deploying FundRouterStorage...");
    let (storage_abi, storage_bytecode) = load_artifact("FundRouterStorage");
    let factory = ContractFactory::new(storage_abi, storage_bytecode, client.clone());
    let storage = factory.deploy(deployer_addr)?.send().await?;
    let storage_address = storage.address();
    println!("   FundRouterStorage: {storage_address:#x}");

    // ── 2. setPermissions(deployer, 0x01) ────────────────────────────
    println!("\n2. Setting deployer as allowed caller...");
    let data = encode_call("setPermissions(address,uint8)", (deployer_addr, ethers::types::U256::from(0x01)));
    let tx = TransactionRequest::new().to(storage_address).data(data);
    let receipt = client.send_transaction(tx, None).await?.await?.ok_or("No receipt")?;
    println!("   tx: {:?}", receipt.transaction_hash);

    // ── 3. setPermissions(TREASURY_ADDRESS, 0x02) ────────────────────
    let treasury_addr: Address = treasury.parse()?;
    println!("\n3. Setting treasury as allowed...");
    let data = encode_call("setPermissions(address,uint8)", (treasury_addr, ethers::types::U256::from(0x02)));
    let tx = TransactionRequest::new().to(storage_address).data(data);
    let receipt = client.send_transaction(tx, None).await?.await?.ok_or("No receipt")?;
    println!("   tx: {:?}", receipt.transaction_hash);

    // ── 4. Deploy FundRouter ─────────────────────────────────────────
    println!("\n4. Deploying FundRouter...");
    let (router_abi, router_bytecode) = load_artifact("FundRouter");
    let factory = ContractFactory::new(router_abi, router_bytecode, client.clone());
    let router = factory.deploy(storage_address)?.send().await?;
    let router_address = router.address();
    println!("   FundRouter: {router_address:#x}");

    // ── 5. Deploy DeterministicProxyDeployer ─────────────────────────
    println!("\n5. Deploying DeterministicProxyDeployer...");
    let (deployer_abi, deployer_bytecode) = load_artifact("DeterministicProxyDeployer");
    let factory = ContractFactory::new(deployer_abi, deployer_bytecode, client.clone());
    let proxy_deployer = factory.deploy(router_address)?.send().await?;
    let proxy_deployer_address = proxy_deployer.address();
    println!("   DeterministicProxyDeployer: {proxy_deployer_address:#x}");

    // ── 6. Save deployment info ──────────────────────────────────────
    println!("\n6. Saving deployment addresses...");
    let deployment = serde_json::json!({
        "network": "sepolia",
        "deployer": format!("{deployer_addr:#x}"),
        "FundRouterStorage": format!("{storage_address:#x}"),
        "FundRouter": format!("{router_address:#x}"),
        "DeterministicProxyDeployer": format!("{proxy_deployer_address:#x}"),
        "treasury": treasury,
        "timestamp": format!("{:?}", std::time::SystemTime::now()),
    });

    let depl_dir = std::env::current_dir()?.join("deployments");
    std::fs::create_dir_all(&depl_dir)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let out_path = depl_dir.join(format!("sepolia-{ts}.json"));
    std::fs::write(&out_path, serde_json::to_string_pretty(&deployment)?)?;
    println!("   Addresses saved to: {out_path:?}");

    println!("\n═══════════════════════════════════════════");
    println!("  Deploy completed successfully!");
    println!("═══════════════════════════════════════════\n");

    Ok(())
}
