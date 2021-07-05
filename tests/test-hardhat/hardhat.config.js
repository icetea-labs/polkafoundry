require("@nomiclabs/hardhat-waffle");
require('dotenv').config()

// Go to https://www.alchemyapi.io, sign up, create
// a new App in its dashboard, and replace "KEY" with its key
// Replace this private key with your Ropsten account private key
// To export your private key from Metamask, open Metamask and
// go to Account Details > Export Private Key
// Be aware of NEVER putting real Ether into testing accounts

module.exports = {
  solidity: "0.7.3",
  networks: {
    halongbay: {
      url: process.env.RPC_URL,
      chainId: 11,
      accounts: [`${process.env.GENESIS_ACCOUNT_PRIVATE_KEY}`]
    }
  }
};