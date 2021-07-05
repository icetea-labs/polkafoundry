require('./config');
const { Transaction } = require('ethereumjs-tx');

const GENESIS_ACCOUNT = process.env.GENESIS_ACCOUNT;
const GENESIS_ACCOUNT_PRIVATE_KEY = process.env.GENESIS_ACCOUNT_PRIVATE_KEY;

const customRequest = async (web3, method, params) => {
    return new Promise((resolve, reject) => {
        web3.currentProvider.send(
            {
                jsonrpc: '2.0',
                id: 1,
                method,
                params,
            },
            (error, result) => {
            if (error) {
                reject(
                    `Failed to send custom request (${method} (${params.join(',')})): ${
                        error.message || error.toString()
                    }`
                );
            }
            resolve(result);
        });
    });
}

async function createAndFinalizeBlock(web3) {
    const response = await customRequest(web3, 'engine_createBlock', [true, true, null]);
    if (!response.result) {
        throw new Error(`Unexpected result: ${JSON.stringify(response)}`);
    }
}

const describeWithPolkafoundry = (title, cb, provider) => {
    cb({
      web3: new Web3(process.env.RPC),
    });
};

const deployContract = async (web3, contractObj, args = []) => {
  const { abi, bytecode } = contractObj;

  const incrementer = new web3.eth.Contract(abi);
  const incrementerTx = incrementer.deploy({
   data: bytecode,
   arguments: args,
  });

  const createTransaction = await web3.eth.accounts.signTransaction({
    from: process.env.GENESIS_ACCOUNT,
    data: incrementerTx.encodeABI(),
    gas: process.env.GAS || await web3.eth.getGasPrice(),
    // gas: '6721975',
    // gasPrice: '0x01',
    // gasLimit: await web3.eth.getBlock("latest").gasLimit,
  }, process.env.GENESIS_ACCOUNT_PRIVATE_KEY);

  return web3.eth.sendSignedTransaction(createTransaction.rawTransaction);
}

const callMethod = async (web3, abi, contractAddress, encoded, account = GENESIS_ACCOUNT, privateKey = GENESIS_ACCOUNT_PRIVATE_KEY) => {
    const callTransaction = await web3.eth.accounts.signTransaction(
        {
            from: account,
            to: contractAddress,
            data: encoded,
            gas: process.env.GAS || await web3.eth.getGasPrice(),
        },
        privateKey
    );

    return web3.eth.sendSignedTransaction(callTransaction.rawTransaction);
}

const transferPayment = async (web3, to, amount, from = GENESIS_ACCOUNT, privateKey = GENESIS_ACCOUNT_PRIVATE_KEY) => {
    const tx = await web3.eth.accounts.signTransaction({
            from: from,
            to: to,
            value: web3.utils.toWei(amount.toString(), "ether"),
            gas: web3.utils.toHex(process.env.GAS),
        },
        privateKey
    );

    return web3.eth.sendSignedTransaction(tx.rawTransaction);
}

module.exports = {
    customRequest,
    createAndFinalizeBlock,
    describeWithPolkafoundry,
    deployContract,
    callMethod,
    transferPayment,
}
