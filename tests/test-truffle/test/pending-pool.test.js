const { expect } = require('chai');

const { createAndFinalizeBlock, customRequest, describeWithPolkafoundry } = require('./utils');

const { GENESIS_ACCOUNT, GENESIS_ACCOUNT_PRIVATE_KEY } = require('./constants');

describeWithPolkafoundry('Frontier RPC (Pending Pool)', 'polka-spec.json', (context) => {
  // Solidity: contract test { function multiply(uint a) public pure returns(uint d)
  // {return a * 7;}}
  const TEST_CONTRACT_BYTECODE =
    '0x6080604052348015600f57600080fd5b5060ae8061001e6000396000f3fe6080604052348015600f57600080fd' +
    '5b506004361060285760003560e01c8063c6888fa114602d575b600080fd5b605660048036036020811015604157' +
    '600080fd5b8101908080359060200190929190505050606c565b6040518082815260200191505060405180910390' +
    'f35b600060078202905091905056fea265627a7a72315820f06085b229f27f9ad48b2ff3dd9714350c1698a37853' +
    'a30136fa6c5a7762af7364736f6c63430005110032';

  it('should return a pending transaction', async function () {
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

    const tx_hash = (
      await customRequest(context.web3, 'eth_sendRawTransaction', [tx.rawTransaction])
    ).result;

    const pending_transaction = (
      await customRequest(context.web3, 'eth_getTransactionByHash', [tx_hash])
    ).result;
    // pending transactions do not know yet to which block they belong to
    expect(pending_transaction).to.include({
      blockNumber: null,
      hash: tx_hash,
      publicKey:
        '0x624f720eae676a04111631c9ca338c11d0f5a80ee42210c6be72983ceb620fbf645a96f951529f' +
        'a2d70750432d11b7caba5270c4d677255be90b3871c8c58069',
      r: '0x6eb497e6292300fffda1228e80593da8581bc7e5166da2427fbd746368db963b',
      s: '0x50943539ef974bb04f85303cd02f6f6eb777cf7dc282c98765b4aa3364954f1a',
      v: '0x3d',
    });

    await createAndFinalizeBlock(context.web3);

    const processed_transaction = (
      await customRequest(context.web3, 'eth_getTransactionByHash', [tx_hash])
    ).result;
    expect(processed_transaction).to.include({
      blockNumber: '0x1',
      hash: tx_hash,
      publicKey:
        '0x624f720eae676a04111631c9ca338c11d0f5a80ee42210c6be72983ceb620fbf645a96f951529f' +
        'a2d70750432d11b7caba5270c4d677255be90b3871c8c58069',
      r: '0x6eb497e6292300fffda1228e80593da8581bc7e5166da2427fbd746368db963b',
      s: '0x50943539ef974bb04f85303cd02f6f6eb777cf7dc282c98765b4aa3364954f1a',
      v: '0x3d',
    });
  });
});