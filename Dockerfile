FROM rust:1.96.1

# تثبيت الحزم الأساسية المطلوبة للنظام
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

# 1. تثبيت إصدار Solana CLI المطلوب
RUN sh -c "$(curl -sSfL https://release.solana.com/v4.1.0-beta.3/install)"

# ضبط مسار الـ PATH لضمان وصول النظام لجميع أدوات سولانا (بما فيها cargo-build-sbf)
ENV PATH="/root/.local/share/solana/install/active_release/bin:/root/.cargo/bin:$PATH"

# 2. تثبيت AVM لجلب نسخة anchor-cli 1.1.2 الجاهزة بدون الحاجة لبنائها من المصدر
RUN cargo install --git https://github.com/coral-xyz/anchor avm --locked --force

RUN avm install 1.1.2
RUN avm use 1.1.2

# إضافة مسار AVM إلى الـ PATH
ENV PATH="/root/.avm/bin:$PATH"

WORKDIR /app
COPY . .

# 3. تشغيل البناء مع استخدام --ignore-keys لتخطي خطأ اختلاف مفاتيح البرنامج
RUN anchor build --ignore-keys
