#!/bin/bash

# Set variables
REGISTRY="k3d-registry.localhost:5001"
REPOSITORY="varunkamath/fabric"

# Build and push Rust node image
docker build -t $REGISTRY/$REPOSITORY-rust-node:latest -f rust/docker/node.Dockerfile .
docker push $REGISTRY/$REPOSITORY-rust-node:latest

# Build and push Python node image
docker build -t $REGISTRY/$REPOSITORY-python-node:latest -f python/docker/node.Dockerfile .
docker push $REGISTRY/$REPOSITORY-python-node:latest

# Build and push Rust orchestrator image
docker build -t $REGISTRY/$REPOSITORY-rust-orchestrator:latest -f rust/docker/orchestrator.Dockerfile .
docker push $REGISTRY/$REPOSITORY-rust-orchestrator:latest

echo "All services built and pushed successfully!"
