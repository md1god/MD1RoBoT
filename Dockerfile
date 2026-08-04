FROM rust:1-bookworm AS builder
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
RUN chmod +x /app/md1robot && (ldd /app/md1robot || true)
COPY config.toml /app/config.toml
WORKDIR /app
VOLUME ["/app/memory", "/app/backups", "/root/.ollama"]
EXPOSE 10000
CMD ["sh", "-c", "echo '--- MD1RoBoT boot ---'; /app/md1robot; code=$?; echo \"--- process ended, exit code $code ---\"; exit $code"]
