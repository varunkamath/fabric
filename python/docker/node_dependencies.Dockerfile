# Use Ubuntu 24.04 as the base image
FROM ubuntu:24.04

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# Update and install necessary packages
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-venv \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create and activate a virtual environment
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Create a directory for our application
WORKDIR /app

# Copy our pyproject.toml, poetry.lock, setup.py, and README.md files
COPY python/pyproject.toml python/poetry.lock python/setup.py python/README.md ./

# Copy the fabric library source code
COPY python/fabric ./fabric

# Install Poetry
RUN pip install --no-cache-dir poetry

# Install Python dependencies and the fabric library
RUN poetry config virtualenvs.create false \
    && poetry install --only main \
    && pip install -e .

# Install eclipse-zenoh
RUN pip install eclipse-zenoh==0.11.0

# The resulting image will have all dependencies installed
