#!/bin/bash

# Set the GitHub Container Registry and repository name
REGISTRY="ghcr.io"
REPO_NAME="varunkamath/fabric"
CLUSTER_NAME="fabric-cluster"

# Array of image names
IMAGES=(
    "rust_node"
    "rust_orchestrator"
    "python_node"
    "python_orchestrator"
)

# Function to load an image into the k3d cluster
load_image() {
    local image_name=$1
    local full_image_name="${REGISTRY}/${REPO_NAME}/${image_name}:latest"

    echo "Loading ${full_image_name} into k3d cluster..."
    k3d image import ${full_image_name} -c ${CLUSTER_NAME}

    if [ $? -eq 0 ]; then
        echo "Successfully loaded ${full_image_name}"
    else
        echo "Failed to load ${full_image_name}"
        exit 1
    fi
}

# Main script execution
echo "Starting to load images into k3d cluster ${CLUSTER_NAME}..."

# Check if the cluster exists
if ! k3d cluster list | grep -q ${CLUSTER_NAME}; then
    echo "Error: Cluster ${CLUSTER_NAME} does not exist. Please create it first."
    exit 1
fi

# Load each image
for image in "${IMAGES[@]}"; do
    load_image $image
done

echo "All images have been loaded into the k3d cluster."
