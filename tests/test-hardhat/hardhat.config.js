require("@nomiclabs/hardhat-waffle");

// Go to https://www.alchemyapi.io, sign up, create
// a new App in its dashboard, and replace "KEY" with its key
// Replace this private key with your Ropsten account private key
// To export your private key from Metamask, open Metamask and
// go to Account Details > Export Private Key
// Be aware of NEVER putting real Ether into testing accounts
const HLB_PRIVATE_KEY = "f20d960bfe1c2810c8e825a7f66b1cd27ac5a0ad045ed47965e5ba10aec3f438";

module.exports = {
  solidity: "0.7.3",
  networks: {
    hlb: {
      url: `https://rpc-halongbay.polkafoundry.com`,
      chainId: 11,
      accounts: [`0x${HLB_PRIVATE_KEY}`]
    }
  }
};