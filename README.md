# Running locally

1. Build smart contract `make build-evm-contracts`
2. Build bridge docker image `make build-docker-dev`
3. Run docker-compose `make start-local`
4. (optional) Run local e2e test `make start-local-e2e-test`

Ethereum and Substrate node can be accessed locally on ports 8545 and 9944, respectively.

### Generating smart contract bindings

Smart contract bindings are used by CLI to interact with deployed smart contracts.

`forge bind --out artifacts/ethereum-contracts`


### Interacting with the bridge

Please refer to `bridge-cli` `README.md` for more inf how to complete setup (adding relayers etc.) and how to interact
with bridge.
An example can be found in `scripts/test-e2e-bridge.sh`.


### Preparing Gramine docker image

1. Create `config.json` file in the root of the repository. This file should contain bridge configuration (see `local/config.json`)
2. Create `auth_key_pub.bin` file containing keystore importer public key.
3. Build docker image `docker build --secret id=signer,src=<path to enclave signing key> -f Dockerfile.gramine --tag litentry/omni-bridge:latest .`