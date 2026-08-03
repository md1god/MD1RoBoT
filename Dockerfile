FROM rust:latest AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
COPY src/ src/
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libssl-dev ca-certificates \
    python3 python3-pip nodejs npm gcc g++ nasm binutils \
    && rm -rf /var/lib/apt/lists/*
RUN pip3 install semgrep --break-system-packages
COPY --from=builder /app/target/release/md1robot /app/md1robot
WORKDIR /app
VOLUME ["/app/memory", "/app/backups", "/root/.ollama"]
EXPOSE 8080
CMD ["/app/md1robot"]
