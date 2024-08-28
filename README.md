# fabric

![CI Status](https://github.com/varunkamath/fabric/workflows/CI%20%2F%20CD/badge.svg)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Python Version](https://img.shields.io/badge/python-3.12.5-blue.svg)
![Rust Version](https://img.shields.io/badge/rust-1.80.1-orange.svg)

#### Framework Agnostically Bridging Resilient Interconnected Components

`fabric` is a distributed sensor network system that demonstrates the integration of multiple sensor nodes with a central control node using the Zenoh communication framework. This project showcases the interoperability between Rust and Python implementations, with a focus on flexibility and extensibility.

## Project Structure

- `rust/`
  - `fabric/`: Core Rust library for the fabric system
    - `src/`: Source code for the fabric library
    - `tests/`: Integration tests for the fabric library
  - `examples/`: Example implementations using the fabric library
    - `example_node/`: Example of a Rust node implementation
    - `example_orchestrator/`: Example of a Rust orchestrator implementation
- `python/` (Not implemented in the current codebase)
  - `sensor_node/`: Python implementation of a sensor node
  - `control_node/`: Python implementation of the control node
- `.github/workflows/`: CI/CD configuration

## Features

- Flexible node system with support for different node types (e.g., generic, radio)
- Central orchestrator for managing and configuring nodes
- Real-time data publishing and subscription
- Dynamic configuration updates
- Plugin system for extending node functionality
- Comprehensive error handling
- Asynchronous operations using Tokio
- Integration with the Zenoh communication framework

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
