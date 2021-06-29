require('../config');
const { expect } = require('chai');
const { createAndFinalizeBlock, customRequest, describeWithPolkafoundry } = require('../utils');
const Test = require('../build/contracts/Storage.json');

const TEST_CONTRACT_ABI = Test.abi
const TEST_CONTRACT_BYTECODE = Test.bytecode;
const GENESIS_ACCOUNT = '0xa99D94C2f8cAef073C3De43b19E5979cb028d9de'; // process.env.GENESIS_ACCOUNT;
const GENESIS_ACCOUNT_BALANCE = process.env.GENESIS_ACCOUNT_BALANCE;
const GENESIS_ACCOUNT_PRIVATE_KEY = '0xcf15b0281d24526b133a830c33bd6cb935b61e8473ec2812b82f06eeeae80f90'; // process.env.GENESIS_ACCOUNT_PRIVATE_KEY;
const TEST_ACCOUNT = process.env.TEST_ACCOUNT;

describeWithPolkafoundry('Polkafoundry RPC Contract', (context) => {
    const TEST_CONTRACT_BYTECODE = Test.bytecode;
    const TEST_CONTRACT_DEPLOYED_BYTECODE = Test.deployedBytecode

    it('Contract creation should return transaction hash', async function () {
        const balance = await context.web3.eth.getBalance(GENESIS_ACCOUNT);
        console.log('Balance:', balance);

        // Deploy contract
        const tx = await context.web3.eth.accounts.signTransaction(
            {
                from: GENESIS_ACCOUNT,
                data: TEST_CONTRACT_BYTECODE,
                value: '0x00',
                gasPrice: '0x01',
                gas: '77744',
            },
            GENESIS_ACCOUNT_PRIVATE_KEY
        );
        const txContract = await customRequest(context.web3, 'eth_sendRawTransaction', [tx.rawTransaction])

        expect( txContract ).to.deep.equal({
            id: 1,
            jsonrpc: '2.0',
            result: '0xf4b88cd0c796d1f30c02313748f40939a62fe520db4b9f22ccd5effbdbef8444',
        });

        // Verify the contract is not yet stored
        expect(await customRequest(context.web3, 'eth_getCode', [STORAGE_CONTRACT_ADDRESS])).to.deep.equal({
            id: 1,
            jsonrpc: '2.0',
            result: '0x',
        });

        // Verify the contract is stored after the block is produced
        await createAndFinalizeBlock(context.web3);
        const receipt = await context.web3.eth.getTransactionReceipt(txContract.result);

        const contractAddress = receipt.contractAddress;

        expect(await customRequest(context.web3, 'eth_getCode', [contractAddress])).to.deep.equal({
            id: 1,
            jsonrpc: '2.0',
            result:
            TEST_CONTRACT_DEPLOYED_BYTECODE,
        });

    })
})