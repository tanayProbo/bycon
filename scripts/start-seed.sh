#!/usr/bin/env bash
set -e

WALLET_FILE="founder-wallet.txt"

if [ ! -f "$WALLET_FILE" ]; then
  echo "Generating founder wallet..."
  ./target/release/byconn-cli wallet new | tee "$WALLET_FILE"
fi

COINBASE=$(grep -oE '0x[0-9a-fA-F]{40}' "$WALLET_FILE" | head -n 1)
if [ -z "$COINBASE" ]; then
  COINBASE="0x0000000000000000000000000000000000000001"
fi

echo "Starting BYCONN Global Seed Node & Miner with coinbase: $COINBASE"
nohup env BYCONN_COINBASE="$COINBASE" BYCONN_RPC_ADDR="0.0.0.0:8545" BYCONN_P2P_PORT="30303" ./target/release/node > byconn-node.log 2>&1 &
echo "BYCONN Node running in background (logs: byconn-node.log, RPC: :8545, P2P: :30303)"
