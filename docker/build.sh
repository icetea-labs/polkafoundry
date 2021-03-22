#!/usr/bin/env bash
set -e

pushd .

# The following line ensure we run from the project root
PROJECT_ROOT=`git rev-parse --show-toplevel`
cd $PROJECT_ROOT

# Find the current version from Cargo.toml
# VERSION=`grep "^version" ./Cargo.toml | egrep -o "([0-9\.]+)"`
GITUSER=public.ecr.aws/o0b6j5s9/polkafoundry
TAG=halongbay-v1

# Build the image
echo "Building ${GITUSER}/${TAG}:latest docker image, hang on!"
time docker build -f ./docker/Dockerfile --build-arg RUSTC_WRAPPER= --build-arg PROFILE=release -t ${GITUSER}:${TAG} --no-cache .

# Show the list of available images for this repo
echo "Image is ready"
docker images | grep ${TAG}

popd