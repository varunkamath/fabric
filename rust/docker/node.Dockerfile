# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/rust_node_dependencies:latest

# Copy the actual source code
COPY rust/examples/example_node/src ./src

# Rebuild the application
RUN cargo build --release --offline

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/example_node"]

# Set the RUST_LOG environment variable
ENV RUST_LOG=info
