import DLMM from "@meteora-ag/dlmm";
import { Connection } from "@solana/web3.js";

const RPC_URL =
  process.env.NEXT_PUBLIC_SOLANA_RPC_URL ?? "https://api.devnet.solana.com";

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("RPC:", RPC_URL);
  console.log("Fetching Meteora DLMM devnet presets...\n");

  const presets = await DLMM.getAllPresetParameters(connection, {
    cluster: "devnet",
  });

  console.dir(presets, {
    depth: null,
    colors: true,
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
