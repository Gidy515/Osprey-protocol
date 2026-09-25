import DLMM from "@meteora-ag/dlmm";
import { Connection, PublicKey } from "@solana/web3.js";

const RPC_URL = "https://api.devnet.solana.com";

const POOL = new PublicKey("53vdaEoUXhhTXGM9odCYVdvGYzLJnSMXovX5MX9E3xnU");

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");

  const accountInfo = await connection.getAccountInfo(POOL);

  if (!accountInfo) {
    throw new Error("LB pair account does not exist");
  }

  console.log("\n=== Raw Account Verification ===");
  console.log("Pool:", POOL.toBase58());
  console.log("Owner:", accountInfo.owner.toBase58());
  console.log("Executable:", accountInfo.executable);
  console.log("Data length:", accountInfo.data.length);

  const dlmm = await DLMM.create(connection, POOL, {
    cluster: "devnet",
  });

  console.log("\n=== Meteora DLMM State ===");

  console.log("Token X:");
  console.log(dlmm.tokenX.publicKey.toBase58());

  console.log("\nToken X decimals:");
  console.log(dlmm.tokenX.mint.decimals);

  console.log("\nToken Y:");
  console.log(dlmm.tokenY.publicKey.toBase58());

  console.log("\nToken Y decimals:");
  console.log(dlmm.tokenY.mint.decimals);

  console.log("\nBin step:");
  console.log(dlmm.lbPair.binStep);

  console.log("\nActive ID:");
  console.log(dlmm.lbPair.activeId);

  const activeBin = await dlmm.getActiveBin();

  console.log("\nActive bin:");
  console.log({
    binId: activeBin.binId,
    price: activeBin.price,
    pricePerToken: activeBin.pricePerToken,
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
