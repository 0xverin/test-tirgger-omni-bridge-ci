// Copyright 2020-2024 Trust Computing GmbH.
// This file is part of Litentry.
//
// Litentry is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Litentry is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Litentry.  If not, see <https://www.gnu.org/licenses/>.

pragma solidity >=0.8.2 <0.9.0;

import {AccessControl} from "@openzeppelin/contracts/access/AccessControl.sol";
import "@openzeppelin/contracts/utils/Pausable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract Bridge is Pausable, AccessControl {
    bytes32 public constant ADMIN_ROLE = keccak256("ADMIN");
    bytes32 public constant RELAYER_ROLE = keccak256("RELAYER");

    address immutable _tokenContractAddress;

    event PaidIn(uint256 value, bytes callData);
    event PaidOut(uint256 value, address recipient);
    event RelayerAdded(address relayer);
    event RelayerRemoved(address relayer);

    constructor(address tokenContractAddress) {
        _tokenContractAddress = tokenContractAddress;
        _grantRole(ADMIN_ROLE, msg.sender);
        _grantRole(DEFAULT_ADMIN_ROLE, msg.sender);
    }

    // Used to transfer tokens to the other end of the bridge. Event is listened by relayer
    function payIn(uint256 value, bytes memory callData) public whenNotPaused {
        IERC20 erc20 = IERC20(_tokenContractAddress);
        erc20.transferFrom(msg.sender, address(this), value);
        emit PaidIn(value, callData);
    }

    // Used to transfer tokens from the other end of the bridge. Called by relayers
    function payOut(uint256 value, address recipient) public whenNotPaused onlyRole(RELAYER_ROLE) {
        IERC20 erc20 = IERC20(_tokenContractAddress);
        erc20.transfer(recipient, value);
        emit PaidOut(value, recipient);
    }

    function addRelayer(address relayer) public onlyRole(ADMIN_ROLE) {
        _grantRole(RELAYER_ROLE, relayer);
        emit RelayerAdded(relayer);
    }

    function removeRelayer(address relayer) public onlyRole(ADMIN_ROLE) {
        _revokeRole(RELAYER_ROLE, relayer);
        emit RelayerRemoved(relayer);
    }

    function pause() public {
        _pause();
    }
}
