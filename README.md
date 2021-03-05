# Polkafoundry

Ethereum compatible parachain based on [Substrate](https://www.substrate.io/) node, ready for hacking :rocket:

## Getting Started

Follow these steps to get started with Polkafoundry :hammer_and_wrench:

### Rust Setup

First, complete the [basic Rust setup instructions](./doc/rust-setup.md).

### Run

Use Rust's native `cargo` command to build and launch the template node:

```sh
cargo run --release -- --dev --tmp
```

### Build

The `cargo run` command will perform an initial build. Use the following command to build the node
without launching it:

```sh
cargo build --release
```

### Embedded Docs

Once the project has been built, the following command can be used to explore all parameters and
subcommands:

```sh
./target/release/polkafoundry -h
```

## Run

The provided `cargo run` command will launch a temporary node and its state will be discarded after
you terminate the process. After the project has been built, there are other ways to launch the
node.

### Single-Node Development Chain

This command will start the single-node development chain with persistent state:

```bash
./target/release/polkafoundry --dev
```

Purge the development chain's state:

```bash
./target/release/polkafoundry purge-chain --dev
```

Start the development chain with detailed logging:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/node-template -lruntime=debug --dev
```

### Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action, refer to
[our Start a Private Network tutorial](https://substrate.dev/docs/en/tutorials/start-a-private-network/).

## Run Parachain

### Starting the Relay Chain
Before we can attach any cumulus-based parachains, we need to compile and launch the relay-chain.

## Compile
```bash
# Clone the Polkadot Repository
git clone https://github.com/paritytech/polkadot.git

# Switch into the Polkadot directory
cd polkadot

# Build the Relay Chain Node
cargo build --release --features=real-overseer

# Print the help page to ensure the node build correctly
./target/release/polkadot --help

# Generate a Plain Chain Spec
./target/release/polkadot build-spec --chain rococo-local --disable-default-bootnode > rococo-custom-plain.json

# Convert to Raw Chain Spec
./target/release/polkadot build-spec --chain rococo-custom-plain.json --raw --disable-default-bootnode > rococo-custom.json

Your final spec must start with the word rococo or the node will not know what runtime logic it includes.
````

### Start Alice's Node

```bash
./target/release/polkadot 
  --chain <path to spec json> \
  --tmp \
  --ws-port 9944 \
  --port 30333 \
  --alice
```
The port and websocket port specified here are the defaults and thus those flags can be omitted. However I've chosen to leave them in to enforce the habit of checking their values. Because Alice is using the defaults, no other nodes on the relay chain or parachains can use these ports.

When the node starts you will see several log messages. Take note of one that looks as follows. This lists Alice's Peer Id. We will need it when connecting other nodes to her.

### Connect Apps UI
To explore and interact with the network, you can use the Polkadot JS Apps UI. If you've started this node using the command above, you can access the node as https://polkadot.js.org/apps/#/?rpc=ws://localhost:9944

### Start Bob's Node
```bash
./target/release/polkadot 
  --chain <path to spec json> \
  --tmp \
  --ws-port 9955 \
  --port 30334 \
  --bob \
  --bootnodes /ip4/<Alice IP>/tcp/30333/p2p/<Alice Peer ID>
```

## Launching a Parachain
We'll begin by deploying a parachain template with parachain id 200. These instructions are written specifically for parachain id 200, however you can re-use these instructions with any parachain id by adjusting occurrences of the number 200 accordingly.

### Generate Genesis State
To register a parachain, the relay chain needs to know the parachain's genesis state. The collator node can export that state to a file for us. The following command will create a file containing the parachain's entire genesis state, hex-encoded.
```bash
./target/release/polkafoundry export-genesis-state --parachain-id 200 > para-200-genesis
```

### Obtain Wasm Validation Function
The relay chain also needs the parachain-specific validation logic to validate parachain blocks. The collator node also has a command to produce this wasm blob.

```bash
./target/release/polkafoundry export-genesis-wasm > para-200-wasm
```

### Start Polkafoundry Node
We can now start the collator node with the following command. Notice that we need to supply the same relay chain spec we used when launching relay chain nodes.
```bash
./target/release/polkafoundry \
  --collator \
  --tmp \
  --parachain-id 200 \
  --port 40333 \
  --ws-port 9844 \
  --alice \
  -- \
  --execution wasm \
  --chain <relay chain spec json> \
  --port 30343 \
  --ws-port 9977
```

The first thing to notice about this command is that several arguments are passed before the lone --, and several more arguments are passed after it. A cumulus collator contains the actual collator node, and also an embedded relay chain node. The arguments before the -- are for the collator, and the arguments after the -- are for the embedded relay chain node.

We give the collator a base path and ports as we did for the relay chain node previously. We also specify the parachain id. Remember to change these collator-specific values if you are executing these instructions a second time for a second parachain. Then we give the embedded relay chain node the relay chain spec we are using. Finally, we give the embedded relay chain node some peer addresses.

### Is It Working?

At this point you should see your collator node running and peering with the relay-chain nodes. You should not see it authoring parachain blocks yet. Authoring will begin when the collator is actually registered on the relay chain (the next step).

At this point your collator's logs should look something like this:

```bash
2021-01-14 15:47:03  Cumulus Test Parachain Collator
2021-01-14 15:47:03  ✌️  version 0.1.0-4786231-x86_64-linux-gnu
2021-01-14 15:47:03  ❤️  by Parity Technologies <admin@parity.io>, 2017-2021
2021-01-14 15:47:03  📋 Chain specification: Local Testnet
2021-01-14 15:47:03  🏷 Node name: Alice
2021-01-14 15:47:03  👤 Role: AUTHORITY
2021-01-14 15:47:03  💾 Database: RocksDb at /tmp/substrateIZ0HQm/chains/local_testnet/db
2021-01-14 15:47:03  ⛓  Native runtime: cumulus-test-parachain-3 (cumulus-test-parachain-1.tx1.au1)
2021-01-14 15:47:03  Parachain id: Id(200)
2021-01-14 15:47:03  Parachain Account: 5Ec4AhPTL6nWnUnw58QzjJvFd3QATwHA3UJnvSD4GVSQ7Gop
2021-01-14 15:47:03  Parachain genesis state: 0x000000000000000000000000000000000000000000000000000000000000000000b86f2a5f94d1029bf54b07867c3c2fa0339e69e31748cfd5921bbb2f176ada6f03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c11131400
2021-01-14 15:47:03  Is collating: yes
2021-01-14 15:47:04  [Relaychain] 🔨 Initializing Genesis block/state (state: 0x1693…5e3f, header-hash: 0x2fc1…2ec3)
2021-01-14 15:47:04  [Relaychain] 👴 Loading GRANDPA authority set from genesis on what appears to be first startup.
2021-01-14 15:47:04  [Relaychain] ⏱  Loaded block-time = 6000 milliseconds from genesis on first-launch
2021-01-14 15:47:04  [Relaychain] 👶 Creating empty BABE epoch changes on what appears to be first startup.
2021-01-14 15:47:04  [Relaychain] 🏷 Local node identity is: 12D3KooWDTBqULpZPTTnRrEZtA53xG3Ade223mQfbLWstg7L3HA4
2021-01-14 15:47:04  [Relaychain] 📦 Highest known block at #0
2021-01-14 15:47:04  [Relaychain] 〽️ Prometheus server started at 127.0.0.1:9616
2021-01-14 15:47:04  [Relaychain] Listening for new connections on 127.0.0.1:9977.
2021-01-14 15:47:05  [Parachain] 🔨 Initializing Genesis block/state (state: 0xb86f…da6f, header-hash: 0x755b…42ca)
2021-01-14 15:47:05  [Parachain] Using default protocol ID "sup" because none is configured in the chain specs
2021-01-14 15:47:05  [Parachain] 🏷 Local node identity is: 12D3KooWEmhCGHnxfuYX9yWoWmnS1MSU7mkoZFnPSAKws2ZL3CCd
2021-01-14 15:47:05  [Parachain] 📦 Highest known block at #0
2021-01-14 15:47:05  [Parachain] Listening for new connections on 127.0.0.1:9855.
2021-01-14 15:47:06  [Relaychain] 🔍 Discovered new external address for our node: /ip4/127.0.0.1/tcp/30343/p2p/12D3KooWDTBqULpZPTTnRrEZtA53xG3Ade223mQfbLWstg7L3HA4
2021-01-14 15:47:06  [Relaychain] 🔍 Discovered new external address for our node: /ip4/192.168.178.77/tcp/30343/p2p/12D3KooWDTBqULpZPTTnRrEZtA53xG3Ade223mQfbLWstg7L3HA4
2021-01-14 15:47:06  [Parachain] 🔍 Discovered new external address for our node: /ip4/192.168.178.77/tcp/30433/p2p/12D3KooWEmhCGHnxfuYX9yWoWmnS1MSU7mkoZFnPSAKws2ZL3CCd
2021-01-14 15:47:08  [Relaychain] 👶 New epoch 29 launching at block 0x765e…c213 (block slot 268439271 >= start slot 268439271).
2021-01-14 15:47:08  [Relaychain] 👶 Next epoch starts at slot 268439281
2021-01-14 15:47:08  [Relaychain] ✨ Imported #291 (0x765e…c213)
2021-01-14 15:47:09  [Relaychain] 💤 Idle (3 peers), best: #291 (0x765e…c213), finalized #289 (0xca88…7eb1), ⬇ 196.9kiB/s ⬆ 161.9kiB/s
2021-01-14 15:47:10  [Parachain] 💤 Idle (0 peers), best: #0 (0x755b…42ca), finalized #0 (0x755b…42ca), ⬇ 809.4kiB/s ⬆ 773.7kiB/s
2021-01-14 15:47:12  [Relaychain] ✨ Imported #292 (0x1cdf…7cf7)
2021-01-14 15:47:12  [Relaychain] ✨ Imported #292 (0x26a5…7d91)
2021-01-14 15:47:14  [Relaychain] 💤 Idle (3 peers), best: #292 (0x1cdf…7cf7), finalized #289 (0xca88…7eb1), ⬇ 256.8kiB/s ⬆ 270.0kiB/s
2021-01-14 15:47:15  [Parachain] 💤 Idle (0 peers), best: #0 (0x755b…42ca), finalized #0 (0x755b…42ca), ⬇ 814.3kiB/s ⬆ 799.9kiB/s
2021-01-14 15:47:18  [Relaychain] ✨ Imported #293 (0x93d5…c54c)
2021-01-14 15:47:19  [Relaychain] 💤 Idle (3 peers), best: #293 (0x93d5…c54c), finalized #290 (0x1109…ea3d), ⬇ 203.6kiB/s ⬆ 200.5kiB/s
2021-01-14 15:47:20  [Parachain] 💤 Idle (0 peers), best: #0 (0x755b…42ca), finalized #0 (0x755b…42ca), ⬇ 751.0kiB/s ⬆ 730.2kiB/s
2021-01-14 15:47:24  [Relaychain] ✨ Imported #294 (0xbd35…8364)
2021-01-14 15:47:24  [Relaychain] 💤 Idle (3 peers), best: #294 (0xbd35…8364), finalized #290 (0x1109…ea3d), ⬇ 175.6kiB/s ⬆ 181.4kiB/s
2021-01-14 15:47:25  [Parachain] 💤 Idle (0 peers), best: #0 (0x755b…42ca), finalized #0 (0x755b…42ca), ⬇ 727.8kiB/s ⬆ 736.7kiB/s
```

## Parachain Registration
We have our relay chain launched and our parachain collator ready to go. Now we have to register the parachain on the relay chain. In the live Polkadot network, this will be accomplished with parachain auctions. But today we will do it with Sudo.

### Registration Transaction
The transaction can be submitted from `Apps > Sudo > parasSudoWrapper > sudoScheduleParaInitialize`
with the following parameters:

- id: `200`
- genesisHead: upload the file `para-200-genesis` (from the previous step)
- validationCode: upload the file `para-200-wasm` (from the previous step)
- parachain: Yes

### Block Production

The collator should start producing parachain blocks (aka collating) once the registration is
successful. The collator should start producing log messages like the following:

```
2021-01-14 16:09:54  [Relaychain] ✨ Imported #519 (0x7c22…71b8)
2021-01-14 16:09:54  [Relaychain] Starting collation for relay parent 0x7c22474df9f10b44aed7616c3ad9aef4d0db82e8421a81cbc3c10e63569971b8 on parent 0x4d77beb48b42979b070e0e81357f66629da194faa0f72be0bb70ee6828c220d0.
2021-01-14 16:09:54  [Relaychain] 🙌 Starting consensus session on top of parent 0x4d77beb48b42979b070e0e81357f66629da194faa0f72be0bb70ee6828c220d0
2021-01-14 16:09:54  [Relaychain] 🎁 Prepared block for proposing at 18 [hash: 0x8cb3aa750b83e1dfc120c81243e8d7fdb3f6926adfe79b977ec7d8f4a5f7bb7b; parent_hash: 0x4d77…20d0; extrinsics (3): [0x9d73…3794, 0xd860…3108, 0x6fdb…0112]]
2021-01-14 16:09:54  [Relaychain] Produced proof-of-validity candidate 0x67b91f2a3e0cc82d0b18a2ec31212081853b24e5c8f7de98d39fabfd89f46bee from block 0x8cb3aa750b83e1dfc120c81243e8d7fdb3f6926adfe79b977ec7d8f4a5f7bb7b.
2021-01-14 16:09:54  [Parachain] ✨ Imported #18 (0x8cb3…bb7b)
2021-01-14 16:09:54  [Relaychain] 💤 Idle (4 peers), best: #519 (0x7c22…71b8), finalized #516 (0x982f…d9cf), ⬇ 239.4kiB/s ⬆ 239.6kiB/s
2021-01-14 16:09:55  [Parachain] 💤 Idle (0 peers), best: #17 (0x4d77…20d0), finalized #15 (0x25ec…a10b), ⬇ 633.5kiB/s ⬆ 622.3kiB/s
2021-01-14 16:09:59  [Relaychain] 💤 Idle (4 peers), best: #519 (0x7c22…71b8), finalized #517 (0x6852…ec17), ⬇ 216.3kiB/s ⬆ 216.6kiB/s
2021-01-14 16:10:00  [Relaychain] ✨ Imported #520 (0x0ecb…4dba)
2021-01-14 16:10:00  [Parachain] 💤 Idle (0 peers), best: #17 (0x4d77…20d0), finalized #16 (0xd7e0…ae67), ⬇ 503.7kiB/s ⬆ 494.3kiB/s
2021-01-14 16:10:04  [Relaychain] 💤 Idle (4 peers), best: #520 (0x0ecb…4dba), finalized #518 (0x15df…f3fa), ⬇ 282.0kiB/s ⬆ 275.3kiB/s
2021-01-14 16:10:05  [Parachain] 💤 Idle (0 peers), best: #17 (0x4d77…20d0), finalized #16 (0xd7e0…ae67), ⬇ 605.2kiB/s ⬆ 595.0kiB/s
```

### Run in Docker

First, install [Docker](https://docs.docker.com/get-docker/) and
[Docker Compose](https://docs.docker.com/compose/install/).

Then run the following command to start a single node development chain.

```bash
./scripts/docker_run.sh
```

This command will firstly compile your code, and then start a local development network. You can
also replace the default command (`cargo build --release && ./target/release/node-template --dev --ws-external`)
by appending your own. A few useful ones are as follow.

```bash
# Run Substrate node without re-compiling
./scripts/docker_run.sh ./target/release/node-template --dev --ws-external

# Purge the local dev chain
./scripts/docker_run.sh ./target/release/node-template purge-chain --dev

# Check whether the code is compilable
./scripts/docker_run.sh cargo check
```
