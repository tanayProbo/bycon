# BYCON Blockchain

BYCON is a highly-scalable, secure blockchain project being built iteratively with robust design and engineering practices.

## Project Structure
- `protocol/`: Core protocol definitions, constants, and cryptographic primitives.
- `node/`: The main node implementation (P2P, consensus, state management).
- `wallet/`: Wallet core logic, key management, and transaction signing.
- `cli/`: Command-line interface for the node and wallet.
- `docs/`: Protocol specifications, architecture diagrams, and developer guides.
- `explorer/`: Backend/frontend for the blockchain explorer.
- `sdk/`: SDKs for interacting with the blockchain.
- `tests/`: Integration, performance, and fuzz tests.
- `scripts/`: Utility scripts for deployment, testing, and maintenance.
- `genesis/`: Genesis block configurations and related tools.
- `docker/`: Dockerfiles and docker-compose.yml for containerized environments.

## Setup and Building
Ensure you have Rust and Cargo installed.
Run `cargo build` to build the entire workspace.
Run `cargo test` to execute tests.

## Contributing
Please see `docs/CONTRIBUTING.md` for our coding standards and contribution process.
