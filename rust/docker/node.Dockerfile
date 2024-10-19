# Use the dependencies image as the base
FROM ghcr.io/varunkamath/fabric/rust_node_dependencies:latest AS builder

# Set the working directory
WORKDIR /app/app

# Copy the actual source code
COPY rust/examples/example_node/ ./

# Rebuild the application
RUN cargo build --release --offline

# Final stage: create a minimal image
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache libgcc

# Copy the build artifact
COPY --from=builder /app/app/target/release/example_node /usr/local/bin/

# Set the entrypoint with the environment variable
ENTRYPOINT ["/bin/sh", "-c", "QUADCOPTER_ID=${QUADCOPTER_ID:-quadcopter-rust-$(uuidgen | cut -d'-' -f1)} /usr/local/bin/example_node"]

# Set the RUST_LOG environment variable
ENV RUST_LOG=info
