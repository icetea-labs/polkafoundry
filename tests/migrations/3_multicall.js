const Multicall = artifacts.require("Multicall");

module.exports = function (deployer) {
    deployer.deploy(Multicall);
};
