import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import DLMM, { StrategyType } from "@meteora-ag/dlmm";
import BN from "bn.js";
import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

const RPC_URL = "https://api.devnet.solana.com";

const POOL = new PublicKey("53vdaEoUXhhTXGM9odCYVdvGYzLJnSMXovX5MX9E3xnU");

const EXPECTED_TOKEN_X = new PublicKey(
  "ArzQGNtfXXQSLcdtVrJPu1CuJZHTiejL55hfum35FsKv"
);

const EXPECTED_TOKEN_Y = new PublicKey(
  "9nPD667QBEV9rzmySdAqttPM5aTQWkjFH3R6JLhhF734"
);

// Raw token amounts.
//
// X = Demo ANTH, 9 decimals
// 20 * 10^9 = 20 ANTH
//
// Y = Demo USDC, 6 decimals
// 20,000 * 10^6 = 20,000 USDC
const TOTAL_X_AMOUNT = new BN("20000000000");
const TOTAL_Y_AMOUNT = new BN("20000000000");

// Concentrate liquidity around the current active bin.
//
// Pool:
//   active bin = 0
//   bin step   = 10
//
// This gives us a compact demo liquidity range around ~$1,000.
const MIN_BIN_ID = -10;
const MAX_BIN_ID = 10;

function loadAuthority(): Keypair {
  const keypairPath = path.join(os.homedir(), ".config", "solana", "id.json");

  const secret = JSON.parse(fs.readFileSync(keypairPath, "utf8")) as number[];

  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");
  const authority = loadAuthority();

  console.log("\n=== Osprey Meteora Liquidity Seeder ===\n");

  console.log("Authority:");
  console.log(authority.publicKey.toBase58());

  console.log("\nPool:");
  console.log(POOL.toBase58());

  const dlmm = await DLMM.create(connection, POOL, {
    cluster: "devnet",
  });

  console.log("\nToken X:");
  console.log(dlmm.tokenX.publicKey.toBase58());

  console.log("\nToken Y:");
  console.log(dlmm.tokenY.publicKey.toBase58());

  if (!dlmm.tokenX.publicKey.equals(EXPECTED_TOKEN_X)) {
    throw new Error(`Unexpected token X: ${dlmm.tokenX.publicKey.toBase58()}`);
  }

  if (!dlmm.tokenY.publicKey.equals(EXPECTED_TOKEN_Y)) {
    throw new Error(`Unexpected token Y: ${dlmm.tokenY.publicKey.toBase58()}`);
  }

  const activeBin = await dlmm.getActiveBin();

  console.log("\nCurrent active bin:");
  console.log(activeBin.binId);

  console.log("\nCurrent price:");
  console.log(activeBin.pricePerToken);

  console.log("\nLiquidity amounts:");
  console.log("X: 20 Demo ANTH");
  console.log("Y: 20,000 Demo USDC");

  console.log("\nBin range:");
  console.log(`${MIN_BIN_ID} -> ${MAX_BIN_ID}`);

  const position = Keypair.generate();

  console.log("\nNew liquidity position:");
  console.log(position.publicKey.toBase58());

  const transaction = await dlmm.initializePositionAndAddLiquidityByStrategy({
    positionPubKey: position.publicKey,

    totalXAmount: TOTAL_X_AMOUNT,
    totalYAmount: TOTAL_Y_AMOUNT,

    strategy: {
      minBinId: MIN_BIN_ID,
      maxBinId: MAX_BIN_ID,
      strategyType: StrategyType.Spot,
    },

    user: authority.publicKey,

    slippage: 1,
  });

  console.log("\nTransaction built.");
  console.log("Sending liquidity transaction...");

  const signature = await sendAndConfirmTransaction(
    connection,
    transaction,
    [authority, position],
    {
      commitment: "confirmed",
    }
  );

  console.log("\n================================");
  console.log("LIQUIDITY SEEDED");
  console.log("================================");

  console.log("\nPosition:");
  console.log(position.publicKey.toBase58());

  console.log("\nTransaction:");
  console.log(signature);

  fs.mkdirSync(".demo", {
    recursive: true,
  });

  fs.writeFileSync(
    ".demo/meteora-position.json",
    JSON.stringify(
      {
        pool: POOL.toBase58(),
        position: position.publicKey.toBase58(),
        transaction: signature,
        tokenX: EXPECTED_TOKEN_X.toBase58(),
        tokenY: EXPECTED_TOKEN_Y.toBase58(),
        amountX: TOTAL_X_AMOUNT.toString(),
        amountY: TOTAL_Y_AMOUNT.toString(),
        minBinId: MIN_BIN_ID,
        maxBinId: MAX_BIN_ID,
      },
      null,
      2
    )
  );

  console.log("\nSaved position metadata to .demo/meteora-position.json");
}

main().catch((error) => {
  console.error("\nLiquidity seeding failed:\n");
  console.error(error);
  process.exit(1);
});
