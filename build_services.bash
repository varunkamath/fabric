#!/bin/bash

# Set the GitHub Container Registry and repository name
REGISTRY="ghcr.io"
REPO_NAME="varunkamath/fabric"

# Build the Rust node
docker build --no-cache -t ${REGISTRY}/${REPO_NAME}/rust_node:latest -f rust/docker/node.Dockerfile .

# Build the Rust orchestrator
docker build --no-cache -t ${REGISTRY}/${REPO_NAME}/rust_orchestrator:latest -f rust/docker/orchestrator.Dockerfile .

# Build the Python node
docker build --no-cache -t ${REGISTRY}/${REPO_NAME}/python_node:latest -f python/docker/node.Dockerfile .

# Build the Python orchestrator
docker build --no-cache -t ${REGISTRY}/${REPO_NAME}/python_orchestrator:latest -f python/docker/orchestrator.Dockerfile .

echo "All services built successfully!"
