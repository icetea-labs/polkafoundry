const Web3 = require('web3');
const { RPC_PORT, SPECS_PATH, PORT, WS_PORT, BINARY_PATH, SPAWNING_TIME } = require('./constants');
const { spawn } = require('child_process');


const customRequest = async (web3, method, params) => {
    return new Promise((resolve, reject) => {
        web3.currentProvider.send(
            {
                jsonrpc: '2.0',
                id: 1,
                method,
                params,
            },
            (error, result) => {
            if (error) {
                reject(
                    `Failed to send custom request (${method} (${params.join(',')})): ${
                        error.message || error.toString()
                    }`
                );
            }
            resolve(result);
        }
    );
    });
}

// Create a block and finalize it.
// It will include all previously executed transactions since the last finalized block.
async function createAndFinalizeBlock(web3) {
    const response = await customRequest(web3, 'engine_createBlock', [true, true, null]);
    if (!response.result) {
        throw new Error(`Unexpected result: ${JSON.stringify(response)}`);
    }
}

const startPolkafoundryNode = async (specFileName, provider) => {
    let web3;
    if (!provider || provider === 'http') {
        web3 = new Web3(`http://localhost:${RPC_PORT}`);
    }

    const cmd = BINARY_PATH;
    const args = [
        `--chain=${SPECS_PATH}/${specFileName}`,
        `--validator`, // Required by manual sealing to author the blocks
        `--execution=Native`, // Faster execution using native
        `--no-telemetry`,
        `--no-prometheus`,
        `--sealing=Manual`,
        `-linfo`,
        `--port=${PORT}`,
        `--rpc-port=${RPC_PORT}`,
        `--ws-port=${WS_PORT}`,
        '--start-dev',
        `--tmp`,
    ];
    const binary = spawn(cmd, args);

    binary.on("error", (err) => {
        if (err.errno === 'ENOENT') {
            console.error(
                `\x1b[31mMissing Polkafoundry binary (${BINARY_PATH}).\nPlease compile the Polkafoundry project:\ncargo build\x1b[0m`
            );
        } else {
            console.error(err);
            binary.kill();
        }
        process.exit(1);
    });
    const binaryLogs = [];
    await new Promise((resolve => {
        const timer = setTimeout(() => {
            console.error(`\x1b[31m Failed to start Polkafoundry Node.\x1b[0m`);
            console.error(`Command: ${cmd} ${args.join(" ")}`);
            console.error(`Logs:`);
            console.error(binaryLogs.map((chunk) => chunk.toString()).join("\n"));
            binary.kill();
            process.exit(1);
        }, SPAWNING_TIME - 2000);
        const onData = async (chunk) => {
            const log = chunk.toString()
            if (process.env.DISPLAY_LOG) {
                console.log(log);
            }

            binaryLogs.push(chunk);

            if (log.match(/Polkafoundry dev ready/)) {
                if (!provider || provider === 'http') {
                    // This is needed as the EVM runtime needs to warmup with a first call
                    await web3.eth.getChainId();
                }

                clearTimeout(timer);
                if (!process.env.DISPLAY_LOG) {
                    binary.stderr.off('data', onData);
                    binary.stdout.off('data', onData);
                }
                // console.log(`\x1b[31m Starting RPC\x1b[0m`);
                resolve();
            }
        };
        binary.stderr.on("data", onData);
        binary.stdout.on("data", onData);
    }))
    if (provider === 'ws') {
        web3 = new Web3(`ws://localhost:${WS_PORT}`);
    }

    return { web3, binary };

}

const describeWithPolkafoundry = (title, specFilename, cb, provider) => {
    describe(title, () => {
        let context = { web3: null };
        let binary;
        // Making sure the Frontier node has started
        before('Starting Polkafoundry Test Node', async function () {
            this.timeout(SPAWNING_TIME);
            const init = await startPolkafoundryNode(specFilename, provider);
            context.web3 = init.web3;
            binary = init.binary;
        });

        after(async function () {
            //console.log(`\x1b[31m Killing RPC\x1b[0m`);
            binary.kill();
        });

        cb(context);
    });

}

module.exports = {
    customRequest,
    createAndFinalizeBlock,
    startPolkafoundryNode,
    describeWithPolkafoundry
}
