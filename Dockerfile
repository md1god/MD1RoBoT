# ========== MD1RoBoT - بيئة التطور الذاتي متعدد اللغات ==========
FROM rust:1.86-slim-bookworm

# ---- أدوات النظام الأساسية ----
RUN apt-get update && apt-get install -y \
    curl pkg-config libssl-dev zstd \
    # أدوات البناء والتحليل للغات متعددة
    python3 python3-pip python3-venv \
    nodejs npm \
    gcc g++ make \
    # أدوات تحليل ثابت إضافية
    semgrep \
    # أدوات عامة
    git \
    && rm -rf /var/lib/apt/lists/*

# ---- تثبيت أدوات Python الأساسية (اختياري) ----
RUN pip3 install --no-cache-dir pylint black

# ---- تثبيت Ollama (لتشغيل النماذج محلياً) ----
RUN curl -fsSL https://ollama.com/install.sh | sh

# ---- إعداد بيئة العمل ----
WORKDIR /app

# نسخ ملفات المشروع
COPY Cargo.toml .
COPY src/ src/

# بناء المشروع (Rust)
RUN cargo build --release

# ---- إعداد Ollama والنماذج الأولية (سيتم تشغيلها عند بدء الحاوية) ----
# سنقوم بتنزيل النماذج في وقت التشغيل لنضمن وجود الخادم
# CMD مخصص لبدء كل شيء

# ---- مجلدات دائمة ----
VOLUME ["/app/memory", "/app/backups", "/root/.ollama"]

# ---- تشغيل الخدمات وتنزيل النماذج ثم تشغيل المحرك ----
CMD ["sh", "-c", "\
    echo '🚀 بدء Ollama...' && \
    ollama serve & \
    sleep 5 && \
    echo '📥 تنزيل النماذج المتخصصة...' && \
    ollama pull qwen2.5-coder:7b && \
    ollama pull deepseek-coder:6.7b && \
    echo '✅ البيئة جاهزة، تشغيل MD1RoBoT...' && \
    ./target/release/md1robot"]
