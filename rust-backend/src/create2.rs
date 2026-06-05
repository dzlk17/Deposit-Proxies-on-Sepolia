use sha3::{Digest, Keccak256};

pub fn compute_create2_address(
    deployer: &[u8; 20],
    salt: &[u8; 32],
    init_code: &[u8],
) -> [u8; 20] {
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

pub fn derive_salt(user_salt: &[u8; 32], caller: &[u8; 20]) -> [u8; 32] {
    let mut hasher = Keccak256::new();
    hasher.update(user_salt);
    hasher.update(caller);
    let result = hasher.finalize();
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&result);
    salt
}

pub fn build_init_code(fund_router_address: &[u8; 20]) -> Vec<u8> {
    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&hex::decode("3d602d80600a3d3981f3").unwrap());
    code.extend_from_slice(&hex::decode("363d3d373d3d3d363d73").unwrap());
    code.extend_from_slice(fund_router_address);
    code.extend_from_slice(&hex::decode("5af43d82803e903d91602b57fd5bf3").unwrap());
    code
}
