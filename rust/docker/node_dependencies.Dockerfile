# Use rust:alpine as the base image
FROM rust:alpine

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev lld

# Create a new empty shell project
WORKDIR /app
RUN cargo new --bin app
WORKDIR /app/app

# Delete the source code
RUN rm -rf src

# Copy our manifests and fabric library
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

# # Build only the dependencies to cache them
# RUN cargo build --release --offline

# Remove the source code to prepare for the next stage
RUN rm -rf src

# Set the working directory
WORKDIR /app/app

# The resulting image will have vendored dependencies and can be used for offline builds
