# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/python_orchestrator_dependencies:latest

# Set the working directory
WORKDIR /app

# Reinstall the application (in case of changes)
RUN pip install -e .

# Run the orchestrator when the container launches
CMD ["python", "-m", "fabric.orchestrator.orchestrator"]
