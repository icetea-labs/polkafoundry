const PolkafoundryERC20 = require('../build/contracts/PolkafoundryERC20.json');
const WETH9 = require('../build/contracts/WETH9.json');
const UniswapV2Factory = require('@uniswap/v2-core/build/UniswapV2Factory.json');
const UniswapV2Pair = require('@uniswap/v2-core/build/UniswapV2Pair.json');
const UniswapRouter02 = require('@uniswap/v2-periphery/build/UniswapV2Router02.json');
const { createAndFinalizeBlock, customRequest, describeWithPolkafoundry, deployContract } = require('./utils');
const { GENESIS_ACCOUNT, GENESIS_ACCOUNT_PRIVATE_KEY, } = require('./constants');
const { expect } = require('chai');

describeWithPolkafoundry('Polkafoundry Uniswap Contract', 'polka-spec.json', (context) => {
    let erc20Address;
    let wethAddress;
    let uniswapFactoryAddress;
    let uniswapRouter02Address;
    let uniswapPairAddress;

    before('Create ERC20 contract', async function () {
        this.timeout(15000);
        const tx = await deployContract(context.web3, PolkafoundryERC20, [BigInt(8000000000000000000000000)], '0x1000000', '0x01')
        await createAndFinalizeBlock(context.web3);
        const recipe = await context.web3.eth.getTransactionReceipt(tx);
        erc20Address = recipe.contractAddress;
    })

    before('Create WETH contract', async function () {
        this.timeout(15000);
        const tx = await deployContract(context.web3, WETH9, [BigInt(8000000000000000000000000)], '0x6000000', '0x01')
        await createAndFinalizeBlock(context.web3);
        const recipe = await context.web3.eth.getTransactionReceipt(tx);
        wethAddress = recipe.contractAddress;
    });

    before('Create UniswapFactory contract', async function () {
        this.timeout(15000);
        const gasLimit = (await context.web3.eth.getBlock("latest")).gasLimit;
        const tx = await deployContract(context.web3, UniswapV2Factory, [GENESIS_ACCOUNT], gasLimit + 1000, '0x01')
        await createAndFinalizeBlock(context.web3);
        const recipe = await context.web3.eth.getTransactionReceipt(tx);
        uniswapFactoryAddress = recipe.contractAddress;
    })

    before('Create UniswapPair contract', async function () {
        this.timeout(15000);
        const gasLimit = (await context.web3.eth.getBlock("latest")).gasLimit;
        const tx = await deployContract(context.web3, UniswapV2Pair, [], gasLimit + 1000, '0x01')
        await createAndFinalizeBlock(context.web3);
        const recipe = await context.web3.eth.getTransactionReceipt(tx);
        uniswapPairAddress = recipe.contractAddress;
    })

    before('Create UniswapRouter contract', async function () {
        this.timeout(15000);
        const gasLimit = (await context.web3.eth.getBlock("latest")).gasLimit;
        const tx = await deployContract(context.web3, UniswapRouter02, [uniswapFactoryAddress, wethAddress], gasLimit + 1000, '0x01')
        await createAndFinalizeBlock(context.web3);
        const recipe = await context.web3.eth.getTransactionReceipt(tx);
        uniswapRouter02Address = recipe.contractAddress;
    })

    it ('Deploy UniswapFactory success', async () => {
        expect(await customRequest(context.web3, 'eth_getCode', [uniswapFactoryAddress])).to.deep.equal({
            id: 1,
            jsonrpc: '2.0',
            result: '0x' + UniswapV2Factory.evm.deployedBytecode.object
        });
    })

    it ('Can swap ERC20 - WETH', async function () {
        const factoryContract = new context.web3.eth.Contract(UniswapV2Factory.abi, uniswapFactoryAddress);
        const router02contract = new context.web3.eth.Contract(UniswapRouter02.abi, uniswapRouter02Address);
        const erc20Contract = new context.web3.eth.Contract(PolkafoundryERC20.abi, erc20Address);
        const gasLimit = (await context.web3.eth.getBlock("latest")).gasLimit;

        // add a pair for ERC20-WETH
        const txPair = await context.web3.eth.accounts.signTransaction(
            {
                from: GENESIS_ACCOUNT,
                to: uniswapFactoryAddress,
                data: await factoryContract.methods.createPair(
                    erc20Address,
                    wethAddress,
                ).encodeABI(),
                value: '0x00',
                gasPrice: '0x01',
                gas: gasLimit + 10000,
            },
            GENESIS_ACCOUNT_PRIVATE_KEY
        );
        await customRequest(context.web3, 'eth_sendRawTransaction', [txPair.rawTransaction]);
        await createAndFinalizeBlock(context.web3);

        const pairFor = await factoryContract.methods.getPair(
            erc20Address,
            wethAddress,
        ).call()
        const pairContract = new context.web3.eth.Contract(UniswapV2Pair.abi, pairFor);

        const txApprove = await context.web3.eth.accounts.signTransaction(
            {
                from: GENESIS_ACCOUNT,
                to: erc20Address,
                data: await erc20Contract.methods.approve(
                    uniswapRouter02Address,
                    (500 * 10 ** 18).toString(),
                ).encodeABI(),
                value: '0x00',
                gasPrice: '0x01',
                gas: gasLimit + 10000,
            },
            GENESIS_ACCOUNT_PRIVATE_KEY
        );
        await customRequest(context.web3, 'eth_sendRawTransaction', [txApprove.rawTransaction]);
        await createAndFinalizeBlock(context.web3);

        // add liquidity between ERC20-WETH
        const tx = await context.web3.eth.accounts.signTransaction(
            {
                from: GENESIS_ACCOUNT,
                to: uniswapRouter02Address,
                data: await router02contract.methods.addLiquidityETH(
                    erc20Address,
                    (500 * 10 ** 18).toString(),
                    (300 * 10 ** 18).toString(),
                    (10 ** 18).toString(),
                    GENESIS_ACCOUNT,
                    '2000000000',
                ).encodeABI(),
                value: (10 ** 18).toString(),
                gasPrice: '0x01',
                gas: gasLimit + 10000,
            },
            GENESIS_ACCOUNT_PRIVATE_KEY
        );
        await customRequest(context.web3, 'eth_sendRawTransaction', [tx.rawTransaction]);
        await createAndFinalizeBlock(context.web3);

        const reserves = await pairContract.methods.getReserves().call();
        let reserveETH;
        let reserveERC20;
        if (wethAddress < erc20Address) {
            reserveETH = reserves[0];
            reserveERC20 = reserves[1];
        } else {
            reserveETH = reserves[1];
            reserveERC20 = reserves[0];
        }

        let erc20AmountMin = await router02contract.methods.getAmountOut(
            (0.1 * 10 ** 18).toString(),
            reserveETH,
            reserveERC20
        ).call();
        const balanceBeforeSwap = await erc20Contract.methods.balanceOf(GENESIS_ACCOUNT).call()
        // Swap
        const txSwap = await context.web3.eth.accounts.signTransaction(
            {
                from: GENESIS_ACCOUNT,
                to: uniswapRouter02Address,
                data: await router02contract.methods.swapExactETHForTokens(
                    erc20AmountMin,
                    [wethAddress, erc20Address],
                    GENESIS_ACCOUNT,
                    '2000000000'
                ).encodeABI(),
                value: (0.1 * 10 ** 18),
                gasPrice: '0x01',
                gas: gasLimit + 10000,
            },
            GENESIS_ACCOUNT_PRIVATE_KEY
        );
        await customRequest(context.web3, 'eth_sendRawTransaction', [txSwap.rawTransaction]);
        await createAndFinalizeBlock(context.web3);

        const balanceAfterSwap = await erc20Contract.methods.balanceOf(GENESIS_ACCOUNT).call()
        // Expect balance after swap great than before swap
        expect(Number(balanceAfterSwap)).to.be.greaterThan(Number(balanceBeforeSwap))
    })
})
