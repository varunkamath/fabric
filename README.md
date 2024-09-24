# fabric

![CI Status](https://github.com/varunkamath/fabric/workflows/CI%20%2F%20CD/badge.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Python Version](https://img.shields.io/badge/python-3.12.5-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.80.1-orange.svg)

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
    - `src/`: Source code for the fabric library
    - `tests/`: Integration tests for the fabric library
  - `examples/`: Example implementations using the fabric library
    - `example_node/`: Example of a Python node implementation
    - `example_orchestrator/`: Example of a Python orchestrator implementation
- `.github/workflows/`: CI/CD configuration

## Prerequisites

- Docker
- Kubernetes cluster (Minikube for local testing, or a multi-node cluster for distributed setup)
- kubectl
- Rust (for Rust components)
- Python 3.x (for Python components)

## Building the Project

1. Build the Rust sensor node:

   ```
   docker build -t rust_node_dependencies:latest -f rust/docker/node_dependencies.Dockerfile .
   docker build -t rust_node:latest -f rust/docker/node.Dockerfile .
   ```

2. Build the Python sensor node:

   ```
   docker build -t python_node_dependencies:latest -f python/docker/node_dependencies.Dockerfile .
   docker build -t python_node:latest -f python/docker/node.Dockerfile .
   ```

3. Build the control node:
   ```
   docker build -t rust_orchestrator_dependencies:latest -f rust/docker/orchestrator_dependencies.Dockerfile .
   docker build -t rust_orchestrator:latest -f rust/docker/orchestrator.Dockerfile .
   ```

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

   You may need to rebuild above images here, as pull policy is set to `Never`

3. Apply the local Kubernetes configuration:

   ```
   kubectl apply -f local-sensor-network.yaml
   ```

### Distributed Deployment

1. Ensure your Kubernetes cluster spans across the 4 target hosts.

2. Label your nodes appropriately (host1, host2, host3, host4):

   ```
   kubectl label nodes <node-name> kubernetes.io/hostname=host1
   ```

   Repeat for each host.

3. Apply the distributed Kubernetes configuration:

   ```
   kubectl apply -f distributed-sensor-network.yaml
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

   k9s is a terminal-based UI to interact with your Kubernetes clusters. It's a powerful tool for managing and monitoring your Kubernetes resources.

   To install k9s, follow the instructions on the [official k9s GitHub page](https://github.com/derailed/k9s).

   To use k9s:

   ```
   k9s
   ```

   This will open the k9s interface. You can navigate through your resources, view logs, and execute commands directly from this interface.

## Changing Configurations

To change the sensor configurations:

1. Edit the `config.yaml` section in the `local-sensor-network.yaml` file.
2. Apply the changes:
   ```
   kubectl apply -f local-sensor-network.yaml
   ```
3. The control node will automatically publish the new configurations to the sensors.

## Custom Local Deployment

To set up a custom local deployment with a specific number of hosts and sensors per host:

1. Edit the `local-sensor-network.yaml` file:

   - Adjust the `replicas` field in the `rust-sensor` and `python-sensor` StatefulSets to set the number of sensors per type.
   - Add or remove sensor configurations in the `control-node-config` ConfigMap.

2. Create a custom `values.yaml` file for Helm:

   ```yaml
   hosts:
     - name: host1
       sensors:
         rust: 2
         python: 2
     - name: host2
       sensors:
         rust: 3
         python: 1
     # Add more hosts as needed

   controlNode:
     replicas: 1

   sensorConfig:
     rust-sensor-0:
       sampling_rate: 5
       threshold: 50.0
     rust-sensor-1:
       sampling_rate: 10
       threshold: 75.0
     # Add more sensor configurations as needed
   ```

3. Create a Helm chart:

   ```
   helm create fabric-chart
   ```

4. Replace the contents of `fabric-chart/templates/deployment.yaml` with your modified `local-sensor-network.yaml`, using Helm templating syntax to make it dynamic based on the `values.yaml`.

5. Deploy your custom configuration:

   ```
   helm install fabric ./fabric-chart -f values.yaml
   ```

This approach allows you to easily customize the number of hosts, sensors per host, and their configurations using a single `values.yaml` file.

Remember to adjust your Dockerfiles and build processes if you need to make changes to the sensor or control node implementations.

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

3. (Optional) Run against all files:
   ```
   pre-commit run --all-files
   ```

The pre-commit configuration can be found in `.pre-commit-config.yaml`.

## Cleaning Up

To remove the deployment:

```
kubectl delete -f local-sensor-network.yaml
minikube stop
```

or

```
kubectl delete -f distributed-sensor-network.yaml
```
