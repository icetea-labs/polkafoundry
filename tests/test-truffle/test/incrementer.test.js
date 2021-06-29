require('../config');
const Web3 = require('web3');
const { expect } = require('chai');
const contractObj = require('../build/contracts/Incrementer.json');
const { deployContract, callMethod } = require('../utils');

const RPC = process.env.RPC;
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
    const callReceipt = await callMethod(web3, abi, contractAddress, encoded);
    const transactionHash = callReceipt.transactionHash;
    expect(transactionHash).to.match(/^0x[0-9A-Za-z]{64}$/);
  });

  it('Call contract get value after set value', async () => {
    const contract = new web3.eth.Contract(abi, contractAddress);
    const data = await contract.methods.getNumber().call();
    expect(data).to.equal((initValue + addValue).toString());
  });
});
