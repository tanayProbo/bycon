# Security Policy

## Protected Files & Secrets
- **Private Keys & Wallets**: `founder-wallet.txt`, `*.key`, `*.pem`, `*.secret`, and `.env*` are strictly excluded from version control and Docker contexts via `.gitignore` and `.dockerignore`, and locked with `0600` owner-only permissions.
- **Consensus & Monetary Policy**: Core consensus rules (`protocol/src/block.rs`, `node/src/consensus.rs`, `node/src/miner.rs`, `node/src/chain.rs`) are protected by `.github/CODEOWNERS` (`@tanayProbo`) and GitHub branch protection on `main`.

## Reporting a Vulnerability
Do **not** open a public GitHub issue for security vulnerabilities. Please report security issues directly to the founder at `tanayagrawal129@gmail.com`.
