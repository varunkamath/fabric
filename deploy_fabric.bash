#!/bin/bash

# Set variables
NAMESPACE="default"
RELEASE_NAME="fabric"
CHART_PATH="helm/fabric-chart"
VALUES_FILE="helm/fabric-chart/values.yaml"
CLUSTER_NAME="fabric-cluster"
REGISTRY_NAME="k3d-registry.localhost"
REGISTRY_PORT=5001

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check for required commands
if ! command_exists kubectl || ! command_exists helm || ! command_exists k3d; then
    echo "Error: kubectl, helm, and k3d are required to run this script."
    exit 1
fi

# Create the registry if it doesn't exist
if ! k3d registry list | grep -q "$REGISTRY_NAME"; then
    echo "Creating k3d registry..."
    k3d registry create $REGISTRY_NAME --port $REGISTRY_PORT
    echo "Waiting for registry to be ready..."
    sleep 10
fi

# After creating the registry and before building images
echo "Verifying registry accessibility..."
if curl -s -f -o /dev/null http://k3d-registry.localhost:5001/v2/; then
    echo "Registry is accessible."
else
    echo "Error: Registry is not accessible. Please check your setup."
    exit 1
fi

# Stop and delete the existing k3d cluster if it exists
if k3d cluster list | grep -q "$CLUSTER_NAME"; then
    echo "Stopping and deleting existing k3d cluster..."
    helm uninstall $RELEASE_NAME
    k3d cluster delete "$CLUSTER_NAME"
fi

# Create a new k3d cluster with a built-in registry
echo "Creating new k3d cluster with built-in registry..."
if ! k3d cluster create "$CLUSTER_NAME" \
    --registry-use "$REGISTRY_NAME:$REGISTRY_PORT" \
    --k3s-arg '--kubelet-arg=container-log-max-size=10Mi@server:*' \
    --k3s-arg '--kubelet-arg=container-log-max-files=5@server:*' ; then
    echo "Failed to create k3d cluster. Exiting."
    exit 1
fi

# Wait for the cluster to be ready
echo "Waiting for k3d cluster to be ready..."
kubectl wait --for=condition=ready node --all --timeout=60s

# # Build and push images
# echo "Building and pushing images..."
# ./build_services.bash

# Add this after creating the cluster and before deploying the Helm chart
echo "Listing images in k3d registry:"
docker exec k3d-registry.localhost:5001 sh -c "ls -l /var/lib/registry/docker/registry/v2/repositories"

# Deploy or upgrade the Helm chart
echo "Deploying Helm chart..."
helm upgrade --install $RELEASE_NAME $CHART_PATH \
    --set image.registry="k3d-registry.localhost:5001" \
    --set image.repository="varunkamath/fabric" \
    --set image.tag="latest" \
    --set replicaCount.rustNode=2 \
    -f $VALUES_FILE \
    -n $NAMESPACE

# Wait for pods to be ready
echo "Waiting for pods to be ready..."
# kubectl wait --for=condition=ready pod --all -n $NAMESPACE --timeout=300s

echo "Fabric system deployed successfully!"

# Display pod status
echo "Current pod status:"
kubectl get pods -n $NAMESPACE

echo "Fabric deployment completed."

echo "Describing pods:"
kubectl describe pods -n $NAMESPACE

echo "Checking events:"
kubectl get events -n $NAMESPACE --sort-by='.metadata.creationTimestamp'
