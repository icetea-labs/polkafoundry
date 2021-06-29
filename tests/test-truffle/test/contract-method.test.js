const Test = require('../build/contracts/Storage.json');
const { createAndFinalizeBlock, customRequest, describeWithPolkafoundry } = require('./utils');
const { GENESIS_ACCOUNT, GENESIS_ACCOUNT_PRIVATE_KEY, STORAGE_CONTRACT_ADDRESS } = require('./constants');
const { expect } = require('chai');

describeWithPolkafoundry('Polkafoundry RPC Contract Method', 'polka-spec.json', (context) => {
    const TEST_CONTRACT_ABI = Test.abi
    const TEST_CONTRACT_BYTECODE = Test.bytecode;


    before('Create the contract', async function () {
        this.timeout(15000);
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
        await customRequest(context.web3, 'eth_sendRawTransaction', [tx.rawTransaction]);
        await createAndFinalizeBlock(context.web3);
    });

    it('Call contract method correctly', async function () {
        const contract = new context.web3.eth.Contract(TEST_CONTRACT_ABI, STORAGE_CONTRACT_ADDRESS);
        const encodeAbi =  await contract.methods.store(13).encodeABI();
        const tx = await context.web3.eth.accounts.signTransaction(
            {
                from: GENESIS_ACCOUNT,
                to: STORAGE_CONTRACT_ADDRESS,
                data: encodeAbi,
                value: '0x00',
                gasPrice: '0x01',
                gas: '0x100000',
            },
            GENESIS_ACCOUNT_PRIVATE_KEY
        );
        await customRequest(context.web3, 'eth_sendRawTransaction', [tx.rawTransaction]);

        await createAndFinalizeBlock(context.web3);

        expect(await contract.methods.retrieve().call()).to.equal('13');
    });

})