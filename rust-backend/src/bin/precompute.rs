use sha3::{Digest, Keccak256};

fn build_init_code(fund_router_address: &[u8; 20]) -> Vec<u8> {
    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&hex::decode("3d602d80600a3d3981f3").unwrap());
    code.extend_from_slice(&hex::decode("363d3d373d3d3d363d73").unwrap());
    code.extend_from_slice(fund_router_address);
    code.extend_from_slice(&hex::decode("5af43d82803e903d91602b57fd5bf3").unwrap());
    code
}

fn derive_salt(user_salt: &[u8; 32], caller: &[u8; 20]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(user_salt);
    hasher.update(caller);
    let result = hasher.finalize();
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&result);
    salt
}

fn compute_create2_address(deployer: &[u8; 20], salt: &[u8; 32], init_code: &[u8]) -> [u8; 20] {
    let mut hasher = Keccak256::new();
    hasher.update([0xff]);
    hasher.update(deployer);
    hasher.update(salt);
    let mut init_hasher = Keccak256::new();
    init_hasher.update(init_code);
    let init_code_hash = init_hasher.finalize();
    hasher.update(init_code_hash);
    let result = hasher.finalize();
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&result[12..]);
    addr
}

fn parse_addr(s: &str) -> [u8; 20] {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).expect("Invalid hex address");
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&bytes);
    addr
}

fn parse_salt(s: &str) -> [u8; 32] {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).expect("Invalid hex salt");
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&bytes);
    salt
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let salt_hex = if args.len() > 1 {
        args[1].clone()
    } else {
        eprintln!("Usage: cargo run --bin precompute -- <salt_hex> [deployer_address] [fund_router_address]");
        eprintln!("");
        eprintln!("Reads DEPLOYER_CONTRACT_ADDRESS and FUND_ROUTER_ADDRESS from .env if not provided as args.");
        std::process::exit(1);
    };

    let deployer_hex = if args.len() > 2 {
        args[2].clone()
    } else {
        dotenvy::dotenv().ok();
        std::env::var("DEPLOYER_CONTRACT_ADDRESS").expect("DEPLOYER_CONTRACT_ADDRESS must be set in .env or provided as arg")
    };

    let router_hex = if args.len() > 3 {
        args[3].clone()
    } else {
        dotenvy::dotenv().ok();
        std::env::var("FUND_ROUTER_ADDRESS").expect("FUND_ROUTER_ADDRESS must be set in .env or provided as arg")
    };

    let deployer = parse_addr(&deployer_hex);
    let fund_router = parse_addr(&router_hex);
    let salt = parse_salt(&salt_hex);

    let init_code = build_init_code(&fund_router);

    let derived_salt = derive_salt(&salt, &deployer);

    let proxy_addr = compute_create2_address(&deployer, &derived_salt, &init_code);

    println!("Input salt:          0x{}", hex::encode(salt));
    println!("Deployer:            0x{}", hex::encode(deployer));
    println!("FundRouter:          0x{}", hex::encode(fund_router));
    println!("Derived salt:        0x{}", hex::encode(derived_salt));
    println!("Init code hash:      0x{}", hex::encode(Keccak256::digest(&init_code)));
    println!("───────────────────────────────────────────────");
    println!("Predicted proxy address: 0x{}", hex::encode(proxy_addr));
}
