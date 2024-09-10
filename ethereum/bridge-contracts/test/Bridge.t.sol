// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {Bridge} from "../src/Bridge.sol";
import {TestToken} from "../src/Token.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/access/IAccessControl.sol";

contract BridgeTest is Test {
    Bridge public bridge;
    TestToken public testToken;

    function setUp() public {
        testToken = new TestToken(10000);
        bridge = new Bridge(address(testToken));
        testToken.transfer(address(bridge), 100);
        testToken.approve(address(bridge), 100);
    }

    function test_it_should_pay_in() public {
        vm.expectEmit(true, true, false, true);
        emit Bridge.PaidIn(10, bytes("test"));

        bridge.payIn(10, bytes("test"));

        // 100 initial bridge tokens + 10 deposited
        assertEq(110, testToken.balanceOf(address(bridge)));
    }

    function test_it_should_revert_pay_in_if_paused() public {
        bridge.pause();
        vm.expectRevert(Pausable.EnforcedPause.selector);

        bridge.payIn(10, bytes("test"));
    }

    function test_it_should_pay_out() public {
        bridge.addRelayer(address(this));
        vm.expectEmit(true, true, false, true);
        emit Bridge.PaidOut(10, 0x70997970C51812dc3A010C7d01b50e0d17dc79C8);

        bridge.payOut(10, 0x70997970C51812dc3A010C7d01b50e0d17dc79C8);

        assertEq(10, testToken.balanceOf(0x70997970C51812dc3A010C7d01b50e0d17dc79C8));
    }

    function test_it_should_revert_pay_out_if_not_relayer() public {
        //todo: cheatcode not released yet
        //vm.expectPartialRevert(IAccessControl.AccessControlUnauthorizedAccount.selector);
        vm.expectRevert();

        bridge.payOut(10, 0x70997970C51812dc3A010C7d01b50e0d17dc79C8);
    }

    function test_it_should_add_relayer() public {
        vm.expectEmit(true, true, false, true);
        emit Bridge.RelayerAdded(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);

        bridge.addRelayer(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);

        assert(bridge.hasRole(keccak256("RELAYER"), 0x70997970C51812dc3A010C7d01b50e0d17dc79C8));
    }

    function test_it_should_reject_relayer_add_for_non_admin_caller() public {
        //todo: cheatcode not released yet
        //vm.expectPartialRevert(Bridge.RelayerAdded.selector);
        vm.expectRevert();
        vm.prank(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);

        bridge.addRelayer(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);
    }

    function test_it_should_remove_relayer() public {
        bridge.addRelayer(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);
        vm.expectEmit(true, true, false, true);
        emit Bridge.RelayerRemoved(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);

        bridge.removeRelayer(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);

        assert(!bridge.hasRole(keccak256("RELAYER"), 0x70997970C51812dc3A010C7d01b50e0d17dc79C8));
    }

    function test_it_should_reject_relayer_remove_for_non_admin_caller() public {
        //todo: cheatcode not released yet
        //vm.expectPartialRevert(Bridge.RelayerRemoved.selector);
        vm.expectRevert();
        vm.prank(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);

        bridge.removeRelayer(0x70997970C51812dc3A010C7d01b50e0d17dc79C8);
    }
}
