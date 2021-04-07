#!/bin/bash

# Run
# ./docker/build.sh

TAG=$(git rev-parse --short HEAD)

docker build -f ../docker/Dockerfile . -t polkafoundry:$TAG
CODE=$?

if [ "$CODE" -eq "0" ]; then
  aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin public.ecr.aws/o0b6j5s9
  docker tag polkafoundry:$TAG public.ecr.aws/o0b6j5s9/polkafoundry:$TAG
  docker push public.ecr.aws/o0b6j5s9/polkafoundry:$TAG
else
  echo "Build image not success. exit with code: $CODE"
fi
