const Test = require('../build/contracts/Storage.json');
const { createAndFinalizeBlock, customRequest, describeWithPolkafoundry } = require('./utils');
const { GENESIS_ACCOUNT, GENESIS_ACCOUNT_PRIVATE_KEY, STORAGE_CONTRACT_ADDRESS } = require('./constants');
const { expect } = require('chai');

describeWithPolkafoundry('Polkafoundry RPC Contract', 'polka-spec.json', (context) => {
    const TEST_CONTRACT_BYTECODE = Test.bytecode;
    const TEST_CONTRACT_DEPLOYED_BYTECODE = Test.deployedBytecode

    it('Contract creation should return transaction hash', async function () {
        this.timeout(15000);
        // Deploy contract
        const tx = await context.web3.eth.accounts.signTransaction(
            {
                from: GENESIS_ACCOUNT,
                data: TEST_CONTRACT_BYTECODE,
                value: '0x00',
                gasPrice: '0x01',
                gas: '0x100000',
            },
            GENESIS_ACCOUNT_PRIVATE_KEY
        );
        const txContract = await customRequest(context.web3, 'eth_sendRawTransaction', [tx.rawTransaction])
        expect(
            txContract
        ).to.deep.equal({
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
