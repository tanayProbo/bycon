# BYCON Blockchain

BYCON is a highly-scalable, secure blockchain project being built iteratively with robust design and engineering practices. It features EVM compatibility, a custom consensus mechanism, and a suite of tools for node operation and wallet management.

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

## Running Virtually (No Local Setup Required)

If you don't have Rust, Docker, or enough local storage installed, you can still develop and run BYCON entirely in your browser using cloud IDEs:

### Option 1: GitHub Codespaces (Recommended)
1. Navigate to this repository on GitHub.
2. Click the green **Code** button.
3. Select the **Codespaces** tab and click **Create codespace on main**.
4. GitHub will provision a full development environment in your browser with Rust and Docker pre-installed. You can immediately run `cargo build` and `cargo run` from the built-in terminal.

### Option 2: Gitpod
1. Prefix the GitHub repository URL with `https://gitpod.io/#` (e.g., `https://gitpod.io/#https://github.com/yourusername/bycon`).
2. Log in with your GitHub account.
3. Gitpod will launch an online VS Code environment ready for Rust development.

### Option 3: Replit
1. Go to [Replit](https://replit.com/) and click **Create Repl**.
2. Choose **Import from GitHub** and paste the URL of this repository.
3. Select the Rust template. Replit will handle the environment, allowing you to run and test the code from your browser.

## Local Setup and Building
Ensure you have Rust and Cargo installed.
Run `cargo build` to build the entire workspace.
Run `cargo test` to execute tests.

## Contributing
Please see `docs/CONTRIBUTING.md` for our coding standards and contribution process.
