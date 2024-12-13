// SPDX-License-Identifier: MIT

pragma solidity ^0.6.12;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/ERC20Burnable.sol";

/// @title Burnable and Mintable ERC20 Token for Testing
/// @dev Allows any address to mint tokens (intended for testing purposes only)
contract Lit is ERC20, ERC20Burnable {
    /// @notice Constructor to initialize the token
    /// @param name_ Name of the token
    /// @param symbol_ Symbol of the token
    /// @param initialSupply Initial supply of tokens to mint to the deployer
    constructor(string memory name_, string memory symbol_, uint256 initialSupply) public ERC20(name_, symbol_) {
        _mint(msg.sender, initialSupply); // Mint the initial supply to the deployer
    }

    /// @notice Mint new tokens
    /// @dev This function allows any address to mint tokens (for testing purposes only)
    /// @param to The address to receive the minted tokens
    /// @param amount The number of tokens to mint
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}
