# Solana Program Build & Verify - Versions Matrix

This document tracks the verified and fully compatible toolchain versions for deterministic builds and Mainnet verification.

| Tool / Component | Version | Purpose / Notes |
| :--- | :--- | :--- |
| **Rust Toolchain** | `1.90.0` | Pinned via `dtolnay/rust-toolchain` for modern compiler features. |
| **Solana CLI (Agave)** | `2.1.0` | Main Agave release for modern SVM features and RPC compatibility. |
| **Anchor Framework** | `0.31.0` | Core smart contract framework. |
| **solana-verify CLI** | Latest (Crates.io) | Used for deterministic building and on-chain hash verification. |
| **Docker Base Image** | `solanafoundation/anchor:v0.31.0` | Used internally by `solana-verify` for isolated, matching bytecode compilation. |
| **Workspace Edition** | `2021` | Rust edition supported by current Solana SBF backend compiler. |
