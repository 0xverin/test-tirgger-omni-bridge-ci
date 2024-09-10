# Running locally

1. Build smart contract `make build-evm-contracts`
2. Build bridge docker image `make build-docker`
3. Build litentry parachain docker image from `bridge-pallet` branch
4. Run docker-compose `make start-local`

Ethereum and Substrate node can be access locally on ports 8545 and 9944 respectively. 

### Generating smart contract bindings

Smart contract bindings are used by CLI to interact with deployed smart contracts.

`forge bind --out artifacts/ethereum-contracts`


### Interacting with the bridge

Please refer to `bridge-cli` `README.md` for more inf how to complete setup (adding relayers etc.) and how to interact
with bridge.