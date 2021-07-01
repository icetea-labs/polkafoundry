require('../config');
const Web3 = require('web3');
const { expect } = require('chai');
const contractObj = require('../build/contracts/WETH9.json');
const { deployContract, transferPayment } = require('../utils');

const RPC = process.env.RPC;
const GENESIS_ACCOUNT = process.env.GENESIS_ACCOUNT;
const OTHER_ACCOUNT = process.env.OTHER_ACCOUNT;
const OTHER_ACCOUNT_PRIVATE_KEY = process.env.OTHER_ACCOUNT_PRIVATE_KEY;
const name = 'Wrapped Ether';
const symbol = 'WETH';
const decimals = 18;
const amountTransfer = 1.23;
const amountOtherTransfer = 1.35;

describe("Contract Weth9", () => {
    const web3 = new Web3(RPC);
    const abi = contractObj.abi;
    let contractAddress = null;

    it('Deploy contract', async () => {
        const contract = await deployContract(web3, contractObj, []);
        contractAddress = contract.contractAddress;
        expect(contractAddress).to.match(/^0x[0-9A-Za-z]{40}$/);
    });

    it('Call method name()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.name().call();
        expect(data).to.equal(name);
    });

    it('Call method symbol()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.symbol().call();
        expect(data).to.equal(symbol);
    });

    it('Call method decimals()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.decimals().call();
        expect(data).to.equal(decimals.toString());
    });

    it('Call method totalSupply()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.totalSupply().call();
        expect(data).to.equal('0');
    });

    // GENESIS_ACCOUNT Transfer to contract
    it(`${GENESIS_ACCOUNT} Transfer ${amountTransfer} to contract`, async () => {
        const transaction = await transferPayment(web3, contractAddress, amountTransfer);
        const transactionHash = transaction.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method balanceOf(${GENESIS_ACCOUNT})`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(GENESIS_ACCOUNT).call();
        expect(data).to.equal(web3.utils.toWei(amountTransfer.toString()));
    });

    it('Call method totalSupply()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.totalSupply().call();
        expect(data).to.equal(web3.utils.toWei(amountTransfer.toString()));
    });
    // End GENESIS_ACCOUNT Transfer to contract

    // OTHER_ACCOUNT Transfer to contract
    it(`${OTHER_ACCOUNT} Transfer ${amountOtherTransfer} to contract`, async () => {
        const transaction = await transferPayment(web3, contractAddress, amountOtherTransfer, OTHER_ACCOUNT, OTHER_ACCOUNT_PRIVATE_KEY);
        const transactionHash = transaction.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method balanceOf(${OTHER_ACCOUNT})`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(OTHER_ACCOUNT).call();
        expect(data).to.equal(web3.utils.toWei(amountOtherTransfer.toString()));
    });

    it('Call method totalSupply()', async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.totalSupply().call();
        expect(data).to.equal(web3.utils.toWei((amountTransfer + amountOtherTransfer).toString()));
    });
    // End OTHER_ACCOUNT Transfer to contract
});
