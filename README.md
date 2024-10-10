# fabric

![CI Status](https://github.com/varunkamath/fabric/workflows/CI%20%2F%20CD/badge.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Python Version](https://img.shields.io/badge/python-3.12.7-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.81.0-orange.svg)

#### Framework Agnostically Bridging Resilient Interconnected Components

`fabric` is a robust framework for building networks of autonomous agents, providing a flexible and scalable solution for creating, managing, and orchestrating distributed systems.

## Features

- **Node Management**: Create and manage individual nodes with customizable configurations.
- **Orchestration**: Centralized control and monitoring of multiple nodes.
- **Real-time Communication**: Utilizes Zenoh for efficient, real-time data exchange between nodes and the orchestrator.
- **Dynamic Configuration**: Nodes can receive and apply configuration updates at runtime.
- **Health Monitoring**: Automatic health checks and status updates for all nodes.
- **Fault Tolerance**: Detects and handles node failures, with automatic offline status updates after 10 seconds of inactivity.
- **Extensible**: Easily create custom node types by implementing the `NodeInterface` trait.
- **Cross-Language Compatibility**: Supports both Python and Rust implementations, allowing for mixed-language deployments.

## Project Structure

- `rust/`
  - `fabric/`: Core Rust library for the fabric system
    - `src/`: Source code for the fabric library
    - `tests/`: Integration tests for the fabric library
  - `examples/`: Example implementations using the fabric library
    - `example_node/`: Example of a Rust node implementation
    - `example_orchestrator/`: Example of a Rust orchestrator implementation
- `python/`
  - `fabric/`: Core Python library for the fabric system
    - `node/`: Node implementation
    - `orchestrator/`: Orchestrator implementation
    - `tests/`: Unit and integration tests
  - `examples/`: Example implementations using the fabric library
    - `example_quadcopter_node.py`: Example of a Python node implementation
- `docker/`: Dockerfiles for building containers
- `.github/workflows/`: CI/CD configuration

## Prerequisites

- Docker
- Kubernetes cluster (Minikube for local testing, or a multi-node cluster for distributed setup)
- kubectl
- Rust (for Rust components)
- Python 3.12.7 (for Python components)
- Helm (for deploying with Kubernetes)

## Building the Project

1. Build all services:

   ```bash
   ./build_services.bash
   ```

   This script builds Docker images for both Rust and Python components.

## Deploying with Kubernetes

### Local Deployment

1. Start Minikube:

   ```
   minikube start
   ```

2. Set up Minikube to use its Docker daemon:

   ```
   eval $(minikube -p minikube docker-env)
   ```

3. Apply the local Kubernetes configuration:

   ```
   kubectl apply -f local-sensor-network.yaml
   ```

### Distributed Deployment

1. Ensure your Kubernetes cluster spans across the target hosts.

2. Label your nodes appropriately (host1, host2, host3, host4):

   ```
   kubectl label nodes <node-name> kubernetes.io/hostname=host1
   ```

   Repeat for each host.

3. Deploy using Helm:

   ```
   helm install fabric ./fabric-chart -f values.yaml
   ```

## Monitoring and Debugging

1. Check the status of your pods:

   ```
   kubectl get pods
   ```

2. View logs from a specific pod:

   ```
   kubectl logs <pod-name>
   ```

3. To interact with the control node service:

   ```
   kubectl port-forward service/control-node-service 7447:7447
   ```

4. Using k9s:

   ```
   k9s
   ```

## Development

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality and consistency. To set up pre-commit hooks:

1. Install pre-commit:

   ```
   pip install pre-commit
   ```

2. Install the git hook scripts:

   ```
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
