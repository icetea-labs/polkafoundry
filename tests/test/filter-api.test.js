const { expect } = require("chai");
const { step } = require("mocha-steps");

const { createAndFinalizeBlock, customRequest, describeWithPolkafoundry } = require("./utils");
const Storage = require('../build/contracts/Storage.json');

describeWithPolkafoundry("Polkafoundry RPC (EthFilterApi)", 'polka-spec.json', (context) => {
  const GENESIS_ACCOUNT = "0x6be02d1d3665660d22ff9624b7be0551ee1ac91b";
  const GENESIS_ACCOUNT_PRIVATE_KEY =
    "0x99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342";

  const TEST_CONTRACT_BYTECODE = Storage.bytecode;

  async function sendTransaction(context) {
    const tx = await context.web3.eth.accounts.signTransaction(
      {
        from: GENESIS_ACCOUNT,
        data: TEST_CONTRACT_BYTECODE,
        value: "0x00",
        gasPrice: "0x01",
        gas: "0x4F930",
      },
      GENESIS_ACCOUNT_PRIVATE_KEY
    );

    await customRequest(context.web3, "eth_sendRawTransaction", [tx.rawTransaction]);
    return tx;
  }

  step("should create a Log filter and return the ID", async function () {
    let create_filter = await customRequest(context.web3, "eth_newFilter", [
      {
        fromBlock: "0x0",
        toBlock: "latest",
        address: [
          "0xC2Bf5F29a4384b1aB0C063e1c666f02121B6084a",
          "0x5c4242beB94dE30b922f57241f1D02f36e906915",
        ],
        topics: ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
      },
    ]);
    expect(create_filter.result).to.be.eq("0x1");
  });

  step("should increment filter ID", async function () {
    let create_filter = await customRequest(context.web3, "eth_newFilter", [
      {
        fromBlock: "0x1",
        toBlock: "0x2",
        address: "0xC2Bf5F29a4384b1aB0C063e1c666f02121B6084a",
        topics: ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
      },
    ]);
    expect(create_filter.result).to.be.eq("0x2");
  });

  step("should create a Block filter and return the ID", async function () {
    let create_filter = await customRequest(context.web3, "eth_newBlockFilter", []);
    expect(create_filter.result).to.be.eq("0x3");
  });

  step(
    "should return unsupported error for Pending Transaction filter creation",
    async function () {
      let r = await customRequest(context.web3, "eth_newPendingTransactionFilter", []);
      expect(r.error).to.include({
        message: "Method not available.",
      });
    }
  );

  step("should return responses for Block filter polling.", async function () {
    let block = await context.web3.eth.getBlock(0);
    let poll = await customRequest(context.web3, "eth_getFilterChanges", ["0x3"]);

    expect(poll.result.length).to.be.eq(1);
    expect(poll.result[0]).to.be.eq(block.hash);

    await createAndFinalizeBlock(context.web3);

    block = await context.web3.eth.getBlock(1);
    poll = await customRequest(context.web3, "eth_getFilterChanges", ["0x3"]);

    expect(poll.result.length).to.be.eq(1);
    expect(poll.result[0]).to.be.eq(block.hash);

    await createAndFinalizeBlock(context.web3);
    await createAndFinalizeBlock(context.web3);

    block = await context.web3.eth.getBlock(2);
    let block_b = await context.web3.eth.getBlock(3);
    poll = await customRequest(context.web3, "eth_getFilterChanges", ["0x3"]);

    expect(poll.result.length).to.be.eq(2);
    expect(poll.result[0]).to.be.eq(block.hash);
    expect(poll.result[1]).to.be.eq(block_b.hash);
  });

  step("should return responses for Log filter polling.", async function () {
    // Create contract.
    let tx = await sendTransaction(context);
    await createAndFinalizeBlock(context.web3);
    let receipt = await context.web3.eth.getTransactionReceipt(tx.transactionHash);

    expect(receipt.logs.length).to.be.eq(1);

    // Create a filter for the created contract.
    let create_filter = await customRequest(context.web3, "eth_newFilter", [
      {
        fromBlock: "0x0",
        toBlock: "latest",
        address: receipt.contractAddress,
        topics: receipt.logs[0].topics,
      },
    ]);
    let poll = await customRequest(context.web3, "eth_getFilterChanges", [create_filter.result]);

    expect(poll.result.length).to.be.eq(1);
    expect(poll.result[0].address.toLowerCase()).to.be.eq(receipt.contractAddress.toLowerCase());
    expect(poll.result[0].topics).to.be.deep.eq(receipt.logs[0].topics);

    // A subsequent request must be empty.
    poll = await customRequest(context.web3, "eth_getFilterChanges", [create_filter.result]);
    expect(poll.result.length).to.be.eq(0);
  });

  step("should return response for raw Log filter request.", async function () {
    // Create contract.
    let tx = await sendTransaction(context);
    await createAndFinalizeBlock(context.polkadotApi);
    let receipt = await context.web3.eth.getTransactionReceipt(tx.transactionHash);

    expect(receipt.logs.length).to.be.eq(1);

    // Create a filter for the created contract.
    let create_filter = await customRequest(context.web3, "eth_newFilter", [
      {
        fromBlock: "0x0",
        toBlock: "latest",
        address: receipt.contractAddress,
        topics: receipt.logs[0].topics,
      },
    ]);
    let poll = await customRequest(context.web3, "eth_getFilterLogs", [create_filter.result]);

    expect(poll.result.length).to.be.eq(1);
    expect(poll.result[0].address.toLowerCase()).to.be.eq(receipt.contractAddress.toLowerCase());
    expect(poll.result[0].topics).to.be.deep.eq(receipt.logs[0].topics);

    // A subsequent request must return the same response.
    poll = await customRequest(context.web3, "eth_getFilterLogs", [create_filter.result]);

    expect(poll.result.length).to.be.eq(1);
    expect(poll.result[0].address.toLowerCase()).to.be.eq(receipt.contractAddress.toLowerCase());
    expect(poll.result[0].topics).to.be.deep.eq(receipt.logs[0].topics);
  });

  step("should uninstall created filters.", async function () {
    let create_filter = await customRequest(context.web3, "eth_newBlockFilter", []);
    let filter_id = create_filter.result;

    // Should return true when removed from the filter pool.
    let uninstall = await customRequest(context.web3, "eth_uninstallFilter", [filter_id]);
    expect(uninstall.result).to.be.eq(true);

    // Should return error if does not exist.
    let r = await customRequest(context.web3, "eth_uninstallFilter", [filter_id]);
    expect(r.error).to.include({
      message: "Filter id 6 does not exist.",
    });
  });

  step("should drain the filter pool.", async function () {
    this.timeout(15000);
    const block_lifespan_threshold = 100;

    let create_filter = await customRequest(context.web3, "eth_newBlockFilter", []);
    let filter_id = create_filter.result;

    for (let i = 0; i <= block_lifespan_threshold; i++) {
      await createAndFinalizeBlock(context.polkadotApi);
    }

    let r = await customRequest(context.web3, "eth_getFilterChanges", [filter_id]);
    expect(r.error).to.include({
      message: "Filter id 6 does not exist.",
    });
  });

  step("should have a filter pool max size of 500.", async function () {
    const max_filter_pool = 500;

    for (let i = 0; i < max_filter_pool; i++) {
      await customRequest(context.web3, "eth_newBlockFilter", []);
    }

    let r = await customRequest(context.web3, "eth_newBlockFilter", []);
    expect(r.error).to.include({
      message: "Filter pool is full (limit 500).",
    });
  });
});