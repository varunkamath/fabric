# Use the dependencies image as the base
FROM python_control_node_dependencies:latest

# Copy our actual source code
COPY ../control_node /app

# Copy the config file
COPY ../control_node/config.yaml /app/config.yaml

# Set the entrypoint to our application
ENTRYPOINT ["python3", "/app/main.py"]
