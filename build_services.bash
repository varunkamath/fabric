#!/bin/bash

# Set the registry and repository name
REGISTRY="ghcr.io"
REPO_NAME="varunkamath/fabric"

# Build and tag the images
docker build -t ${REGISTRY}/${REPO_NAME}/rust_node:latest -f rust/docker/node.Dockerfile .
docker build -t ${REGISTRY}/${REPO_NAME}/rust_orchestrator:latest -f rust/docker/orchestrator.Dockerfile .
docker build --no-cache -t ${REGISTRY}/${REPO_NAME}/python_node:latest -f python/docker/node.Dockerfile .
docker build --no-cache -t ${REGISTRY}/${REPO_NAME}/python_orchestrator:latest -f python/docker/orchestrator.Dockerfile .

# Tag the images to k3d-registry.localhost:5001/varunkamath/fabric
docker tag ghcr.io/varunkamath/fabric/rust_node:latest k3d-registry.localhost:5001/varunkamath/fabric/rust_node:latest
docker tag ghcr.io/varunkamath/fabric/rust_orchestrator:latest k3d-registry.localhost:5001/varunkamath/fabric/rust_orchestrator:latest
docker tag ghcr.io/varunkamath/fabric/python_node:latest k3d-registry.localhost:5001/varunkamath/fabric/python_node:latest
docker tag ghcr.io/varunkamath/fabric/python_orchestrator:latest k3d-registry.localhost:5001/varunkamath/fabric/python_orchestrator:latest

# Push images to the k3d registry
echo "Pushing images to k3d registry..."
docker push k3d-registry.localhost:5001/varunkamath/fabric/rust_node:latest
docker push k3d-registry.localhost:5001/varunkamath/fabric/rust_orchestrator:latest
docker push k3d-registry.localhost:5001/varunkamath/fabric/python_node:latest
docker push k3d-registry.localhost:5001/varunkamath/fabric/python_orchestrator:latest

echo "All services built and pushed successfully!"
