Ethereum bridge contract needs to be built first in order to interact with its instance.

`cd ../ethereum/chainbridge-contracts/ && forge build`

# Bridging using CLI

Cli commands are divided into two groups [ethereum,substrate]. Pick one of them and execute `pay-in` command in order to trigger asset bridging.

For bridging substrate -> ethereum

1. Set up relayer on ethereum side: `./bridge-cli ethereum add-relayer --relayer_address 0x70997970c51812dc3a010c7d01b50e0d17dc79c8`
2. Set up bridge on substrate side: `./bridge-cli substrate setup-bridge`
2. Pay in from substrate: `./bridge-cli substrate pay-in --dest-address 70997970C51812dc3A010C7d01b50e0d17dc79C8 --amount 100000000000000000000`

Later you can query the HEI balance of dest-address by `./bridge-cli ethereum balance --account 0x70997970C51812dc3A010C7d01b50e0d17dc79C8`

For bridging ethereum -> substrate

1. Set up chainbridge contracts: `./bridge-cli ethereum setup-bridge`
2. Pay in from ethereum: `./bridge-cli ethereum pay-in --dest-address 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty --amount 100000000000000000000`

Later you should see `PaidOut` event emitted on substrate chain, and query the LIT balance of dest-address by `./bridge-cli substrate balance --account 5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty`
