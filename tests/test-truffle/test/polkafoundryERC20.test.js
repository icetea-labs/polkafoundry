require('../config');
const Web3 = require('web3');
const { expect } = require('chai');
const contractObj = require('../build/contracts/PolkafoundryERC20.json');
const { deployContract, callMethod } = require('../utils');

const RPC = process.env.RPC;
const GENESIS_ACCOUNT = process.env.GENESIS_ACCOUNT;
const OTHER_ACCOUNT = process.env.OTHER_ACCOUNT;
const OTHER_ACCOUNT_PRIVATE_KEY = process.env.OTHER_ACCOUNT_PRIVATE_KEY;
const OTHER_ACCOUNT2 = process.env.OTHER_ACCOUNT2;
const OTHER_ACCOUNT_PRIVATE_KEY2 = process.env.OTHER_ACCOUNT_PRIVATE_KEY2;
const name = 'Polkafoundry';
const totalSupply = 2e6; // uint256
const symbol = 'PKF';
const decimals = 18;
const amountTransfer = 123; // uint256
const amountOtherTransfer = 13; // uint256

describe("Contract PolkafoundryERC20", () => {
    const web3 = new Web3(RPC);
    const abi = contractObj.abi;
    let contractAddress = null;

    it('Deploy contract', async () => {
        const contract = await deployContract(web3, contractObj, [totalSupply]);
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
        expect(data).to.equal(totalSupply.toString());
    });

    it(`Call method balanceOf(${GENESIS_ACCOUNT}) -> owner`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(GENESIS_ACCOUNT).call();
        expect(data).to.equal(totalSupply.toString());
    });

    it(`Call method balanceOf(${OTHER_ACCOUNT})`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(OTHER_ACCOUNT).call();
        expect(data).to.equal('0');
    });

    // transfer step 1
    it(`${GENESIS_ACCOUNT} call method transfer(${OTHER_ACCOUNT}, ${amountTransfer})`, async () => {
        const contract = new web3.eth.Contract(abi);
        const encoded = contract.methods.transfer(OTHER_ACCOUNT, amountTransfer).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method balanceOf(${GENESIS_ACCOUNT}) after transfer`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(GENESIS_ACCOUNT).call();
        expect(data).to.equal((totalSupply - amountTransfer).toString());
    });

    it(`Call method balanceOf(${OTHER_ACCOUNT}) after transfer`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(OTHER_ACCOUNT).call();
        expect(data).to.equal(amountTransfer.toString());
    });
    // end transfer step 1

    // transfer step 2
    it(`${OTHER_ACCOUNT} call method transfer(${GENESIS_ACCOUNT}, ${amountOtherTransfer})`, async () => {
        const contract = new web3.eth.Contract(abi);
        const encoded = contract.methods.transfer(GENESIS_ACCOUNT, amountOtherTransfer).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded, OTHER_ACCOUNT, OTHER_ACCOUNT_PRIVATE_KEY);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method balanceOf(${GENESIS_ACCOUNT}) after transfer`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(GENESIS_ACCOUNT).call();
        expect(data).to.equal((totalSupply - amountTransfer + amountOtherTransfer).toString());
    });

    it(`Call method balanceOf(${OTHER_ACCOUNT}) after transfer`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(OTHER_ACCOUNT).call();
        expect(data).to.equal((amountTransfer - amountOtherTransfer).toString());
    });
    // end transfer step 2

    // transfer step 3
    it(`${OTHER_ACCOUNT} call method transfer(${GENESIS_ACCOUNT}, ${amountTransfer}) (over balance) -> not success`, async () => {
        const contract = new web3.eth.Contract(abi);
        const encoded = contract.methods.transfer(GENESIS_ACCOUNT, amountTransfer).encodeABI();
        try {
            const callReceipt = await callMethod(web3, abi, contractAddress, encoded, OTHER_ACCOUNT, OTHER_ACCOUNT_PRIVATE_KEY);
            const transactionHash = callReceipt.transactionHash;
            expect(transactionHash).to.not.match(/^0x[0-9A-Za-z]{64}$/);
        } catch (e) {
            expect(e.message).to.equal('Returned error: VM Exception while processing transaction: revert ERC20: transfer amount exceeds balance');
        }
    });

    it(`Call method balanceOf(${GENESIS_ACCOUNT}) after transfer over balance`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(GENESIS_ACCOUNT).call();
        expect(data).to.equal((totalSupply - amountTransfer + amountOtherTransfer).toString());
    });

    it(`Call method balanceOf(${OTHER_ACCOUNT}) after transfer over balance`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(OTHER_ACCOUNT).call();
        expect(data).to.equal((amountTransfer - amountOtherTransfer).toString());
    });
    // end transfer step 3

    // allowance
    it(`Call method allowance(${GENESIS_ACCOUNT}, ${OTHER_ACCOUNT})`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.allowance(GENESIS_ACCOUNT, OTHER_ACCOUNT).call();
        expect(data).to.equal('0');
    });

    // approve
    it(`${GENESIS_ACCOUNT} call method approve(${OTHER_ACCOUNT}, ${amountTransfer})`, async () => {
        const contract = new web3.eth.Contract(abi);
        const encoded = contract.methods.approve(OTHER_ACCOUNT, amountTransfer).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method allowance(${GENESIS_ACCOUNT}, ${OTHER_ACCOUNT}) after approve`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.allowance(GENESIS_ACCOUNT, OTHER_ACCOUNT).call();
        expect(data).to.equal(amountTransfer.toString());
    });
    // end approve

    // increaseAllowance
    it(`${GENESIS_ACCOUNT} call method increaseAllowance(${OTHER_ACCOUNT}, ${amountTransfer})`, async () => {
        const contract = new web3.eth.Contract(abi);
        const encoded = contract.methods.increaseAllowance(OTHER_ACCOUNT, amountTransfer).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method allowance(${GENESIS_ACCOUNT}, ${OTHER_ACCOUNT}) after increaseAllowance`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.allowance(GENESIS_ACCOUNT, OTHER_ACCOUNT).call();
        expect(data).to.equal((amountTransfer * 2).toString());
    });
    // end increaseAllowance

    // decreaseAllowance
    it(`${GENESIS_ACCOUNT} call method decreaseAllowance(${OTHER_ACCOUNT}, ${amountOtherTransfer})`, async () => {
        const contract = new web3.eth.Contract(abi);
        const encoded = contract.methods.decreaseAllowance(OTHER_ACCOUNT, amountOtherTransfer).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method allowance(${GENESIS_ACCOUNT}, ${OTHER_ACCOUNT}) after decreaseAllowance`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.allowance(GENESIS_ACCOUNT, OTHER_ACCOUNT).call();
        expect(data).to.equal((amountTransfer * 2 - amountOtherTransfer).toString());
    });
    // end decreaseAllowance

    // transferFrom
    it(`${OTHER_ACCOUNT} call method transferFrom(${GENESIS_ACCOUNT}, ${OTHER_ACCOUNT2}, ${amountTransfer - amountOtherTransfer})`, async () => {
        const contract = new web3.eth.Contract(abi);
        const encoded = contract.methods.transferFrom(GENESIS_ACCOUNT, OTHER_ACCOUNT2, amountTransfer - amountOtherTransfer).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded, OTHER_ACCOUNT, OTHER_ACCOUNT_PRIVATE_KEY);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method balanceOf(${OTHER_ACCOUNT2}) after transferFrom`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(OTHER_ACCOUNT2).call();
        expect(data).to.equal((amountTransfer - amountOtherTransfer).toString());
    });

    it(`Call method allowance(${GENESIS_ACCOUNT}, ${OTHER_ACCOUNT}) after transferFrom`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.allowance(GENESIS_ACCOUNT, OTHER_ACCOUNT).call();
        expect(data).to.equal((amountTransfer).toString());
    });
    // end transferFrom

    // account other 2 use amount
    it(`${OTHER_ACCOUNT2} call method transfer(${OTHER_ACCOUNT}, ${amountTransfer - amountOtherTransfer})`, async () => {
        const contract = new web3.eth.Contract(abi);
        const encoded = contract.methods.transfer(OTHER_ACCOUNT, amountTransfer - amountOtherTransfer).encodeABI();
        const callReceipt = await callMethod(web3, abi, contractAddress, encoded, OTHER_ACCOUNT2, OTHER_ACCOUNT_PRIVATE_KEY2);
        const transactionHash = callReceipt.transactionHash;
        expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
    });

    it(`Call method balanceOf(${OTHER_ACCOUNT2}) after transfer`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(OTHER_ACCOUNT2).call();
        expect(data).to.equal('0');
    });

    it(`Call method balanceOf(${OTHER_ACCOUNT}) after transfer`, async () => {
        const contract = new web3.eth.Contract(abi, contractAddress);
        const data = await contract.methods.balanceOf(OTHER_ACCOUNT).call();
        expect(data).to.equal((amountTransfer * 2 - amountOtherTransfer * 2).toString());
    });
    // end account other 2 use amount
});
