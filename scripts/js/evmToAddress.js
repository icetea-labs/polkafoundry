const { cryptoWaitReady, evmToAddress } = require('@polkadot/util-crypto');

const init = async () => {
    await cryptoWaitReady();
    const args = process.argv.slice(2);

    console.log("Evm to AccountId32", evmToAddress(args[0] || "0xd43593c715fdd31c61141abd04a99fd6822c8558"))
}

init()
