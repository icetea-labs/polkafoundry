// SPDX-License-Identifier: MIT
pragma solidity 0.6.6;


// Import OpenZeppelin Contract
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

// This ERC-20 contract mints the specified amount of tokens to the contract creator.
contract TeaERC20 is ERC20 {
    constructor(uint256 initialSupply) ERC20("Tea", "TEA") public
    {
        _mint(msg.sender, initialSupply);
    }
}
