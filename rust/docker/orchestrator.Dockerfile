# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/rust_orchestrator_dependencies:latest

# Copy the actual source code
COPY rust/examples/example_orchestrator/src ./src

# Rebuild the application
RUN cargo build --release --offline

# Copy the config file
COPY rust/examples/example_orchestrator/config.yaml /usr/local/bin/config.yaml

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/example_orchestrator"]

# Set the RUST_LOG environment variable
ENV RUST_LOG=info
