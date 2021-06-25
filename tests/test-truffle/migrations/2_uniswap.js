const jsonFactory = require('@uniswap/v2-core/build/UniswapV2Factory.json')
const jsonRouter02 = require('@uniswap/v2-periphery/build/UniswapV2Router02.json');
const contract = require('@truffle/contract');
const UniswapV2Factory = contract(jsonFactory);
const UniswapV2Router02 = contract(jsonRouter02);
const HDWalletProvider = require('@truffle/hdwallet-provider');
const Web3 = require('web3');

const provider = (process.env.RPC || '').indexOf('127.0.0.1') >=0 ? new Web3.providers.HttpProvider(process.env.RPC) : new HDWalletProvider(process.env.PRIVATEKEY, process.env.RPC);
const WETH = artifacts.require("WETH9");

UniswapV2Factory.setProvider(provider);
UniswapV2Router02.setProvider(provider);

module.exports = async function(_deployer, network, accounts) {
    await _deployer.deploy(WETH)
    await _deployer.deploy(UniswapV2Factory, accounts[0], {from: accounts[0]});
    console.log('UniswapV2Factory adddress', UniswapV2Factory.address);
    await _deployer.deploy(UniswapV2Router02, UniswapV2Factory.address, WETH.address, {from: accounts[0]})
    console.log('UniswapV2Router address', UniswapV2Router02.address);
};
