# Background

To align with the rebranding, we are going to change our token ticker from `LIT` to `HEI`. While we'll have most HEI tokens on parachain, we still want to have an ERC20/BEP20 contract for use-cases like DEX on ETH/BSC, respectively.

Unlike the most traditional ERC20 contract where you specify a fixed total_supply , we want a more flexible one that:

- allows minting and burning: only possible by bridging from/to parachain

- allows swapping LIT for HEI 1:1

- can query total_supply, the initial total_supply for ERC20/BEP20 token contract will be 0 without any bridging or swap

# Contract functions

## Swap LIT for HEI

`depositFor` can be called by users to swap LIT for HEI, the precondition is that the HEI contract is allowed to spend x LIT from users (set in LIT contract), where x is at least the amount of LIT the user wants to swap.

## Role-based access control

### MINT_ROLE

Admin can use `grantMinter` to grant an address MINT_ROLE, who can then mint tokens. In practice only the bridge contract should be granted this role.

### Admin ownership transfer

1. `beginDefaultAdminTransfer()` by the current admin
2. `acceptDefaultAdminTransfer()` by the new admin

## Unit test

Install `node` and `hardhat`, and then:

```bash
pnpm add -D hardhat
pnpm run test
```
