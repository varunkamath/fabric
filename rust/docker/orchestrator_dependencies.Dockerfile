# Use Ubuntu 24.04 as the base image
FROM ubuntu:24.04 AS base

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

# Use rust:latest as builder
FROM rust:latest AS builder

# Install lld
RUN apt-get update && apt-get install -y lld && rm -rf /var/lib/apt/lists/*

# Copy our actual source code
COPY rust /app/rust

# Build our application
WORKDIR /app/rust/examples/example_orchestrator
RUN cargo build --release

# Create the final image
FROM base

# Create a directory for our application
WORKDIR /app/example_orchestrator

# Copy the built executable, dependencies, and fabric directory from the builder stage
COPY --from=builder /app/rust/examples/example_orchestrator/target/release/example_orchestrator /usr/local/bin/example_orchestrator
COPY --from=builder /app/rust/examples/example_orchestrator/Cargo.toml /app/rust/examples/example_orchestrator/Cargo.lock ./
COPY --from=builder /app/rust/fabric /fabric

# Use cargo vendor to download and cache dependencies
RUN cargo vendor
RUN mkdir .cargo
RUN cargo vendor > .cargo/config.toml

# Remove the source code and build artifacts
RUN rm -rf src target

# The resulting image will have all dependencies downloaded and cached
