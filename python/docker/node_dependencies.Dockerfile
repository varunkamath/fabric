# Use Alpine Linux as the base image
FROM alpine:latest

# Install Python and necessary build tools
RUN apk add --no-cache python3 py3-pip gcc musl-dev libffi-dev openssl-dev

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

# Copy the application code
COPY python/examples ./examples

# The resulting image will have all dependencies installed and source code
