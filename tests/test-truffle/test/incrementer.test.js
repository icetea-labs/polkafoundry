require('../config');
const Web3 = require('web3');
const { expect } = require('chai');
const contractObj = require('../build/contracts/Incrementer.json');
const { deployContract } = require('../utils');

const RPC = process.env.RPC;
const GENESIS_ACCOUNT = process.env.GENESIS_ACCOUNT;
const GENESIS_ACCOUNT_PRIVATE_KEY = process.env.GENESIS_ACCOUNT_PRIVATE_KEY;
const initValue = 554; // uint256
const addValue = 3453; // uint256

describe("Contract Incrementer", () => {
  const web3 = new Web3(RPC);
  const abi = contractObj.abi;
  let contractAddress = null;

  it('Deploy contract', async () => {
    const contract = await deployContract(web3, contractObj, [initValue]);
    contractAddress = contract.contractAddress;
    expect(contractAddress).to.match(/^0x[0-9A-Za-z]{40}$/);
  });

  it('Call contract get init value', async () => {
    const contract = new web3.eth.Contract(abi, contractAddress);
    const data = await contract.methods.getNumber().call();
    expect(data).to.equal(initValue.toString());
  });

  it('Call contract set value', async () => {
    const incrementer = new web3.eth.Contract(abi);
    const encoded = incrementer.methods.increment(addValue).encodeABI();
    const callTransaction = await web3.eth.accounts.signTransaction(
        {
          from: GENESIS_ACCOUNT,
          to: contractAddress,
          data: encoded,
          gas: process.env.GAS || await web3.eth.getGasPrice(),
        },
        GENESIS_ACCOUNT_PRIVATE_KEY
    );

    const callReceipt = await web3.eth.sendSignedTransaction(callTransaction.rawTransaction);
    const transactionHash = callReceipt.transactionHash;
    expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
  });

  it('Call contract get value after set value', async () => {
    const contract = new web3.eth.Contract(abi, contractAddress);
    const data = await contract.methods.getNumber().call();
    expect(data).to.equal((initValue + addValue).toString());
  });
});
