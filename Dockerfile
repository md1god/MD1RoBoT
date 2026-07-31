FROM rust:1.77-slim-bookworm

# تثبيت الأدوات الأساسية + zstd (إصلاح خطأ Ollama)
RUN apt-get update && apt-get install -y \
    curl \
    pkg-config \
    libssl-dev \
    zstd \
    && rm -rf /var/lib/apt/lists/*

# تثبيت Ollama (بعد توفير zstd)
RUN curl -fsSL https://ollama.com/install.sh | sh

WORKDIR /app
COPY Cargo.toml .
COPY src/ src/

RUN cargo build --release

VOLUME ["/app/memory", "/app/backups"]

CMD ["sh", "-c", "ollama serve & sleep 5 && ollama pull qwen2.5:14b && ./target/release/genesis_core"]
