#!/bin/bash

# Set the GitHub Container Registry and repository name
REGISTRY="ghcr.io"
REPO_NAME="varunkamath/fabric"

# Build the Rust sensor node dependencies
docker build -t ${REGISTRY}/${REPO_NAME}/rust_node_dependencies:latest -f rust/docker/node_dependencies.Dockerfile .

# Build the Rust orchestrator dependencies
docker build -t ${REGISTRY}/${REPO_NAME}/rust_orchestrator_dependencies:latest -f rust/docker/orchestrator_dependencies.Dockerfile .

# Build the Python sensor node dependencies
docker build -t ${REGISTRY}/${REPO_NAME}/python_node_dependencies:latest -f python/docker/node_dependencies.Dockerfile .

# Build the Python orchestrator dependencies
docker build -t ${REGISTRY}/${REPO_NAME}/python_orchestrator_dependencies:latest -f python/docker/orchestrator_dependencies.Dockerfile .

echo "All containers built successfully!"
