import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import DLMM from "@meteora-ag/dlmm";
import BN from "bn.js";
import {
  Connection,
  Keypair,
  PublicKey,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

const RPC_URL =
  process.env.NEXT_PUBLIC_SOLANA_RPC_URL ?? "https://api.devnet.solana.com";

const COLLATERAL_MINT = new PublicKey(
  "ArzQGNtfXXQSLcdtVrJPu1CuJZHTiejL55hfum35FsKv"
);

const DEBT_MINT = new PublicKey("9nPD667QBEV9rzmySdAqttPM5aTQWkjFH3R6JLhhF734");

const PRESET_PARAMETER = new PublicKey(
  "BB2atM1VveWJJUERbufE73fzZAss74J6DcEx1jGdTvwg"
);

const COLLATERAL_DECIMALS = 9;
const DEBT_DECIMALS = 6;

const TARGET_PRICE_USDC = 1_000;
const BIN_STEP = 10;

function loadAuthority(): Keypair {
  const keypairPath = path.join(os.homedir(), ".config", "solana", "id.json");

  const secret = JSON.parse(fs.readFileSync(keypairPath, "utf8")) as number[];

  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");
  const authority = loadAuthority();

  console.log("\n=== Osprey Meteora Pool Setup ===\n");

  console.log("RPC:");
  console.log(RPC_URL);

  console.log("\nAuthority:");
  console.log(authority.publicKey.toBase58());

  console.log("\nCollateral mint:");
  console.log(COLLATERAL_MINT.toBase58());

  console.log("\nDebt mint:");
  console.log(DEBT_MINT.toBase58());

  console.log("\nPreset:");
  console.log(PRESET_PARAMETER.toBase58());

  /*
   * Meteora works with price per smallest token unit.
   *
   * Human price:
   *
   *   1 Demo ANTH = 1,000 Demo USDC
   *
   * Demo ANTH decimals = 9
   * Demo USDC decimals = 6
   *
   * pricePerLamport:
   *
   *   1000 * 10^(6 - 9)
   * = 1
   */
  const pricePerLamport = DLMM.getPricePerLamport(
    COLLATERAL_DECIMALS,
    DEBT_DECIMALS,
    TARGET_PRICE_USDC
  );

  const activeIdNumber = DLMM.getBinIdFromPrice(
    pricePerLamport,
    BIN_STEP,
    false
  );

  const activeId = new BN(activeIdNumber);

  console.log("\nTarget human price:");
  console.log(`1 Demo ANTH = ${TARGET_PRICE_USDC} Demo USDC`);

  console.log("\nPrice per lamport:");
  console.log(pricePerLamport);

  console.log("\nActive bin ID:");
  console.log(activeId.toString());

  const balance = await connection.getBalance(authority.publicKey);

  console.log("\nAuthority balance:");
  console.log(`${balance / 1_000_000_000} SOL`);

  console.log("\nBuilding Meteora createLbPair2 transaction...");

  const transaction = await DLMM.createLbPair2(
    connection,
    authority.publicKey,
    COLLATERAL_MINT,
    DEBT_MINT,
    PRESET_PARAMETER,
    activeId,
    {
      cluster: "devnet",
    }
  );

  console.log("Sending transaction...");

  const signature = await sendAndConfirmTransaction(
    connection,
    transaction,
    [authority],
    {
      commitment: "confirmed",
    }
  );

  console.log("\n================================");
  console.log("METEORA POOL CREATED");
  console.log("================================");

  console.log("\nTransaction:");
  console.log(signature);

  console.log(
    "\nPool creation succeeded. Next we will derive/read the created LB pair address."
  );
}

main().catch((error) => {
  console.error("\nPool creation failed:\n");
  console.error(error);
  process.exit(1);
});
