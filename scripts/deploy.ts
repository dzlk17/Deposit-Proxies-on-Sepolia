import { ethers } from "hardhat";
import * as fs from "fs";
import * as path from "path";

// I wrote first deploy script in ts, then added one in rust as in instructions.
async function main() {
  const [deployer] = await ethers.getSigners();
  console.log("Deploying with account:", deployer.address);
  console.log("Balance:", (await ethers.provider.getBalance(deployer.address)).toString());

  const treasury = process.env.NEXT_PUBLIC_TREASURY_ADDRESS;

  // ── 1. Deploy FundRouterStorage ──────────────────────────────────
  console.log("\n1. Deploying FundRouterStorage...");
  const FundRouterStorage = await ethers.getContractFactory("FundRouterStorage");
  const storage = await FundRouterStorage.deploy(deployer.address);
  await storage.waitForDeployment();
  const storageAddress = await storage.getAddress();
  console.log("   FundRouterStorage:", storageAddress);

  // ── 2. setPermissions(deployer, 0x01) ────────────────────────────
  console.log("\n2. Setting deployer as allowed caller...");
  let tx = await storage.setPermissions(deployer.address, 0x01);
  await tx.wait();
  console.log("   Deployer is now an allowed caller");

  // ── 3. setPermissions(TREASURY_ADDRESS, 0x02) ────────────────────
  console.log("\n3. Setting treasury as allowed...");
  if (treasury && treasury !== "0x123...") {
    tx = await storage.setPermissions(treasury, 0x02);
    await tx.wait();
    console.log("   Treasury is now allowed:", treasury);
  } else {
    console.log("   ⚠️  No valid NEXT_PUBLIC_TREASURY_ADDRESS in .env — skipping");
  }

  // ── 4. Deploy FundRouter ─────────────────────────────────────────
  console.log("\n4. Deploying FundRouter...");
  const FundRouter = await ethers.getContractFactory("FundRouter");
  const router = await FundRouter.deploy(storageAddress);
  await router.waitForDeployment();
  const routerAddress = await router.getAddress();
  console.log("   FundRouter:", routerAddress);

  // ── 5. Deploy DeterministicProxyDeployer ─────────────────────────
  console.log("\n5. Deploying DeterministicProxyDeployer...");
  const ProxyDeployer = await ethers.getContractFactory("DeterministicProxyDeployer");
  const proxyDeployer = await ProxyDeployer.deploy(routerAddress);
  await proxyDeployer.waitForDeployment();
  const proxyDeployerAddress = await proxyDeployer.getAddress();
  console.log("   DeterministicProxyDeployer:", proxyDeployerAddress);

  // ── 6. Save deployment info ──────────────────────────────────────
  console.log("\n6. Saving deployment addresses...");
  const deployment = {
    network: "sepolia",
    deployer: deployer.address,
    FundRouterStorage: storageAddress,
    FundRouter: routerAddress,
    DeterministicProxyDeployer: proxyDeployerAddress,
    treasury: treasury || null,
    timestamp: new Date().toISOString(),
  };

  const outDir = path.resolve(__dirname, "../deployments");
  fs.mkdirSync(outDir, { recursive: true });
  const outPath = path.join(outDir, `sepolia-${Date.now()}.json`);
  fs.writeFileSync(outPath, JSON.stringify(deployment, null, 2));
  console.log("   Addresses saved to:", outPath);

  console.log("\n═══════════════════════════════════════════");
  console.log("  Deploy completed successfully!");
  console.log("═══════════════════════════════════════════\n");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
