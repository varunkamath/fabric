# Use Ubuntu 24.04 as the base image
FROM ubuntu:24.04

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# Update and install necessary packages
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Get the latest version of uv
COPY --from=ghcr.io/astral-sh/uv:latest /uv /bin/uv

# Initialize the virtual environment
RUN uv venv /opt/venv
# Use the virtual environment automatically
ENV VIRTUAL_ENV=/opt/venv
# Place entry points in the environment at the front of the path
ENV PATH="/opt/venv/bin:$PATH"

# Create a directory for our application
WORKDIR /app

# Copy our pyproject.toml and poetry.lock files
COPY python/pyproject.toml python/poetry.lock ./

# Install Python dependencies
RUN uv pip install --no-cache-dir poetry
RUN poetry config virtualenvs.create false
RUN poetry install --no-dev

# The resulting image will have all dependencies installed
