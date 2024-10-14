# Use Alpine Linux as the base image
FROM alpine:latest

# Install Python and necessary build tools
RUN apk add --no-cache python3 py3-pip gcc musl-dev libffi-dev openssl-dev

# Create and activate a virtual environment
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Create a directory for our application
WORKDIR /app

# Copy our pyproject.toml and poetry.lock files
COPY python/pyproject.toml python/poetry.lock ./

# Install Poetry and dependencies
RUN pip install --no-cache-dir poetry \
    && poetry config virtualenvs.create false \
    && poetry install --no-dev

# Copy the application code
COPY python/fabric ./fabric
COPY python/setup.py python/README.md ./

# Install the fabric library
RUN pip install -e .

# The resulting image will have all dependencies installed and source code
