# MD1usd Solana Program

MD1$ is a 1:1 asset-backed token on Solana, built with Anchor. This repository contains the on-chain program source code for mint and redeem operations.

- **Program ID:** `3fN2LAt47q3oSgNq4dJZt4DuAh5yJw6mb6B3dRYJGHa8`
- **Network:** Solana mainnet-beta
- **Framework:** Anchor (`anchor-lang`/`anchor-spl` 0.30.1)
- **Backing asset:** USDC (`Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`)

## What this program does

- `initialize` — creates the MD1$ mint and its Metaplex metadata account.
- `mint` — accepts the backing asset into a program-owned vault and mints an equal amount of MD1$ to the user.
- `redeem` — burns MD1$ from the user and returns the backing asset from the vault.

Mint/vault authorities are program-derived addresses (PDAs), not externally controlled keys.

## Build

This program was built and deployed with:

- `rustc 1.96.1`
- `solana-cli 4.1.0-beta.3`
- `anchor-cli 1.1.2`

```bash
anchor build
```

The resulting `.so` file is committed at `programs/md1usd_solana/target/deploy/md1usd_solana.so` for reference and independent verification.

## Verification status

The on-chain program hash matches the committed build artifact exactly (confirmed via `sha256sum` against `solana program dump`, see `.github/workflows/` for the automated check).

Automated third-party verification (`solana-verify` / OtterSec) currently cannot reproduce this build, because their Docker build images are pinned to an older Rust toolchain (`rustc 1.81.0` as of this writing) than the one used for this build (`rustc 1.96.1`, released June 2026). This is a tooling gap on the verifier side, not a mismatch in the source code. See the open issue tracking this: *(link added once filed)*.

## Repository structure

```
programs/md1usd_solana/   Program source (src/lib.rs, instructions, state, errors)
Anchor.toml                Anchor workspace configuration
Cargo.toml / Cargo.lock    Rust workspace and locked dependency versions
```

## Security

See [SECURITY.md](./SECURITY.md) for the security model and how to report a vulnerability.
