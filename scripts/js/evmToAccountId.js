const { decodeAddress, encodeAddress } = require("@polkadot/keyring");
const { u8aToHex } = require('@polkadot/util');
const { cryptoWaitReady, evmToAddress } = require('@polkadot/util-crypto');

const init = async () => {
    await cryptoWaitReady();

    const args = process.argv.slice(2);

    console.log("AccountId to Hex", (evmToAddress(args[0] || '0xa99D94C2f8cAef073C3De43b19E5979cb028d9de')));
}

init()
