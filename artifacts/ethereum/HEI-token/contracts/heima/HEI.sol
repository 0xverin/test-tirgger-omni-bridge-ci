// SPDX-License-Identifier: MIT
// Heima Contracts

pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/access/extensions/AccessControlDefaultAdminRules.sol";

import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

contract HEI is ERC20Burnable, AccessControlDefaultAdminRules {
    address public constant BLACK_HOLE_ADDRESS =
        0x000000000000000000000000000000000000dEaD;
    bytes32 public constant MINT_ROLE = keccak256(abi.encode("MINTER"));

    IERC20 private immutable _underlying;

    /**
     * @dev The underlying token couldn't be wrapped.
     */
    error ERC20InvalidUnderlying(address token);

    /**
     * @dev The new minter is not a valid minter.
     */
    error AccessControlInvalidMinter(address minter);

    constructor(
        IERC20 underlyingToken,
        string memory _name,
        string memory _symbol,
        address _admin
    ) ERC20(_name, _symbol) AccessControlDefaultAdminRules(0, _admin) {
        if (underlyingToken == this) {
            revert ERC20InvalidUnderlying(address(this));
        }
        require(
            address(underlyingToken) != address(0),
            "Invalid underlyingToken address"
        );
        require(
            address(underlyingToken) != BLACK_HOLE_ADDRESS,
            "Invalid underlyingToken address"
        );

        _underlying = underlyingToken;
        _setRoleAdmin(MINT_ROLE, DEFAULT_ADMIN_ROLE);
    }

    /**
     * @dev See {ERC20-decimals}.
     */
    function decimals() public view virtual override returns (uint8) {
        return super.decimals();
    }

    /**
     * @dev Returns the address of the underlying ERC-20 token that is being wrapped.
     */
    function underlying() external view returns (IERC20) {
        return _underlying;
    }

    /**
     * @dev Allow a user to deposit underlying tokens and mint the corresponding number of wrapped tokens.
     */
    function depositFor(
        address account,
        uint256 value
    ) external returns (bool) {
        address sender = _msgSender();
        if (sender == address(this)) {
            revert ERC20InvalidSender(address(this));
        }
        if (account == address(this)) {
            revert ERC20InvalidReceiver(account);
        }

        SafeERC20.safeTransferFrom(
            _underlying,
            sender,
            BLACK_HOLE_ADDRESS,
            value
        );
        _mint(account, value);
        return true;
    }

    /**
     * @dev Mint token for cross-chain transfer
     */
    function mint(
        address account,
        uint256 value
    ) external onlyRole(MINT_ROLE) returns (bool) {
        _mint(account, value);
        return true;
    }

    /**
     * @dev grant a minter
     */
    function grantMinter(
        address _minter
    ) external onlyRole(DEFAULT_ADMIN_ROLE) returns (bool) {
        if (_minter == address(0)) {
            revert AccessControlInvalidMinter(address(0));
        }
        return _grantRole(MINT_ROLE, _minter);
    }
}
