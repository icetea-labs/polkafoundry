const ERC20 = artifacts.require("PolkafoundryERC20");

module.exports = function (deployer) {
    deployer.deploy(ERC20, BigInt(8000000000000000000000000));
};
