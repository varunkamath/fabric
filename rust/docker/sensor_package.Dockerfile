# Use the dependencies image as the base
FROM rust_sensor_node_dependencies:latest AS base-image

# Use the rust image as the builder
FROM rust:1.80.1 AS builder

# Copy the vendored dependencies from the base image
COPY --from=base-image /app/vendor /app/vendor
COPY --from=base-image /app/.cargo /app/.cargo

# Copy our actual source code
COPY rust/sensor_node /app

# Build our application
WORKDIR /app
RUN cargo build --release --offline --jobs 8

# Create a new stage for a smaller final image
FROM base-image

# Copy the built executable from the builder stage
COPY --from=builder /app/target/release/sensor_node /usr/local/bin/sensor_node

# Set the entrypoint to our application
ENTRYPOINT ["sensor_node"]
