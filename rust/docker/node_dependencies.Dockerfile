# Use Alpine Linux as the base image
FROM alpine:latest AS base

# Install build dependencies
RUN apk add --no-cache curl gcc musl-dev openssl-dev lld

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Create a new empty shell project
RUN USER=root cargo new --bin app
WORKDIR /app

# Copy our manifests
COPY rust/examples/example_node/Cargo.toml ./
COPY rust/fabric /fabric

# Vendor dependencies
RUN cargo vendor

# Configure cargo to use vendored dependencies
RUN mkdir .cargo
RUN cat <<EOF > .cargo/config.toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

# Build only the dependencies to cache them
RUN cargo build --release --offline

# Remove the source code
RUN rm -rf src

# Copy the actual source code
COPY rust/examples/example_node/src ./src

# Build for release
RUN cargo build --release --offline

# Final stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache libgcc curl gcc musl-dev openssl-dev lld

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy the build artifact, source code, and vendored dependencies
COPY --from=base /app/target/release/example_node /usr/local/bin/
COPY --from=base /app /app
COPY --from=base /root/.cargo /root/.cargo

# Add fabric library
COPY --from=base /fabric /fabric

# Set the working directory
WORKDIR /app/example_node

# The resulting image will have all dependencies and source code
