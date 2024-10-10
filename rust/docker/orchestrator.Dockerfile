# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/rust_orchestrator_dependencies:latest AS base-image

# Use the rust image as the builder
FROM rust:1.80.1 AS builder

# Copy our actual source code
COPY rust/examples/example_orchestrator /app

RUN apt update && apt install -y lld

# Copy the vendored dependencies from the base image
COPY --from=base-image /app/example_orchestrator/vendor /app/example_orchestrator/vendor
COPY --from=base-image /app/example_orchestrator/.cargo /app/example_orchestrator/.cargo

# Add local dependency `fabric`
COPY rust/fabric /fabric

# Build our application
WORKDIR /app/example_orchestrator
RUN cargo build --release --offline

# Create a new stage for a smaller final image
FROM base-image

# Copy the built executable from the builder stage
COPY --from=builder /app/target/release/example_orchestrator /usr/local/bin/example_orchestrator

# Copy the config file
COPY rust/examples/example_orchestrator/config.yaml /usr/local/bin/config.yaml

# Set the entrypoint to our application
ENTRYPOINT ["example_orchestrator"]
