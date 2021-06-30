require('../config');
const Web3 = require('web3');
const { expect } = require('chai');
const contractObj = require('../build/contracts/Storage.json');
const { deployContract, callMethod } = require('../utils');

const RPC = process.env.RPC;
const OTHER_ACCOUNT = process.env.OTHER_ACCOUNT;
const OTHER_ACCOUNT_PRIVATE_KEY = process.env.OTHER_ACCOUNT_PRIVATE_KEY;
const value = 1235446; // uint256
const otherValue = 123544654634563; // uint256

describe("Contract Storage", () => {
    const web3 = new Web3(RPC);
    const abi = contractObj.abi;
    let contractAddress = null;

    it('Deploy contract', async () => {
        const contract = await deployContract(web3, contractObj, []);
        contractAddress = contract.contractAddress;
        expect(contractAddress).to.match(/^0x[0-9A-Za-z]{40}$/);
    });

    it('Call method store()', async () => {
        const incrementer = new web3.eth.Contract(abi);
        const encoded = incrementer.methods.store(value).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it('Call method retrieve()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.retrieve().call();
        expect(data).to.equal(value.toString());
    });

    it('Call method store() by other account', async () => {
        const incrementer = new web3.eth.Contract(abi);
        const encoded = incrementer.methods.store(otherValue).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded, OTHER_ACCOUNT, OTHER_ACCOUNT_PRIVATE_KEY);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it('Call method retrieve() after other account storage', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.retrieve().call();
        expect(data).to.equal(otherValue.toString());
    });
});
