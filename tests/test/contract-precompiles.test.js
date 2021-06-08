const { customRequest, describeWithPolkafoundry } = require('./utils');
const { GENESIS_ACCOUNT } = require('./constants');
const { expect } = require('chai');

describeWithPolkafoundry('Polkafoundry Precompiles', 'polka-spec.json', (context) => {
    // it.skip('ECR20 should be valid', async () => {
    //     const message = await context.web3.eth.accounts.sign(context.web3.utils.sha3('Hello world'), GENESIS_ACCOUNT_PRIVATE_KEY);
    //     const tx = await customRequest(context.web3, 'eth_call', [
    //         {
    //             from: GENESIS_ACCOUNT,
    //             value: '0x00',
    //             gasPrice: '0x01',
    //             gas: '0x100000',
    //             to: '0x0000000000000000000000000000000000000001',
    //             // data: `0x${Buffer.from(message.messageHash).toString("hex")}`,
    //         },
    //     ]);
    //     console.log('tx_call', tx)
    //     expect(tx.result).equals(
    //         '...'
    //     );
    //
    // })

    it('Sha256 should be valid', async () => {
        const tx = await customRequest(context.web3, 'eth_call', [
            {
                from: GENESIS_ACCOUNT,
                value: '0x00',
                gasPrice: '0x01',
                gas: '0x100000',
                to: '0x0000000000000000000000000000000000000002',
                data: `0x${Buffer.from('Hello world!').toString('hex')}`,
            },
        ]);
        expect(tx.result).equals(
            '0xc0535e4be2b79ffd93291305436bf889314e4a3faec05ecffcbb7df31ad9e51a'
        );

    })

    it('Ripemd160 should be valid', async () => {
        const tx = await customRequest(context.web3, 'eth_call', [
            {
                from: GENESIS_ACCOUNT,
                value: '0x00',
                gasPrice: '0x01',
                gas: '0x100000',
                to: '0x0000000000000000000000000000000000000003',
                data: `0x${Buffer.from('Hello world!').toString('hex')}`,
            },
        ]);

        expect(tx.result).equals(
            '0x0000000000000000000000007f772647d88750add82d8e1a7a3e5c0902a346a3'
        );
    });

    it('Sha3FIPS256 should be valid', async () => {
        const tx = await customRequest(context.web3, 'eth_call', [
            {
                from: GENESIS_ACCOUNT,
                value: '0x00',
                gasPrice: '0x01',
                gas: '0x100000',
                to: '0x0000000000000000000000000000000000000007',
                data: `0x${Buffer.from('hello').toString('hex')}`,
            },
        ]);

        expect(tx.result).equals(
            '0x3338be694f50c5f338814986cdf0686453a888b84f424d792af4b9202398f392'
        );
    });

    it('Sha3FIPS512 should be valid', async () => {
        const tx = await customRequest(context.web3, 'eth_call', [
            {
                from: GENESIS_ACCOUNT,
                value: '0x00',
                gasPrice: '0x01',
                gas: '0x100000',
                to: '0x0000000000000000000000000000000000000008',
                data: `0x${Buffer.from('hello').toString('hex')}`,
            },
        ]);

        expect(tx.result).equals(
            '0x75d527c368f2efe848ecf6b073a36767800805e9eef2b1857d5f984f036eb6df891d75f72d9b154518c1cd58835286d1da9a38deba3de98b5a53e5ed78a84976'
        );
    });

})
