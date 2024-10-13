#!/bin/bash

# Set variables
NAMESPACE="default"
RELEASE_NAME="fabric"
CHART_PATH="helm/fabric-chart"
VALUES_FILE="helm/fabric-chart/values.yaml"
CLUSTER_NAME="fabric-cluster"  # Add this line for your k3d cluster name

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check for required commands
if ! command_exists kubectl || ! command_exists helm || ! command_exists k3d; then
    echo "Error: kubectl, helm, and k3d are required to run this script."
    exit 1
fi

# Stop and delete the existing k3d cluster if it exists
if k3d cluster list | grep -q "$CLUSTER_NAME"; then
    echo "Stopping and deleting existing k3d cluster..."
    k3d cluster delete "$CLUSTER_NAME"
fi

# Create a new k3d cluster
echo "Creating new k3d cluster..."
k3d cluster create "$CLUSTER_NAME"

# Wait for the cluster to be ready
echo "Waiting for k3d cluster to be ready..."
kubectl wait --for=condition=ready node --all --timeout=60s

# Delete existing pods
echo "Deleting existing pods..."
kubectl delete pods --all -n $NAMESPACE

# Uninstall the existing Helm release if it exists
if helm list -q | grep -q "^$RELEASE_NAME$"; then
    echo "Uninstalling existing Helm release..."
    helm uninstall $RELEASE_NAME -n $NAMESPACE
fi

# Wait for pods to be deleted
echo "Waiting for pods to be deleted..."
kubectl wait --for=delete pod --all -n $NAMESPACE --timeout=60s

# Run deploy_images.bash to rebuild and load images
echo "Building and loading images..."
./deploy_images.bash

# Deploy or upgrade the Helm chart
echo "Deploying Helm chart..."
helm upgrade --install $RELEASE_NAME $CHART_PATH -f $VALUES_FILE -n $NAMESPACE

# Wait for pods to be ready
echo "Waiting for pods to be ready..."
kubectl wait --for=condition=ready pod --all -n $NAMESPACE --timeout=300s

echo "Fabric system deployed successfully!"

# Display pod status
echo "Current pod status:"
kubectl get pods -n $NAMESPACE
