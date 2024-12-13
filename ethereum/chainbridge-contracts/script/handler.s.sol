// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.6.12;

import {Script} from "forge-std/Script.sol";
import {ERC20Handler} from "../src/handlers/ERC20Handler.sol";

// 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
contract DeployHandlerScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        address bridge = 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512;

        bytes32[] memory initialResourceIds;
        address[] memory initialContractAddresses;
        address[] memory initialBurnableAddresses;
        ERC20Handler handler = new ERC20Handler(address(bridge), initialResourceIds, initialContractAddresses, initialBurnableAddresses);

        vm.stopBroadcast();
    }
}