// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.6.12;

import {Script} from "forge-std/Script.sol";
import {Lit} from "../src/Token.sol";

// 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
contract DeployTokenScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        Lit token = new Lit("Litentry", "LIT", 10000000000000000000000000);

        vm.stopBroadcast();
    }
}