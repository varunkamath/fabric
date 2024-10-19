# fabric

![CI Status](https://github.com/varunkamath/fabric/workflows/CI%20%2F%20CD/badge.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Python Version](https://img.shields.io/badge/python-3.13.0-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.82.0-orange.svg)

#### Framework Agnostically Bridging Resilient Interconnected Components

`fabric` is a robust framework for building networks of autonomous agents, providing a flexible and scalable solution for creating, managing, and orchestrating distributed systems.

## Features

- Hybrid Rust and Python implementation for optimal performance and flexibility
- Zenoh-based communication for efficient and reliable data exchange
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

## Example Deployment (Kubernetes with Helm)

This project includes an example deployment using Kubernetes and Helm charts. This is just one possible way to deploy a fabric-based system.

- `helm/`: Helm charts for example Kubernetes deployment
- `docker/`: Dockerfiles for building example containers
- `.github/workflows/`: CI/CD configuration for the example deployment

### Prerequisites for Example Deployment

- Docker
- k3d
- kubectl
- Helm
- Rust (for Rust components)
- Python 3.12.7 (for Python components)

### Building and Deploying the Example

1. Build all services:

   ```bash
   ./build_services.bash
   ```

2. Deploy the fabric system:

   ```bash
   ./deploy_fabric.bash
   ```

   This script will create a k3d cluster, set up a local registry, and deploy the Helm chart.

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
pytest python/tests
```

### To run the Rust tests:

```bash
cargo test
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
