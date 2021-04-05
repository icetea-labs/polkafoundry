#!/bin/bash

# Run
# ./docker/generator.sh => current commit
# ./docker/generator.sh <tag>

TAG=$(git rev-parse --short HEAD)

if [ $1 ]
then
  TAG=$1
fi

WORKDIR=$(pwd)/specs/halongbay-v1
DOCKER_WORKDIR="/generator"

chmod 777 $WORKDIR
IMG_ID=$(docker run -it --rm -v $WORKDIR:/$DOCKER_WORKDIR -d public.ecr.aws/o0b6j5s9/polkafoundry:$TAG)

INPUT=$DOCKER_WORKDIR/halongbaySpec.json
docker exec -it $IMG_ID sh -c "
  /polkafoundry/polkafoundry export-genesis-state --parachain-id 14 --chain $INPUT > $DOCKER_WORKDIR/genesis-state &&
  /polkafoundry/polkafoundry export-genesis-wasm --chain $INPUT > $DOCKER_WORKDIR/genesis-wasm.wasm"

docker stop $IMG_ID