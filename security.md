# Security Policy

## Program overview

MD1$ (program ID `3fN2LAt47q3oSgNq4dJZt4DuAh5yJw6mb6B3dRYJGHa8`) is a 1:1 asset-backed token on Solana. Minting requires depositing the backing asset (USDC) into a program-owned vault; redeeming burns MD1$ and returns the backing asset. All mint/vault authorities are program-derived addresses (PDAs) — there is no externally held admin or upgrade key used in day-to-day operation beyond the program's upgrade authority.

## Verification

The deployed program's executable hash can be independently confirmed against this repository's committed build artifact (`programs/md1usd_solana/target/deploy/md1usd_solana.so`) using:

```bash
solana program dump 3fN2LAt47q3oSgNq4dJZt4DuAh5yJw6mb6B3dRYJGHa8 onchain.so --url <RPC_URL>
sha256sum onchain.so programs/md1usd_solana/target/deploy/md1usd_solana.so
```

(Trailing zero-padding must be stripped from both files before hashing — see the `Confirm Hash Match` GitHub Actions workflow in this repository for a working example.)

## Reporting a vulnerability

If you discover a security issue in this program, please report it privately rather than opening a public issue. Open an issue on this repository requesting a private contact channel, or reach out through the contact information listed on the project's official site.

Please include:
- A clear description of the issue and its potential impact
- Steps to reproduce, if applicable
- Any relevant transaction signatures or account addresses

We will acknowledge reports as quickly as possible and work on a fix before any public disclosure.
