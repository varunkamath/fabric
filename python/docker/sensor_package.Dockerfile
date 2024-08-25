# Use the dependencies image as the base
FROM python_sensor_node_dependencies:latest

# Copy our actual source code
COPY python/sensor_node /app

# Set the entrypoint to our application
ENTRYPOINT ["python3", "/app/main.py"]
