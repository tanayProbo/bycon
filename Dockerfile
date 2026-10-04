FROM rust:1.77-slim as builder
WORKDIR /app

# Install build dependencies if needed (e.g. for secp256k1 or rocksdb if added later)
RUN apt-get update && apt-get install -y pkg-config libssl-dev build-essential

COPY . .

# Run cargo check to compile and verify all crates in the workspace
RUN cargo check --workspace

# Also test everything
RUN cargo test --workspace

# Finally build the release binaries
RUN cargo build --release --workspace

# If it succeeds, the node binary will be at /app/target/release/node
