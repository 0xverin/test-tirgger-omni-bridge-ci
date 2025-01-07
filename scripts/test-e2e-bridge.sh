#!/bin/bash

TARGET_DIR="./target/release"
BINARY_NAME="bridge-cli"
BINARY_PATH="$TARGET_DIR/$BINARY_NAME"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Binary $BINARY_NAME not found. Building..."
    
    cargo build -p bridge-cli --release

    # Check if the build was successful
    if [ $? -ne 0 ]; then
        echo "Build failed. Exiting."
        exit 1
    fi
fi

CLI="$BINARY_PATH"

echo "CLI binary path: $CLI"

echo "Setting up chainbridge for dev testing using CLI" 
$CLI substrate setup-dev-chain-bridge 
echo "Finished setting up chainbridge on Parachain" 

# This is a test private_key for local e2e test
export PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

echo "Deploying contracts on Anvil node" 
cd ethereum/chainbridge-contracts
forge script script/all-contracts.s.sol:DeployAllContracts --rpc-url http://localhost:8545 --broadcast
echo "Completed Deploying contracts" 

cd .. 
cd .. 

echo "$(pwd)"
echo "Waiting for Deposit..." 
sleep 24

# This should be 10_000_000_000_000_000_000 if bridge works 
$CLI substrate balance esrJNKDP4tvAkGMC9Su2VYTAycU2nrQy8qt4dFhdXwV19Yh1K

echo "Perform Withdraw Operation" 
$CLI substrate dev-test-withdraw




