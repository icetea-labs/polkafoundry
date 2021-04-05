#!/bin/bash

cargo build --release
docker build -f docker/Dockerfile . -t polkafoundry-image

WORKDIR=$(pwd)/specs/halongbay-v1
chmod 777 $WORKDIR
IMG_ID=$(docker run -it --rm -v $WORKDIR:/generator -d polkafoundry-image)
#sleep 1
docker exec -it $IMG_ID sh /generator/docker-run.sh
docker stop $IMG_ID