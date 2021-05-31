#!/bin/bash

# Run
# ./script/generate-parachain-specs.sh => current commit
# ./script/generate-parachain-specs.sh <tag>

TAG=$(git rev-parse --short HEAD)

if [ $1 ]; then
  TAG=$1
fi

WORKDIR=$(pwd)/node/res
DOCKER_WORKDIR="/data"

chmod 777 $WORKDIR
IMG_ID=$(docker run -it --rm -v $WORKDIR:/$DOCKER_WORKDIR -d tungicetea/polkafoundry:halongbay-29be30f)

INPUT=$DOCKER_WORKDIR/halongbay.json
docker exec -it $IMG_ID sh -c "
  /polkafoundry/polkafoundry export-genesis-state --parachain-id 1111 --chain $INPUT > $DOCKER_WORKDIR/genesis-state &&
  /polkafoundry/polkafoundry export-genesis-wasm --chain $INPUT > $DOCKER_WORKDIR/genesis-wasm.wasm"

docker stop $IMG_ID
