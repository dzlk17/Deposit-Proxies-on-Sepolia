use ethers::middleware::NonceManagerMiddleware;
use ethers::prelude::*;
use std::sync::Arc;

type EthResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
type TxClient = NonceManagerMiddleware<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>;

abigen!(
    IDeterministicProxyDeployer,
    r#"[
        function deployMultiple(bytes32[] calldata salts) external returns (address[] memory)
        function calculateDestinationAddresses(bytes32[] calldata salts) external view returns (address[] memory)
        function FUND_ROUTER_ADDRESS() external view returns (address)
    ]"#,
);

abigen!(
    IFundRouter,
    r#"[
        function transferFunds(uint256 etherAmount, address[] calldata tokens, uint256[] calldata amounts, address payable treasuryAddress) external
    ]"#,
);

pub struct EthClient {
    pub provider: Arc<Provider<Http>>,
    pub wallet: LocalWallet,
    pub deployer_address: Address,
    pub fund_router_address: Address,
    pub treasury_address: Address,
    pub chain_id: u64,
    pub client: Arc<TxClient>,
}

impl EthClient {
    pub fn new(
        rpc_url: &str,
        private_key: &str,
        fund_router_address: &str,
        treasury_address: &str,
        chain_id: u64,
    ) -> EthResult<Self> {
        let provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        let wallet: LocalWallet = private_key.parse::<LocalWallet>()?.with_chain_id(chain_id);
        let deployer_address = wallet.address();
        let fund_router_address: Address = fund_router_address.parse()?;
        let treasury_address: Address = treasury_address.parse()?;

        let signer =
            SignerMiddleware::new(provider.clone(), wallet.clone().with_chain_id(chain_id));
        let client = Arc::new(NonceManagerMiddleware::new(signer, deployer_address));

        Ok(Self {
            provider,
            wallet,
            deployer_address,
            fund_router_address,
            treasury_address,
            chain_id,
            client,
        })
    }

    pub async fn get_balance(&self, address: &str) -> EthResult<f64> {
        let addr: Address = address.parse()?;
        let balance = self.provider.get_balance(addr, None).await?;
        let eth = balance.as_u128() as f64 / 1e18;
        Ok(eth)
    }

    pub async fn deploy_proxy(&self, salt: [u8; 32], deployer_addr: Address) -> EthResult<H256> {
        let deployer = IDeterministicProxyDeployer::new(deployer_addr, self.client.clone());
        let salt: [u8; 32] = salt;
        let salts = vec![salt];
        let call = deployer.deploy_multiple(salts);
        let tx = call.send().await?;
        let receipt = tx.await?.ok_or("No receipt for deploy proxy tx")?;
        Ok(receipt.transaction_hash)
    }

    pub async fn route_proxy(&self, proxy_address: Address, ether_amount: U256) -> EthResult<H256> {
        let router = IFundRouter::new(proxy_address, self.client.clone());
        let tokens: Vec<Address> = vec![];
        let amounts: Vec<U256> = vec![];
        let call = router.transfer_funds(ether_amount, tokens, amounts, self.treasury_address);
        let tx = call.send().await?;
        let receipt = tx.await?.ok_or("No receipt for route tx")?;
        Ok(receipt.transaction_hash)
    }
}
