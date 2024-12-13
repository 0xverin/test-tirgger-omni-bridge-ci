// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.6.12;

import {Script} from "forge-std/Script.sol";
import {Bridge} from "../src/Bridge.sol";


contract DeployBridgeScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        uint8 domainID = 0;
        address[] memory initialRelayers = new address[](1);
        initialRelayers[0] = 0x0e209C5dEdFfE34120679A681a0d7d21A360a97f;
        uint256 initialRelayerThreshold = 1;
        uint256 fee = 0;
        uint256 expiry = 0;

        Bridge bridge = new Bridge(domainID, initialRelayers, initialRelayerThreshold, fee, expiry);

        vm.stopBroadcast();
    }
}
