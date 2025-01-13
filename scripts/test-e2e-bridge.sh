#!/bin/bash
set -euo pipefail

# we assume the environment is already intialised by `docker compose up`
# TODO: add check

ROOTDIR=$(git rev-parse --show-toplevel)
cd "$ROOTDIR"
cargo b -p bridge-cli

CLI=./target/debug/bridge-cli

$CLI --version

echo "adds relayer to ethereum chain bridge ..."
RUST_LOG=info $CLI ethereum add-relayer

echo "set up ethereum bridge ..."
RUST_LOG=info $CLI ethereum setup-bridge

echo "set up substrate bridge ..."
RUST_LOG=info $CLI substrate setup-bridge

echo "bridge 100 LIT from heima to eth ..."
RUST_LOG=info $CLI substrate pay-in --dest-address 70997970C51812dc3A010C7d01b50e0d17dc79C8 --amount 100000000000000000000

echo "wait for 18s ..." 
sleep 18

echo "check if the bridge was ok ..."
r=$($CLI ethereum balance --account 0x70997970C51812dc3A010C7d01b50e0d17dc79C8)
if [ $r = "100000000000000000000" ]; then
  echo "balance ok"
else
  echo "nok: $r"
  exit 1
fi

echo "bridge 100 HEI from eth to heima ..."
# use `//Bob` as recipient for a deterministic balance check, as Alice paied some tx fee for previou tx 
RUST_LOG=info $CLI ethereum pay-in --dest-address 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty --amount 100000000000000000000

echo "wait for 30s ..."
sleep 30

echo "check if bridge was ok ..."
r=$($CLI substrate balance --account 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty)
# Bob should have 1100 LIT now: 1000 from genesis + 100 bridged
if [ $r = "1100000000000000000000" ]; then
  echo "balance ok"
  exit 0
else
  echo "nok: $r"
  exit 1
fi
