#!/bin/bash

WORKDIR="/generator"
INPUT=$WORKDIR/halongbaySpec.json

/polkafoundry/polkafoundry export-genesis-state --parachain-id 14 --chain $INPUT > $WORKDIR/genesis-state
/polkafoundry/polkafoundry export-genesis-wasm --chain $INPUT > $WORKDIR/genesis-wasm.wasm