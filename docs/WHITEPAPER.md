# BYCONN Protocol Specification & Whitepaper

## 1. Vision
The BYCONN blockchain aims to solve the blockchain trilemma by providing a highly scalable, secure, and decentralized infrastructure. Through an optimized Proof-of-Stake (PoS) BFT consensus engine and an EVM-compatible execution layer, BYCONN supports high-throughput decentralized applications (dApps) without compromising on censorship resistance and network security.

## 2. Native Coin
The native cryptocurrency of the BYCONN network is the **BYC** coin.
**Utility of BYC**:
- **Gas Fees**: Used to pay for transaction execution and smart contract interactions.
- **Staking**: Bonded by validators and delegators to secure the network.
- **Governance**: Used as voting power in on-chain governance proposals.

## 3. Tokenomics
- **Ticker**: BYC
- **Total Supply**: 1,000,000,000 BYC (Fixed, no inflation beyond this cap).
- **Distribution**:
  - 40% (400M) - Staking & Validator Rewards
  - 20% (200M) - Core Team & Contributors (subject to vesting)
  - 20% (200M) - Ecosystem Development Fund
  - 20% (200M) - Public Sale & Initial Liquidity
- **Deflationary Mechanism**: A portion of the base transaction fee is permanently burned, applying deflationary pressure as network usage increases.

## 4. Consensus
BYCONN utilizes a Byzantine Fault Tolerant (BFT) Proof-of-Stake consensus mechanism.
- **Finality**: Single-slot finality. A block is finalized once >2/3 of validator voting power signs it.
- **Block Time**: Approximately 3 seconds.
- **Rounds**: A proposer is pseudo-randomly selected based on stake weight. If a proposer fails to produce a block, a timeout triggers a view change to the next proposer.

## 5. Block Structure
A BYCONN block consists of:
- **Header**:
  - `parent_hash`: Hash of the previous block.
  - `state_root`: Merkle-Patricia tree root of the state database.
  - `transactions_root`: Merkle tree root of transactions in the block.
  - `receipts_root`: Merkle tree root of transaction receipts.
  - `timestamp`: Unix timestamp of block creation.
  - `proposer`: Address of the validator proposing the block.
- **Body**: List of Transactions.
- **Signatures**: Aggregated BLS signatures from validators committing the block.

## 6. Transactions
Transactions use the Ethereum transaction format (EIP-155) for maximum compatibility.
- **Fields**: `nonce`, `gasPrice` (or `maxFeePerGas`/`maxPriorityFeePerGas`), `gasLimit`, `to`, `value`, `data`, `v, r, s` (ECDSA signature on secp256k1).
- **Validation**:
  1. Valid signature and recovery of sender address.
  2. Nonce matches the sender's account nonce.
  3. Sender has sufficient balance for `value + (gasLimit * maxFeePerGas)`.

## 7. Validators
Validators are node operators responsible for proposing and finalizing blocks.
- **Active Set**: Top 100 validators by total staked BYC (self-stake + delegated stake).
- **Staking Requirements**: Minimum self-stake of 100,000 BYC.
- **Slashing Conditions**:
  - *Equivocation (Double Signing)*: Slashing of 5% of staked funds and permanent jailing.
  - *Downtime*: Missing blocks over a prolonged period results in a minor slashing (0.1%) and temporary jailing to protect network liveness.
- **Rewards**: Distributed proportionally to stake at the end of each epoch (e.g., every 1000 blocks).

## 8. Governance
BYCONN employs on-chain governance for protocol upgrades and ecosystem fund allocation.
- **Proposals**: Any address can submit a proposal by locking a minimum deposit of 1,000 BYC.
- **Voting**: 1 BYC staked = 1 Vote.
- **Thresholds**: Requires a quorum of 40% of all staked BYC and a >50% majority approval to pass.

## 9. Gas Model
Based on EIP-1559 mechanics:
- **Base Fee**: Dynamically adjusts based on network congestion. The Base Fee is entirely burned.
- **Priority Fee**: An optional tip paid directly to the block proposer to incentivize inclusion during congestion.
- **Computation Accounting**: Standard EVM opcodes and gas costs apply.

## 10. Smart Contracts
BYCONN features full Ethereum Virtual Machine (EVM) compatibility.
- Existing Solidity and Vyper contracts can be deployed without modification.
- Accesses state through the same trie structures used in Ethereum.
- Standard Web3 RPC endpoints are supported for seamless integration with tools like MetaMask, Hardhat, and Foundry.

## 11. Security Assumptions
- **Byzantine Fault Tolerance**: The network remains secure and live as long as fewer than 1/3 of the validator voting power is malicious or offline.
- **Cryptography**: Assumes the security of secp256k1 for transaction signatures, Keccak256 for hashing, and BLS12-381 for consensus signature aggregation.
- **Network Synchrony**: Assumes partial synchrony for liveness, but guarantees safety even in asynchronous network partitions.

## 12. Upgrade Process
- **Soft/Hard Forks**: Governed by the on-chain voting mechanism.
- **Implementation**: Upon a successful governance vote, an upgrade block height is set. Node operators must upgrade their software binaries before this height.
- **State Migrations**: For structural changes to the state, migration logic is baked into the new node software to execute deterministically at the upgrade block.
