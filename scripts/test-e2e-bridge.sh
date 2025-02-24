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

echo "adds relayer to ethereum-2 chain bridge ..."
RUST_LOG=info $CLI ethereum add-relayer --port 8546

echo "set up ethereum bridge ..."
RUST_LOG=info $CLI ethereum setup-bridge

echo "set up ethereum-2 bridge ..."
RUST_LOG=info $CLI ethereum setup-bridge --port 8546

echo "set up substrate bridge ..."
RUST_LOG=info $CLI substrate setup-bridge

echo "bridge 100 LIT from heima to ethereum node ..."
RUST_LOG=info $CLI substrate pay-in --dest-address 70997970C51812dc3A010C7d01b50e0d17dc79C8 --amount 100000000000000000000 --ethereum-id 00

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


echo "bridge 100 LIT from heima to ethereum-2 node ..."
RUST_LOG=info $CLI substrate pay-in --dest-address 70997970C51812dc3A010C7d01b50e0d17dc79C8 --amount 100000000000000000000 --ethereum-id 56

echo "wait for 18s ..."
sleep 18

echo "check if the bridge was ok ..."
r=$($CLI ethereum balance --account 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 --port 8546)
if [ $r = "100000000000000000000" ]; then
  echo "balance ok"
else
  echo "nok: $r"
  exit 1
fi



echo "bridge 100 HEI from ethereum to heima ..."
# use `//Bob` as recipient for a deterministic balance check, as Alice paied some tx fee for previou tx 
RUST_LOG=info $CLI ethereum pay-in --dest-address 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty --amount 100000000000000000000

echo "bridge 100 HEI from ethereum-2 to heima ..."
# use `//Bob` as recipient for a deterministic balance check, as Alice paied some tx fee for previou tx
RUST_LOG=info $CLI ethereum pay-in --dest-address 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty --amount 100000000000000000000 --port 8546

echo "wait for 30s ..."
sleep 60

echo "check if bridge was ok ..."
r=$($CLI substrate balance --account 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty)
# Bob should have 1100 LIT now: 1000 from genesis + 100 bridged
if [ $r = "1200000000000000000000" ]; then
  echo "balance ok"
  exit 0
else
  echo "nok: $r"
  exit 1
fi
