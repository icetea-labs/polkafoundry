require('../config');
const Web3 = require('web3');
const { expect } = require('chai');
const contractObj = require('../build/contracts/Migrations.json');
const { deployContract, callMethod } = require('../utils');

const RPC = process.env.RPC;
const GENESIS_ACCOUNT = process.env.GENESIS_ACCOUNT;
const OTHER_ACCOUNT = process.env.OTHER_ACCOUNT;
const OTHER_ACCOUNT_PRIVATE_KEY = process.env.OTHER_ACCOUNT_PRIVATE_KEY;
const value = 1232132; // uint

describe("Contract Migrations", () => {
    const web3 = new Web3(RPC);
    const abi = contractObj.abi;
    let contractAddress = null;

    it('Deploy contract', async () => {
        const contract = await deployContract(web3, contractObj, []);
        contractAddress = contract.contractAddress;
        expect(contractAddress).to.match(/^0x[0-9A-Za-z]{40}$/);
    });

    it('Get owner from contract', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.owner().call();
        expect(data).to.equal(GENESIS_ACCOUNT);
    });

    it('Get last_completed_migration from contract', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.last_completed_migration().call();
        expect(data).to.equal('0');
    });

    it('Call method setCompleted() -> set last_completed_migration', async () => {
        const incrementer = new web3.eth.Contract(abi);
        const encoded = incrementer.methods.setCompleted(value).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it('Get last_completed_migration after set value', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.last_completed_migration().call();
        expect(data).to.equal(value.toString());
    });

    it('Call method setCompleted() by other address => not success', async () => {
        const incrementer = new web3.eth.Contract(abi);
        const encoded = incrementer.methods.setCompleted(value - 1).encodeABI();

        try {
            const callReceipt = await callMethod(web3, abi, contractAddress, encoded, OTHER_ACCOUNT, OTHER_ACCOUNT_PRIVATE_KEY);
            const transactionHash = callReceipt.transactionHash;
            expect(transactionHash).to.not.match(/^0x[0-9A-Za-z]{64}$/);
        } catch (e) {
            expect(e.message).to.equal('Returned error: VM Exception while processing transaction: revert This function is restricted to the contract\'s owner');
        }
    });

    it('Get last_completed_migration after other set value => keep value', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.last_completed_migration().call();
        expect(data).to.equal(value.toString());
    });
});
