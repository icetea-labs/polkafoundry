require('../config');
const Web3 = require('web3');
const { expect } = require('chai');
const contractObj = require('../build/contracts/Multicall.json');
const { deployContract } = require('../utils');

const RPC = process.env.RPC;
const GENESIS_ACCOUNT = process.env.GENESIS_ACCOUNT;
let currentBlock = 0;

describe("Contract Multicall", () => {
    const web3 = new Web3(RPC);
    const abi = contractObj.abi;
    let contractAddress = null;

    it('Deploy contract', async () => {
        const contract = await deployContract(web3, contractObj, []);
        contractAddress = contract.contractAddress;
        expect(contractAddress).to.match(/^0x[0-9A-Za-z]{40}$/);
    });

    // it('Call method aggregate', async () => {
    //     const contract = new web3.eth.Contract(abi, contractAddress);
    //     const data = await contract.methods.aggregate([
    //         {
    //             target: '0x68F53108e500e8ddD05868E6c0947133d90Ac1AE', // address
    //             callData: 0, // bytes
    //         }, {
    //             target: '0xa99D94C2f8cAef073C3De43b19E5979cb028d9de',
    //             callData: 1,
    //         }, {
    //             target: '0x6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b',
    //             callData: 2,
    //         }, {
    //             target: '0x7CEdd3d04abD3f538232445f006ccAd2e5865E4F',
    //             callData: 3,
    //         }, {
    //             target: '0x449F4d71ea0acd1886e8F6EBeAAbbCe2514393ad',
    //             callData: 4,
    //         }
    //     ]).call();
    //     console.log(data);
    //     // expect(data).to.equal('1');
    // });

    it('Call method getEthBalance', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.getEthBalance(GENESIS_ACCOUNT).call();
        const balance = parseInt(data);
        expect(balance).to.above(0);
    });

    it('Call method getLastBlockNumber()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.getLastBlockNumber().call();
        currentBlock = parseInt(data);
        expect(currentBlock).to.above(0);
    });

    it('Call method getBlockHash()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.getBlockHash(currentBlock).call();
        expect(data).to.match(/^0x[0-9a-zA-Z]{64}$/);
    });

    it('Call method getLastBlockHash()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.getLastBlockHash().call();
        expect(data).to.match(/^0x[0-9a-zA-Z]{64}$/);
    });

    it('Call method getCurrentBlockTimestamp()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.getCurrentBlockTimestamp().call();
        expect(data).to.match(/^\d{10}$/);
    });

    it('Call method getCurrentBlockDifficulty()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.getCurrentBlockDifficulty().call();
        expect(data).to.match(/^\d+$/);
    });

    it('Call method getCurrentBlockGasLimit()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.getCurrentBlockGasLimit().call();
        expect(data).to.match(/^\d+$/);
    });

    it('Call method getCurrentBlockCoinbase()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.getCurrentBlockCoinbase().call();
        expect(data).to.match(/^0x[0-9a-zA-Z]{40}$/);
    });
});
