// SPDX-License-Identifier: MIT

pragma solidity ^0.8.20;

import {IERC20, IERC20Metadata, ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

contract ERC20Test is ERC20 {
    mapping(address => uint256) private _balances;
    mapping(address => mapping(address => uint256)) private _allowances;
    uint256 private _totalSupply;
    string private _name;
    string private _symbol;
    address private _owner;

    constructor(
        string memory name_,
        string memory symbol_,
        uint256 supply,
        address owner
    ) ERC20(name_, symbol_) {
        _name = name_;
        _symbol = symbol_;
        _totalSupply = supply;
        _balances[owner] = supply;
        _owner = owner;
    }

    modifier onlyOwner() {
        require(msg.sender == _owner, "Only the owner can mint");
        _;
    }

    function mint(address account, uint256 amount) external onlyOwner {
        _mint(account, amount);
    }
}
