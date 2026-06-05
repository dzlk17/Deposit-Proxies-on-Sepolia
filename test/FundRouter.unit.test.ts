import { expect } from "chai";
import { ethers } from "hardhat";
import { FundRouter, FundRouterStorage, DeterministicProxyDeployer } from "../typechain-types";

describe("FundRouterStorage", function () {
  let storage: FundRouterStorage;
  let owner: any, caller: any, treasury: any, stranger: any;

  before(async () => {
    [owner, caller, treasury, stranger] = await ethers.getSigners();
    const Factory = await ethers.getContractFactory("FundRouterStorage");
    storage = await Factory.deploy(owner.address);
    await storage.waitForDeployment();
  });

  it("should set owner on deploy", async () => {
    expect(await storage.owner()).to.equal(owner.address);
  });

  it("should revert on zero-address owner", async () => {
    const Factory = await ethers.getContractFactory("FundRouterStorage");
    await expect(Factory.deploy(ethers.ZeroAddress)).to.be.revertedWithCustomError(
      storage, "ZeroAddress"
    );
  });

  it("should set permissions (owner only)", async () => {
    await storage.setPermissions(caller.address, 0x01);
    await storage.setPermissions(treasury.address, 0x02);
    expect(await storage.isAllowedCaller(caller.address)).to.be.true;
    expect(await storage.isAllowedTreasury(treasury.address)).to.be.true;
  });

  it("should revert setPermissions from non-owner", async () => {
    await expect(
      storage.connect(stranger).setPermissions(stranger.address, 0x01)
    ).to.be.revertedWithCustomError(storage, "NotOwner");
  });

  it("should allow combined caller+treasury (0x03)", async () => {
    await storage.setPermissions(treasury.address, 0x03);
    expect(await storage.isAllowedCaller(treasury.address)).to.be.true;
    expect(await storage.isAllowedTreasury(treasury.address)).to.be.true;
  });

  it("should transfer ownership", async () => {
    await storage.transferOwnership(caller.address);
    expect(await storage.owner()).to.equal(caller.address);
    await storage.connect(caller).transferOwnership(owner.address);
    expect(await storage.owner()).to.equal(owner.address);
  });

  it("should revert transferOwnership from non-owner", async () => {
    await expect(
      storage.connect(stranger).transferOwnership(stranger.address)
    ).to.be.revertedWithCustomError(storage, "NotOwner");
  });

  it("should revert isAllowedCallerAndTreasury correctly", async () => {
    await storage.setPermissions(caller.address, 0x01);
    await storage.setPermissions(treasury.address, 0x02);
    expect(await storage.isAllowedCallerAndTreasury(caller.address, treasury.address)).to.be.true;
    expect(await storage.isAllowedCallerAndTreasury(stranger.address, treasury.address)).to.be.false;
    expect(await storage.isAllowedCallerAndTreasury(caller.address, stranger.address)).to.be.false;
  });
});

describe("FundRouter", function () {
  let storage: FundRouterStorage;
  let router: FundRouter;
  let owner: any, caller: any, treasury: any, stranger: any;

  before(async () => {
    [owner, caller, treasury, stranger] = await ethers.getSigners();
    const StorageFactory = await ethers.getContractFactory("FundRouterStorage");
    storage = await StorageFactory.deploy(owner.address);
    await storage.waitForDeployment();

    const RouterFactory = await ethers.getContractFactory("FundRouter");
    router = await RouterFactory.deploy(await storage.getAddress());
    await router.waitForDeployment();

    await storage.setPermissions(caller.address, 0x01);
    await storage.setPermissions(treasury.address, 0x02);
  });

  it("should reject transferFunds from non-allowed caller", async () => {
    await expect(
      router.connect(stranger).transferFunds(0, [], [], treasury.address)
    ).to.be.revertedWithCustomError(router, "NotAuthorizedCaller");
  });

  it("should reject zero treasury", async () => {
    await expect(
      router.connect(caller).transferFunds(0, [], [], ethers.ZeroAddress)
    ).to.be.revertedWithCustomError(router, "ZeroTreasury");
  });

  it("should reject non-allowed treasury", async () => {
    await expect(
      router.connect(caller).transferFunds(0, [], [], stranger.address)
    ).to.be.revertedWithCustomError(router, "TreasuryNotAllowed");
  });

  it("should reject length mismatch", async () => {
    await expect(
      router.connect(caller).transferFunds(0, [ethers.ZeroAddress], [], treasury.address)
    ).to.be.revertedWithCustomError(router, "LengthMismatch");
  });

  it("should transfer ETH to treasury", async () => {
    const amount = ethers.parseEther("0.1");
    await owner.sendTransaction({ to: await router.getAddress(), value: amount });
    const treasuryBefore = await ethers.provider.getBalance(treasury.address);
    const routerBefore = await ethers.provider.getBalance(await router.getAddress());

    await router.connect(caller).transferFunds(amount, [], [], treasury.address);

    expect(await ethers.provider.getBalance(treasury.address)).to.equal(treasuryBefore + amount);
    expect(await ethers.provider.getBalance(await router.getAddress())).to.equal(routerBefore - amount);
  });

  it("should transfer ERC20 tokens to treasury", async () => {
    const ERC20 = await ethers.getContractFactory("TestERC20");
    const token = await ERC20.deploy("Test", "TST", ethers.parseEther("1000"));
    await token.waitForDeployment();
    const tokenAddr = await token.getAddress();

    await token.transfer(await router.getAddress(), ethers.parseEther("100"));

    const treasuryBefore = await token.balanceOf(treasury.address);
    await router.connect(caller).transferFunds(
      0,
      [tokenAddr],
      [ethers.parseEther("50")],
      treasury.address
    );
    expect(await token.balanceOf(treasury.address)).to.equal(treasuryBefore + ethers.parseEther("50"));
  });

  it("should not revert on zero ETH transfer", async () => {
    await router.connect(caller).transferFunds(0, [], [], treasury.address);
  });
});

