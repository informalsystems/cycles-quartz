// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "../src/openzeppelin/ERC20.sol";

/**
 * @title MockERC20
 * @dev ERC20 token with a public mint function that can be called by anyone.
 */
contract MockERC20 is ERC20 {
    /**
     * @dev Constructor that initializes the token with a name and symbol.
     * @param name_ The name of the token.
     * @param symbol_ The symbol of the token.
     */
    constructor(string memory name_, string memory symbol_) ERC20(name_, symbol_) {}

    /**
     * @dev Mints `amount` tokens to the specified `account`.
     * @param account The address to which the tokens will be minted.
     * @param amount The amount of tokens to mint.
     */
    function mint(address account, uint256 amount) external {
        _mint(account, amount);
    }
}
