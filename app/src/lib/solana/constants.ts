import { PublicKey } from "@solana/web3.js";

export const SOLANA_NETWORK = "devnet" as const;

export const SOLANA_RPC_URL =
  process.env.NEXT_PUBLIC_SOLANA_RPC_URL ??
  "https://api.devnet.solana.com";

export const OSPREY_PROGRAM_ID = new PublicKey(
  process.env.NEXT_PUBLIC_OSPREY_PROGRAM_ID ??
    "jAUJs14M4WiAvrgRgZiKQ2nunkWMrqqM7sGZotmWwss"
);

export const DEMO_ANTHROPIC_MINT = new PublicKey(
  process.env.NEXT_PUBLIC_DEMO_COLLATERAL_MINT ??
    "ArzQGNtfXXQSLcdtVrJPu1CuJZHTiejL55hfum35FsKv"
);

export const DEMO_USDC_MINT = new PublicKey(
  process.env.NEXT_PUBLIC_DEMO_DEBT_MINT ??
    "9nPD667QBEV9rzmySdAqttPM5aTQWkjFH3R6JLhhF734"
);

export const METEORA_DLMM_POOL = new PublicKey(
  process.env.NEXT_PUBLIC_METEORA_DLMM_POOL ??
    "53vdaEoUXhhTXGM9odCYVdvGYzLJnSMXovX5MX9E3xnU"
);

export const MARKET_SEED = Buffer.from("market");
export const POSITION_SEED = Buffer.from("position");
export const VAULT_SEED = Buffer.from("vault");
export const RISK_SNAPSHOT_SEED = Buffer.from("risk-snapshot");

export const COLLATERAL_DECIMALS = 9;
export const DEBT_DECIMALS = 6;