describe("DeterministicProxyDeployer", function () {
  let deployer: DeterministicProxyDeployer;
  let router: FundRouter;
  let owner: any, user: any;

  before(async () => {
    [owner, user] = await ethers.getSigners();
    const StorageFactory = await ethers.getContractFactory("FundRouterStorage");
    const storage = await StorageFactory.deploy(owner.address);
    await storage.waitForDeployment();

    const RouterFactory = await ethers.getContractFactory("FundRouter");
    router = await RouterFactory.deploy(await storage.getAddress());
    await router.waitForDeployment();

    const DeployerFactory = await ethers.getContractFactory("DeterministicProxyDeployer");
    deployer = await DeployerFactory.deploy(await router.getAddress());
    await deployer.waitForDeployment();
  });

  it("should deploy a proxy via deployMultiple", async () => {
    const salt = ethers.hexlify(ethers.randomBytes(32));
    const [expectedAddr] = await deployer.calculateDestinationAddresses([salt]);
    const tx = await deployer.deployMultiple([salt]);
    await tx.wait();
    const code = await ethers.provider.getCode(expectedAddr);
    expect(code).to.not.equal("0x");
  });

  it("should calculate correct destination addresses", async () => {
    const salt = ethers.hexlify(ethers.randomBytes(32));
    const [preview] = await deployer.calculateDestinationAddresses([salt]);
    const tx = await deployer.deployMultiple([salt]);
    await tx.wait();
    const code = await ethers.provider.getCode(preview);
    expect(code).to.not.equal("0x");
  });

  it("should revert on duplicate salt", async () => {
    const salt = ethers.hexlify(ethers.randomBytes(32));
    await (await deployer.deployMultiple([salt])).wait();
    await expect(
      deployer.deployMultiple([salt])
    ).to.be.revertedWithCustomError(deployer, "Create2Failed");
  });

  it("should deploy multiple proxies", async () => {
    const salts = Array.from({ length: 3 }, () => ethers.hexlify(ethers.randomBytes(32)));
    const [a0, a1, a2] = await deployer.calculateDestinationAddresses(salts);
    const tx = await deployer.deployMultiple(salts);
    await tx.wait();
    const codes = await Promise.all([a0, a1, a2].map(a => ethers.provider.getCode(a)));
    codes.forEach(code => expect(code).to.not.equal("0x"));
  });

  it("should forward ETH via proxy to router and allow transferFunds", async () => {
    const StorageFactory = await ethers.getContractFactory("FundRouterStorage");
    const storage = await StorageFactory.deploy(owner.address);
    await storage.waitForDeployment();

    const RouterFactory = await ethers.getContractFactory("FundRouter");
    const router2 = await RouterFactory.deploy(await storage.getAddress());
    await router2.waitForDeployment();

    const DeployerFactory = await ethers.getContractFactory("DeterministicProxyDeployer");
    const deployer2 = await DeployerFactory.deploy(await router2.getAddress());
    await deployer2.waitForDeployment();

    await storage.setPermissions(owner.address, 0x01);
    const treasury = user;
    await storage.setPermissions(treasury.address, 0x02);

    const salt = ethers.hexlify(ethers.randomBytes(32));
    const [proxyAddr] = await deployer2.calculateDestinationAddresses([salt]);
    await (await deployer2.deployMultiple([salt])).wait();

    const amount = ethers.parseEther("0.05");
    await owner.sendTransaction({ to: proxyAddr, value: amount });

    const treasuryBefore = await ethers.provider.getBalance(treasury.address);

    const proxyAsRouter = router2.attach(proxyAddr) as FundRouter;
    await proxyAsRouter.connect(owner).transferFunds(amount, [], [], treasury.address);

    expect(await ethers.provider.getBalance(treasury.address)).to.equal(treasuryBefore + amount);
  });
});
