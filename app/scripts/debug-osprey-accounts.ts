import {
  BorshAccountsCoder,
  Idl,
} from "@coral-xyz/anchor";
import {
  Connection,
  PublicKey,
} from "@solana/web3.js";

import idl from "../src/lib/anchor/idl.json";

const connection = new Connection(
  "https://api.devnet.solana.com",
  "confirmed"
);

const market = new PublicKey(
  "Dsci5yxTLxHF6332xRc4gMEEsZxqjGMdQdv1GJ8WvNG"
);

const snapshot = new PublicKey(
  "2Nx8YXGyVUSceNcP1YgH141T5BS24QQqJxVPsSXjpvSf"
);

async function main() {
  const coder = new BorshAccountsCoder(
    idl as Idl
  );

  const marketInfo =
    await connection.getAccountInfo(market);

  const snapshotInfo =
    await connection.getAccountInfo(snapshot);

  if (!marketInfo) {
    throw new Error("Market account missing");
  }

  if (!snapshotInfo) {
    throw new Error("Snapshot account missing");
  }

  const decodedMarket = coder.decode(
    "MarketConfig",
    marketInfo.data
  );

  const decodedSnapshot = coder.decode(
    "LiquidityRiskSnapshot",
    snapshotInfo.data
  );

  console.log("\n=== MARKET ===");
  console.dir(decodedMarket, {
    depth: null,
  });

  console.log("\n=== MARKET KEYS ===");
  console.log(Object.keys(decodedMarket as object));

  console.log("\n=== SNAPSHOT ===");
  console.dir(decodedSnapshot, {
    depth: null,
  });

  console.log("\n=== SNAPSHOT KEYS ===");
  console.log(
    Object.keys(decodedSnapshot as object)
  );
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
