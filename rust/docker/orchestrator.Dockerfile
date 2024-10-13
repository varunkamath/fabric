# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/rust_orchestrator_dependencies:latest AS base-image

# Copy our actual source code
COPY rust/examples/example_orchestrator /app
COPY rust/fabric /fabric

# Build our application
WORKDIR /app
RUN cargo build --release --offline

# Create a new stage for a smaller final image
FROM base-image

# Copy the built executable from the builder stage
COPY --from=0 /app/target/release/example_orchestrator /usr/local/bin/example_orchestrator

# Copy the config file
COPY rust/examples/example_orchestrator/config.yaml /usr/local/bin/config.yaml

# Set the entrypoint to our application
ENTRYPOINT ["example_orchestrator"]

# Set the RUST_LOG environment variable
ENV RUST_LOG=info
