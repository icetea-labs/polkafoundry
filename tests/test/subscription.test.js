const { expect } = require("chai");
const { step } = require("mocha-steps");
const Test = require('../build/contracts/Storage.json');

const { createAndFinalizeBlock, customRequest, describeWithPolkafoundry } = require("./utils");


describeWithPolkafoundry("Polkafoundry RPC (Subscription)", 'polka-spec.json', (context) => {
  let subscription;
  let logs_generated = 0;

  const GENESIS_ACCOUNT = "0x6be02d1d3665660d22ff9624b7be0551ee1ac91b";
  const GENESIS_ACCOUNT_PRIVATE_KEY =
    "0x99B3C12287537E38C90A9219D4CB074A89A16E9CDB20BF85728EBD97C343E342";

  const TEST_CONTRACT_BYTECODE = Test.bytecode
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

  step("should connect", async function () {
    await createAndFinalizeBlock(context.web3);
    // @ts-ignore
    const connected = context.web3.currentProvider.connected;
    expect(connected).to.equal(true);
  });

  step("should subscribe", async function () {
    subscription = context.web3.eth.subscribe("newBlockHeaders", function (error, result) { });

    let connected = false;
    let subscriptionId = "";
    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        connected = true;
        subscriptionId = d;
        resolve();
      });
    });

    subscription.unsubscribe();
    expect(connected).to.equal(true);
    expect(subscriptionId).to.have.lengthOf(16);
  });

  step("should get newHeads stream", async function (done) {
    subscription = context.web3.eth.subscribe("newBlockHeaders", function (error, result) { });
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        resolve();
      });
    });
    subscription.unsubscribe();
    expect(data).to.include({
      author: "0x0000000000000000000000000000000000000000",
      difficulty: "0",
      extraData: "0x",
      logsBloom: `0x${"0".repeat(512)}`,
      miner: "0x0000000000000000000000000000000000000000",
      number: 2,
      receiptsRoot: "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
      sha3Uncles: "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
      stateRoot: "0x0000000000000000000000000000000000000000000000000000000000000000",
      transactionsRoot: "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    });
    expect((data).sealFields).to.eql([
      "0x0000000000000000000000000000000000000000000000000000000000000000",
      "0x0000000000000000",
    ]);
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should get newPendingTransactions stream", async function (done) {
    subscription = context.web3.eth.subscribe("pendingTransactions", function (error, result) { });

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    expect(data).to.be.not.null;
    expect(tx["transactionHash"]).to.be.eq(data);
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should subscribe to all logs", async function (done) {
    subscription = context.web3.eth.subscribe("logs", {}, function (error, result) { });

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    const block = await context.web3.eth.getBlock("latest");
    expect(data).to.include({
      blockHash: block.hash,
      blockNumber: block.number,
      data: "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
      logIndex: 0,
      removed: false,
      transactionHash: block.transactions[0],
      transactionIndex: 0,
      transactionLogIndex: "0x0",
    });
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should subscribe to logs by address", async function (done) {
    subscription = context.web3.eth.subscribe(
      "logs",
      {
        address: "0x42e2EE7Ba8975c473157634Ac2AF4098190fc741",
      },
      function (error, result) { }
    );

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    expect(data).to.not.be.null;
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should subscribe to logs by topic", async function (done) {
    subscription = context.web3.eth.subscribe(
      "logs",
      {
        topics: ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
      },
      function (error, result) { }
    );

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    expect(data).to.not.be.null;
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should get past events on subscription", async function (done) {
    subscription = context.web3.eth.subscribe(
      "logs",
      {
        fromBlock: "0x0",
      },
      function (error, result) { }
    );

    let data = [];
    await new Promise((resolve) => {
      subscription.on("data", function (d) {
        data.push(d);
        if (data.length == logs_generated) {
          resolve();
        }
      });
    });
    subscription.unsubscribe();

    expect(data).to.not.be.empty;
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should support topic wildcards", async function (done) {
    subscription = context.web3.eth.subscribe(
      "logs",
      {
        topics: [null, "0x0000000000000000000000000000000000000000000000000000000000000000"],
      },
      function (error, result) { }
    );

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    expect(data).to.not.be.null;
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should support single values wrapped around a sequence", async function (done) {
    subscription = context.web3.eth.subscribe(
      "logs",
      {
        topics: [
          ["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"],
          ["0x0000000000000000000000000000000000000000000000000000000000000000"],
        ],
      },
      function (error, result) { }
    );

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    expect(data).to.not.be.null;
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should support topic conditional parameters", async function (done) {
    subscription = context.web3.eth.subscribe(
      "logs",
      {
        topics: [
          "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
          [
            "0x0000000000000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91b",
            "0x0000000000000000000000000000000000000000000000000000000000000000",
          ],
        ],
      },
      function (error, result) { }
    );

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    expect(data).to.not.be.null;
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should support multiple topic conditional parameters", async function (done) {
    subscription = context.web3.eth.subscribe(
      "logs",
      {
        topics: [
          "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
          [
            "0x0000000000000000000000000000000000000000000000000000000000000000",
            "0x0000000000000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91b",
          ],
          [
            "0x0000000000000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91b",
            "0x0000000000000000000000000000000000000000000000000000000000000000",
          ],
        ],
      },
      function (error, result) { }
    );

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    expect(data).to.not.be.null;
    setTimeout(done, 10000);
  }).timeout(20000);

  step("should combine topic wildcards and conditional parameters", async function (done) {
    subscription = context.web3.eth.subscribe(
      "logs",
      {
        topics: [
          null,
          [
            "0x0000000000000000000000006be02d1d3665660d22ff9624b7be0551ee1ac91b",
            "0x0000000000000000000000000000000000000000000000000000000000000000",
          ],
          null,
        ],
      },
      function (error, result) { }
    );

    await new Promise((resolve) => {
      subscription.on("connected", function (d) {
        resolve();
      });
    });

    const tx = await sendTransaction(context);
    let data = null;
    await new Promise((resolve) => {
      createAndFinalizeBlock(context.web3);
      subscription.on("data", function (d) {
        data = d;
        logs_generated += 1;
        resolve();
      });
    });
    subscription.unsubscribe();

    expect(data).to.not.be.null;
    setTimeout(done, 10000);
  }).timeout(20000);
},
  "ws"
);