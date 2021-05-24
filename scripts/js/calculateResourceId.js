const ethers = require('ethers');

const init = () => {
    const args = process.argv.slice(2);
    if (!args[0].length) {
        throw Error('You must provide address')
    }

    console.log('resource id', ethers.utils.hexZeroPad((args[0] + ethers.utils.hexlify(0).substr(2)), 32))
}

init()
