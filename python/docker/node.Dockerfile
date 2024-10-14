# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/python_node_dependencies:latest

# Set the working directory
WORKDIR /app

# Reinstall the application (in case of changes)
RUN pip install -e .

# Set environment variables
ENV PYTHONUNBUFFERED=1
ENV RUST_LOG=info

# Run the quadcopter node when the container launches
CMD ["python", "examples/example_quadcopter_node.py"]
