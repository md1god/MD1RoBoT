FROM rust:1.86-slim-bookworm

RUN apt-get update && apt-get install -y \
    curl pkg-config libssl-dev zstd \
    python3 python3-pip python3-venv \
    nodejs npm \
    gcc g++ make \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN pip3 install --no-cache-dir --break-system-packages pylint black

RUN curl -fsSL https://ollama.com/install.sh | sh

WORKDIR /app

COPY Cargo.toml .
COPY src/ src/

RUN cargo build --release

VOLUME ["/app/memory", "/app/backups", "/root/.ollama"]

CMD ["sh", "-c", "\
    echo 'Starting Ollama...' && \
    ollama serve & \
    sleep 5 && \
    echo 'Pulling models...' && \
    ollama pull qwen2.5-coder:7b && \
    ollama pull deepseek-coder:6.7b && \
    echo 'Environment ready, launching MD1RoBoT...' && \
    ./target/release/md1robot"]
