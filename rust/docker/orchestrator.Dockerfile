# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/rust_orchestrator_dependencies:latest AS builder

# Set the working directory
WORKDIR /app/app

# Copy the actual source code
COPY rust/examples/example_orchestrator/ ./

# Rebuild the application
RUN cargo build --release --offline

# Final stage: create a minimal image
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache libgcc

# Copy the build artifact
COPY --from=builder /app/app/target/release/example_orchestrator /usr/local/bin/

# Copy the config file
COPY rust/examples/example_orchestrator/config.yaml /usr/local/bin/config.yaml

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/example_orchestrator"]

# Set the RUST_LOG environment variable
ENV RUST_LOG=info
