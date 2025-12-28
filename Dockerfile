# 1. Build Stage
FROM rust:latest as builder

WORKDIR /app

# Install CMake (Required for rdkafka)
RUN apt-get update && apt-get install -y cmake build-essential

# Create a blank project
RUN cargo new --bin backend
WORKDIR /app/backend

# Copy manifests
COPY ./Cargo.toml ./Cargo.toml

# Build dependencies
RUN cargo build --release
RUN rm src/*.rs

# 2. Copy source code
COPY ./src ./src

# 3. Touch main to force rebuild
RUN touch src/main.rs

# 4. Build the app
RUN cargo build --release


# 5. Runtime Stage: Changed from 'bookworm-slim' to 'testing-slim'
# This provides the newer glibc version (2.38+) required by the Rust compiler
FROM debian:testing-slim

# Install OpenSSL
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/backend/target/release/backend /usr/local/bin/backend

CMD ["backend"]
