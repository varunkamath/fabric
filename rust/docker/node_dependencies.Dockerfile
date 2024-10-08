# Use Ubuntu 24.04 as the base image
FROM ubuntu:24.04

# Avoid prompts from apt
ENV DEBIAN_FRONTEND=noninteractive

# Update and install necessary packages
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    pkg-config \
    libssl-dev \
    lld \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Create a directory for our application
WORKDIR /app/example_node

# Copy our Cargo.toml
COPY rust/examples/example_node/Cargo.toml Cargo.lock ./

# Create a dummy src/main.rs file
RUN mkdir src && echo "fn main() {println!(\"Hello, world!\");}" > src/main.rs

# Add local dependency `fabric`
COPY rust/fabric /fabric

# Build dependencies to ensure they are all downloaded
RUN cargo build --release

# Use cargo vendor to download and cache dependencies
RUN mkdir .cargo
RUN cargo vendor > .cargo/config.toml

# Remove the dummy src directory and target directory
RUN rm -rf src target

# The resulting image will have all dependencies downloaded and cached
