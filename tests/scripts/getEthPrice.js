const Web3 = require('web3');
const Client = require('./Client.json')
const { customRequest } = require('../test/utils');
const { GENESIS_ACCOUNT, GENESIS_ACCOUNT_PRIVATE_KEY } = require('../test/constants');

const web3 = new Web3(`http://54.169.215.160:9933`);

const jobId = '805e213f04d14d8483b97a704e200b21';
const oracleAddress = '0x7c2Fb3d889a61DB16F1FC62002024959e16794Df';
const clientAddress = '0x50d634E43F5aD7748cf2860760b887655524B593';


const init = async () => {
    const clientContract = new web3.eth.Contract(Client.abi, clientAddress);
    const tx = await web3.eth.accounts.signTransaction(
        {
            from: GENESIS_ACCOUNT,
            to: clientAddress,
            data: await clientContract.methods
                .requestPrice(oracleAddress, jobId, (0.1 * 10 ** 18).toString())
                .encodeABI(),
            value: '0x00',
            gasPrice: '0x01',
            gas: '0x100000',
        },
        GENESIS_ACCOUNT_PRIVATE_KEY
    );
    await customRequest(web3, 'eth_sendRawTransaction', [tx.rawTransaction]);
    console.log('ethPrice', await clientContract.methods.currentPrice().call())

}

init();
