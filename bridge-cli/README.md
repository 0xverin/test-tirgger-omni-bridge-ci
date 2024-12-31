Ethereum bridge contract needs to be built first in order to interact with it's instance.

`cd ../ethereum/bridge-contracts/ && forge build`

# Setting up local env


For bridging substrate -> ethereum

(asset handling on substrate side is missing - no tokens are taken)

1. set ethereum relayer with public key printed to logs `./bridge-cli ethereum add-relayer 0x70997970c51812dc3a010c7d01b50e0d17dc79c8`
2. fund bridge EoA with ERC20 tokens `./bridge-cli ethereum transfer 0x5FbDB2315678afecb367f032d93F642f64180aa3 10000000`
3. Pay in `./bridge-cli substrate pay-in 10`

For bridging ethereum -> substrate

(asset handling on substrate side is missing - no tokens are given)

1. Add substrate account 5C7C2Z5sWbytvHpuLTvzKunnnRwQxft1jiqrLD5rhucQ5S9X as Admin on PalletBridge using sudo call through polkadotjs
2. Add substrate relayer `./bridge-cli substrate add-relayer 5DFW6oheaiW3XMDaPFi7RYLsKdPimAaY8Ajz2zA6S4STHG1D`
3. Fund user account `./bridge-cli ethereum transfer 0x70997970C51812dc3A010C7d01b50e0d17dc79C8 10000`
4. Approve bridge account `./bridge-cli ethereum approve 0x5FbDB2315678afecb367f032d93F642f64180aa3 10000`
5. Pay in `./bridge-cli ethereum pay-in 10`
6. You should see `PaidOut` event emitted on substrate chain

# Bridging using CLI

Cli commands are divided into two groups [ethereum,substrate]. Pick one of them and execute `pay-in` command in order to trigger asset bridging.

### Example
Bridge tokens from substrate to the other end of the bridge (ethereum):

`RUST_LOG=info ./bridge-cli substrate pay-in 10`