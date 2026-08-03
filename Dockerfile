FROM rust:1.76 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
COPY src/ src/
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libssl-dev ca-certificates \
    python3 nodejs npm gcc g++ nasm binutils semgrep \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/md1robot /app/md1robot
WORKDIR /app
VOLUME ["/app/memory", "/app/backups", "/root/.ollama"]
EXPOSE 8080
CMD ["/app/md1robot"]
