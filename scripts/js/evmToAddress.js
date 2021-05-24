const { cryptoWaitReady, evmToAddress } = require('@polkadot/util-crypto');

const init = async () => {
    await cryptoWaitReady();
    console.log("Evm to AccountId32", evmToAddress("0xd43593c715fdd31c61141abd04a99fd6822c8558"))
}

init()
