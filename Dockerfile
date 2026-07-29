# Build stage
FROM rust:1.89.0 as builder

WORKDIR /app

# Install Anchor CLI and dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install anchor-cli --locked

# Copy workspace manifests
COPY Cargo.toml Cargo.lock rust-toolchain.toml Anchor.toml ./
COPY programs ./programs

# Build the project
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy build artifacts from builder
COPY --from=builder /app/target/release /app/target/release
COPY --from=builder /usr/local/cargo/bin/anchor /usr/local/bin/anchor
COPY Anchor.toml ./
COPY programs ./programs

# Use anchor as the default entry point
ENTRYPOINT ["anchor"]
CMD ["--version"]
