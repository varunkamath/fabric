# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/python_node_dependencies:latest

# Set the working directory in the container
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY ./python /app

# Make port 7447 available to the world outside this container
EXPOSE 7447

# Run the node when the container launches
CMD ["python", "-m", "fabric.node.node"]