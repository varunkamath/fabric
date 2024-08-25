# fabric

![CI Status](https://github.com/varunkamath/fabric/workflows/CI%20%2F%20CD/badge.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Python Version](https://img.shields.io/badge/python-3.12.5-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.80.1-orange.svg)

#### Framework Agnostically Bridging Resilient Interconnected Components

`fabric` is a distributed sensor network system that demonstrates the integration of multiple sensor nodes with a central control node using the Zenoh communication framework. This project showcases the interoperability between Rust and Python implementations, containerization with Docker, and deployment using Kubernetes.

## Project Structure

- `rust/`
  - `sensor_node/`: Rust implementation of a sensor node
  - `control_node/`: Rust implementation of the control node
  - `docker/`: Dockerfiles for Rust components
- `python/`
  - `sensor_node/`: Python implementation of a sensor node
  - `control_node/`: Python implementation of the control node
  - `docker/`: Dockerfiles for Python components
- `local-sensor-network.yaml`: Kubernetes configuration for local deployment
- `distributed-sensor-network.yaml`: Kubernetes configuration for distributed deployment

## Features

- Multiple sensor nodes (both Rust and Python implementations)
- Central control node for orchestration
- Real-time data publishing and subscription
- Dynamic configuration updates
- Containerized deployment using Docker
- Kubernetes-based orchestration for both local and distributed setups

## Prerequisites

- Docker
- Kubernetes cluster (Minikube for local testing, or a multi-node cluster for distributed setup)
- kubectl
- Rust (for Rust components)
- Python 3.x (for Python components)

## Building the Project

1. Build the Rust sensor node:

   ```
   docker build -t rust_sensor_node_dependencies:latest -f rust/docker/sensor_dependencies.Dockerfile .
   docker build -t rust_sensor_node:latest -f rust/docker/sensor_package.Dockerfile .
   ```

2. Build the Python sensor node:

   ```
   docker build -t python_sensor_node_dependencies:latest -f python/docker/sensor_dependencies.Dockerfile .
   docker build -t python_sensor_node:latest -f python/docker/sensor_package.Dockerfile .
   ```

3. Build the control node:
   ```
   docker build -t rust_control_node_dependencies:latest -f rust/docker/control_dependencies.Dockerfile .
   docker build -t rust_control_node:latest -f rust/docker/control_package.Dockerfile .
   ```

## Deploying with Kubernetes

### Local Deployment

1. Start Minikube:

   ```
   minikube start
   ```

2. Set up Minikube to use its Docker daemon:

   ```
   eval $(minikube docker-env)
   ```

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

## License

This project is licensed under the MIT License - see the LICENSE file for details.
