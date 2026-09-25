export function getTransactionErrorMessage(error: unknown): string {
  if (!(error instanceof Error)) {
    return "The transaction could not be completed. Please try again.";
  }

  const message = error.message.toLowerCase();
  const name = error.name.toLowerCase();

  if (
    name.includes("walletsigntransactionerror") ||
    message.includes("user rejected") ||
    message.includes("rejected the request")
  ) {
    return "Transaction cancelled or rejected by your wallet.";
  }

  if (
    message.includes("insufficient funds") ||
    message.includes("insufficient token") ||
    message.includes("insufficient balance")
  ) {
    return "You do not have enough tokens to complete this transaction.";
  }

  if (
    message.includes("staleliquidityquote") ||
    message.includes("stale liquidity") ||
    message.includes("liquidity quote has expired")
  ) {
    return "The liquidity quote has expired. Refresh the market risk snapshot and try again.";
  }

  if (
    message.includes("borrowexceedsltv") ||
    message.includes("borrow exceeds")
  ) {
    return "This borrow would exceed the market's current liquidity-adjusted borrowing limit.";
  }

  if (
    message.includes("withdrawalviolatesltv") ||
    message.includes("withdrawal violates")
  ) {
    return "This withdrawal would leave your position above the allowed borrowing limit.";
  }

  if (
    message.includes("insufficientdebtliquidity") ||
    message.includes("insufficient debt liquidity")
  ) {
    return "The market does not currently have enough USDC liquidity for this borrow.";
  }

  if (
    message.includes("positionhealthy") ||
    message.includes("position healthy")
  ) {
    return "This position is healthy and cannot be liquidated.";
  }

  if (
    message.includes("liquidationexceedscap") ||
    message.includes("liquidation exceeds")
  ) {
    return "The requested liquidation exceeds the market's liquidation limit.";
  }

  if (message.includes("blockhash")) {
    return "The transaction expired before confirmation. Please try again.";
  }

  if (message.includes("invalidamount") || message.includes("invalid amount")) {
    return "Enter a valid amount greater than zero.";
  }

  if (
    message.includes("insufficientcollateral") ||
    message.includes("insufficient collateral")
  ) {
    return "You do not have enough deposited collateral for this action.";
  }

  if (message.includes("you do not have a demo usdc")) {
    return "You do not have Demo USDC available to repay this position.";
  }

  if (
    message.includes("fetch failed") ||
    message.includes("failed to fetch") ||
    message.includes("network") ||
    message.includes("timeout")
  ) {
    return "The Solana network connection failed. Please try again.";
  }

  return "Transaction failed. Please try again or check your wallet for more details.";
}
