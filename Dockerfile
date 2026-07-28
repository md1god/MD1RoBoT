FROM rust:1.96.1

# تثبيت الحزم الأساسية وأدوات النظام
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libssl-dev \
    pkg-config \
    git \
    libudev-dev \
    llvm \
    clang \
    && rm -rf /var/lib/apt/lists/*

# تثبيت Solana CLI بالإصدار المطلوب
RUN sh -c "$(curl -sSfL https://release.solana.com/v4.1.0-beta.3/install)"
ENV PATH="/root/.local/share/solana/install/active_release/bin:$PATH"

# تثبيت Anchor CLI بالإصدار 1.1.2
RUN cargo install --git https://github.com/coral-xyz/anchor --tag v1.1.2 anchor-cli --locked

WORKDIR /app
COPY . .

# تنفيذ عملية البناء البرمجي للبرنامج
RUN anchor build
