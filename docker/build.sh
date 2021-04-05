#!/bin/bash

# Run
# ./docker/build.sh

TAG=$(git rev-parse --short HEAD)

cargo build --release
docker build -f docker/Dockerfile . -t polkafoundry:$TAG

aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin public.ecr.aws/o0b6j5s9

docker tag polkafoundry:$TAG public.ecr.aws/o0b6j5s9/polkafoundry:$TAG
docker push public.ecr.aws/o0b6j5s9/polkafoundry:$TAG
