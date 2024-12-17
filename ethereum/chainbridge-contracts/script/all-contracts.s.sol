// This script deploys all the contracts, i.e. Bridge, Handler and Token 
// So steps for deployment would include the following 
// 1. Deploy the bridge contract 
// 2. Deploy the handler contract 
// 3. Deploy the token contract 
// 4. Set the minter role to the handler 
// 5. Set the admin resource 
// 6. Set the contract as a burnable resource
// 7. Perform a deposit 
// 8. Perform a withdraw from the relayer 

// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.6.12;

import {Script} from "forge-std/Script.sol";
import {Bridge} from "../src/Bridge.sol";
import {ERC20Handler} from "../src/handlers/ERC20Handler.sol";
import {Lit} from "../src/Token.sol";
import "forge-std/console.sol";

// forge script script/all-contracts.s.sol:DeployAllContracts --rpc-url http://localhost:8545 --broadcast

contract DeployAllContracts is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        uint8 domainID = 0;
        address[] memory initialRelayers = new address[](1);
        initialRelayers[0] = 0x0e209C5dEdFfE34120679A681a0d7d21A360a97f;
        uint256 initialRelayerThreshold = 1;
        uint256 fee = 0;
        uint256 expiry = 0;
        bytes32 resource_id = keccak256(abi.encodePacked("bridgeTransfer.transfer"));


        // Deploy the bridge contract 
        Bridge bridge = new Bridge(domainID, initialRelayers, initialRelayerThreshold, fee, expiry);
        console.log("Bridge contract deployed at address:", address(bridge));

        // Deploy the handler contract 
        bytes32[] memory initialResourceIds;
        address[] memory initialContractAddresses;
        address[] memory initialBurnableAddresses;
        ERC20Handler handler = new ERC20Handler(address(bridge), initialResourceIds, initialContractAddresses, initialBurnableAddresses);
        console.log("Handler contract deployed at address:", address(handler));


        // Deploy the token contract 
        Lit token = new Lit("Litentry", "LIT", 10000 * 10 ** 18);

        address token_address = address(token);
        address handler_address = address(handler);        
        // Admin add resource 
        bridge.adminSetResource(handler_address, resource_id, token_address);

        // Set the contract as burnable 
        bridge.adminSetBurnable(handler_address, token_address);

        // Give an allowance to the handler
        token.approve(address(handler), 100 * 10 ** 18);

        // Perform a deposit 
        bytes memory data = abi.encodePacked(
            uint256(10 * 10 ** 18),     // Amount
            uint256(32),                     // Length of recipient address
            bytes32(0xf4ccccb07c01d5c00e4e6135b6ff83c26d06dea0addf6fda925564b140e97976) // sr25519 address
        );

        bridge.deposit(1, resource_id, data);

        vm.stopBroadcast();
    }
}
