require('./config');

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

module.exports = {
    customRequest,
    createAndFinalizeBlock,
    describeWithPolkafoundry,
    deployContract,
}
