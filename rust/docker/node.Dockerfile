# Use the dependencies image as the base
FROM rust_node_dependencies:latest AS base-image

# Use the rust image as the builder
FROM rust:1.80.1 AS builder

# Copy our actual source code
COPY rust/examples/example_node /app

# Copy the vendored dependencies from the base image
COPY --from=base-image /app/vendor /app/vendor
COPY --from=base-image /app/.cargo /app/.cargo

# Build our application
WORKDIR /app
RUN cargo build --release --offline

# Create a new stage for a smaller final image
FROM base-image

# Copy the built executable from the builder stage
COPY --from=builder /app/target/release/example_node /usr/local/bin/example_node

# Set the entrypoint to our application
ENTRYPOINT ["example_node"]
