const { decodeAddress, encodeAddress } = require("@polkadot/keyring");
const { u8aToHex } = require('@polkadot/util');
const { cryptoWaitReady } = require('@polkadot/util-crypto');

const init = async () => {
    await cryptoWaitReady();

    const args = process.argv.slice(2);

    console.log("AccountId to Hex", u8aToHex(decodeAddress(args[0] || '5HNFRkCYoriHQwuJbt5YgSwegRTxmSQRe51UKEEBWnUZuHf5')));
}

init()
