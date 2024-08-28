# Use the dependencies image as the base
FROM rust_control_node_dependencies:latest AS base-image

# Use the rust image as the builder
FROM rust:1.80.1 AS builder

# Copy our actual source code
COPY rust/examples/example_orchestrator /app

# Copy the vendored dependencies from the base image
COPY --from=base-image /app/vendor /app/vendor
COPY --from=base-image /app/.cargo /app/.cargo

# Build our application
WORKDIR /app
RUN cargo build --release --offline --jobs 8

# Create a new stage for a smaller final image
FROM base-image

# Copy the built executable from the builder stage
COPY --from=builder /app/target/release/example_orchestrator /usr/local/bin/example_orchestrator

# Copy the config file
COPY rust/examples/example_orchestrator/config.yaml /usr/local/bin/config.yaml

# Set the entrypoint to our application
ENTRYPOINT ["example_orchestrator"]
