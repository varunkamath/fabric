# fabric

![CI Status](https://github.com/varunkamath/fabric/workflows/CI%20%2F%20CD/badge.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Python Version](https://img.shields.io/badge/python-3.12.7-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.81.0-orange.svg)

#### Framework Agnostically Bridging Resilient Interconnected Components

`fabric` is a robust framework for building networks of autonomous agents, providing a flexible and scalable solution for creating, managing, and orchestrating distributed systems.

## Features

- Hybrid Rust and Python implementation for optimal performance and flexibility
- Zenoh-based communication for efficient and reliable data exchange
- Kubernetes-ready with Helm charts for easy deployment
- Support for both Rust and Python nodes
- Configurable node behavior and dynamic reconfiguration
- Telemetry data collection and publishing
- Command handling for node control
- Heartbeat mechanism for node health monitoring

## Project Structure

- `rust/`
  - `fabric/`: Core Rust library for the fabric system
  - `examples/`: Example implementations using the fabric library
    - `example_node/`: Example of a Rust node implementation (Quadcopter)
    - `example_orchestrator/`: Example of a Rust orchestrator implementation
- `python/`
  - `fabric/`: Core Python library for the fabric system
  - `examples/`: Example implementations using the fabric library
    - `example_quadcopter_node.py`: Example of a Python node implementation
- `helm/`: Helm charts for Kubernetes deployment
- `docker/`: Dockerfiles for building containers
- `.github/workflows/`: CI/CD configuration

## Prerequisites

- Docker
- k3d
- kubectl
- Helm
- Rust (for Rust components)
- Python 3.12.7 (for Python components)

## Building the Project

1. Build all services:

   ```bash
   ./build_services.bash
   ```

   This script builds Docker images for both Rust and Python components.

## Deploying with k3d and Helm

1. Create a k3d cluster:

   ```bash
   k3d cluster create fabric-cluster
   ```

2. Build and load images into the k3d cluster:

   ```bash
   ./deploy_images.bash
   ```

3. Deploy using Helm:

   ```bash
   ./deploy_fabric.bash
   ```

   This script will create a new k3d cluster if it doesn't exist, load the images, and deploy the Helm chart.

## Monitoring and Debugging

1. Check the status of your pods:

   ```bash
   kubectl get pods
   ```

2. View logs from a specific pod:

   ```bash
   kubectl logs <pod-name>
   ```

3. To interact with the control node service:

   ```bash
   kubectl port-forward service/control-node-service 7447:7447
   ```

## Development

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality and consistency. To set up pre-commit hooks:

1. Install pre-commit:

   ```bash
   pip install pre-commit
   ```

2. Install the git hook scripts:

   ```bash
   pre-commit install
   ```

### Running Tests

To run the Python tests:

```bash
pytest
```

To run the Rust tests:

```bash
cd rust/fabric
cargo test
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
