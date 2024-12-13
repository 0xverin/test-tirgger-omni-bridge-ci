import {Script} from "forge-std/Script.sol";
import {Bridge} from "../src/Bridge.sol";


contract PerformDeposit is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        Bridge bridge = Bridge(0x5FbDB2315678afecb367f032d93F642f64180aa3);

        bytes32 resource_id = keccak256(abi.encodePacked("bridgeTransfer.transfer"));

        bytes memory data = abi.encodePacked(
            uint256(10 * 10 ** 18),     // Amount
            uint256(32),                     // Length of recipient address
            bytes32(0xf4ccccb07c01d5c00e4e6135b6ff83c26d06dea0addf6fda925564b140e97976) // sr25519 address
        );

        bridge.deposit(1, resource_id, data);

        vm.stopBroadcast();
    }
}