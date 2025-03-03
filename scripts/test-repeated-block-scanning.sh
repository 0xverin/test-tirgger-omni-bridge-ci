#!/bin/bash
set -euo pipefail

# we assume the environment is already intialised by `docker compose up`
# TODO: add check

ROOTDIR=$(git rev-parse --show-toplevel)
cd "$ROOTDIR"
cargo b -p bridge-cli

CLI=./target/debug/bridge-cli

$CLI --version

echo "Force recreating the omni-bridge container..." 
docker compose -f docker/chains.yml -f docker/deployers.yml -f docker/explorer.yml -f docker/omni-bridge.yml up -d --force-recreate --no-deps omni-bridge 
echo "sleeping for 60s.."
sleep 60

echo "look for failed extrinsics in heima.." 
r=$($CLI substrate failed-bridge-tx)
if [ $r = "2" ]; then
  echo "2 extrinsic failed in substrate; ok"
else
  echo "nok: $r"
  exit 1
fi


echo "check if the balance remains unchanged on Ethereum.."
r=$($CLI ethereum balance --account 0x70997970C51812dc3A010C7d01b50e0d17dc79C8)
if [ $r = "100000000000000000000" ]; then
  echo "balance ok"
else
  echo "nok: $r"
  exit 1
fi

echo "check if balance remains unchanged on Heima.."
r=$($CLI substrate balance --account 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty)
# Bob should have 1100 LIT now: 1000 from genesis + 100 bridged
if [ $r = "1200000000000000000000" ]; then
  echo "balance ok"
else
  echo "nok: $r"
  exit 1
fi

echo "bridge 100 LIT from heima to ethereum-2 node, it should be relayed despite previous events were relayed"
RUST_LOG=info $CLI substrate pay-in --dest-address 70997970C51812dc3A010C7d01b50e0d17dc79C8 --amount 100000000000000000000 --ethereum-id 56

echo "wait for 18s ..."
sleep 18

echo "check if the bridge was ok ..."
r=$($CLI ethereum balance --account 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 --port 8546)
if [ $r = "200000000000000000000" ]; then
  echo "balance ok"
  exit 0
else
  echo "nok: $r"
  exit 1
fi