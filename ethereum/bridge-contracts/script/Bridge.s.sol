// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Script, console} from "forge-std/Script.sol";
import {Bridge} from "../src/Bridge.sol";

contract BridgeScript is Script {
    Bridge public bridge;

    function setUp() public {}

    function run() public {
        vm.startBroadcast();

        address tokenAddress = 0x5FbDB2315678afecb367f032d93F642f64180aa3;

        bridge = new Bridge(tokenAddress);

        vm.stopBroadcast();
    }
}
