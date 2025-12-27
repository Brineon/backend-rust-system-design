# 1. Build Stage (CHANGED FROM 1.75 TO latest)
FROM rust:latest as builder

WORKDIR /app

# Create a blank project
RUN cargo new --bin backend
WORKDIR /app/backend

# Copy manifests
COPY ./Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
RUN cargo build --release
RUN rm src/*.rs

# 2. Copy the actual source code
COPY ./src ./src

# 3. Touch the main file to force a rebuild
RUN touch src/main.rs

# 4. Build the actual app
RUN cargo build --release

# 5. Runtime Stage
FROM debian:bookworm-slim
# Install OpenSSL (Required for Axum/Reqwest)
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/backend/target/release/backend /usr/local/bin/backend

CMD ["backend"]
