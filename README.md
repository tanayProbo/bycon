# BYCON Blockchain

> A from-scratch, Ethereum-inspired blockchain node built in Rust. Implements Proof-of-Work mining, secp256k1 ECDSA transaction signing (EIP-155), a binary Merkle tree for block commitments, a sled-backed world-state database, an in-memory mempool, BFT consensus scaffolding, a JSON-RPC server, and a standalone wallet binary.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Workspace Layout](#workspace-layout)
3. [Protocol Layer (`protocol/`)](#protocol-layer-protocol)
   - [Hashing](#hashing)
   - [Accounts](#accounts)
   - [Transactions & Signatures](#transactions--signatures)
   - [Receipts & Logs](#receipts--logs)
   - [Blocks](#blocks)
   - [Merkle Tree](#merkle-tree)
   - [Genesis](#genesis)
4. [Node Layer (`node/`)](#node-layer-node)
   - [State Database](#state-database)
   - [Blockchain Storage](#blockchain-storage)
   - [Mempool](#mempool)
   - [Execution Engine](#execution-engine)
   - [Miner (PoW)](#miner-pow)
   - [Consensus Engine (BFT)](#consensus-engine-bft)
   - [Sync Manager](#sync-manager)
   - [JSON-RPC Server](#json-rpc-server)
   - [P2P Network](#p2p-network)
5. [Wallet (`wallet/`)](#wallet-wallet)
6. [CLI (`cli/`)](#cli-cli)
7. [Mathematical Operations & Formulas](#mathematical-operations--formulas)
8. [Hard Limits & Constants](#hard-limits--constants)
9. [Data Type Bounds](#data-type-bounds)
10. [Known Stubs / Roadmap](#known-stubs--roadmap)
11. [Local Setup & Running](#local-setup--running)
12. [Running Without Local Setup](#running-without-local-setup)
13. [Testing](#testing)
14. [Contributing](#contributing)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        BYCON Node                           │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │  Miner   │  │Consensus │  │   Sync   │  │  JSON-RPC │  │
│  │  (PoW)   │  │  (BFT)   │  │ Manager  │  │  :8545    │  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬─────┘  │
│       │              │              │               │        │
│  ┌────▼──────────────▼──────────────▼───────────────▼────┐  │
│  │                  Execution Engine                      │  │
│  └────────────────────────┬───────────────────────────────┘  │
│                           │                                  │
│  ┌──────────────┐  ┌──────▼──────┐  ┌────────────────────┐  │
│  │  Blockchain  │  │  State DB   │  │      Mempool       │  │
│  │  (sled KV)   │  │  (sled KV)  │  │  (HashMap in-mem)  │  │
│  └──────────────┘  └─────────────┘  └────────────────────┘  │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Protocol Library (shared types)          │   │
│  │  Account · Transaction · Block · Receipt · Trie · Hash│   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
         ▲                                      ▲
    ┌────┴─────┐                         ┌──────┴──────┐
    │  Wallet  │                         │     CLI     │
    │ (binary) │                         │  (binary)   │
    └──────────┘                         └─────────────┘
```

All shared types live in `protocol/`. The `node/` binary links against it and runs the full stack. `wallet/` and `cli/` are standalone binaries that also link against `protocol/`.

---

## Workspace Layout

```
bycon/
├── Cargo.toml          # Workspace root — members: protocol, node, wallet, cli
├── protocol/           # Shared types, crypto primitives, encoding
│   └── src/
│       ├── lib.rs          # re-exports all public modules
│       ├── hash.rs         # keccak256 wrapper
│       ├── account.rs      # Account struct + EMPTY_CODE_HASH / EMPTY_STORAGE_ROOT
│       ├── transaction.rs  # Transaction struct, ECDSA sign/recover (EIP-155)
│       ├── receipt.rs      # Receipt + Log structs
│       ├── block.rs        # BlockHeader + Block structs, header hashing
│       ├── trie.rs         # Binary Merkle tree (Phase 1)
│       └── genesis.rs      # get_genesis_block()
├── node/               # Runnable node binary
│   └── src/
│       ├── main.rs         # Tokio async entry point
│       ├── state.rs        # World-state KV (sled), account CRUD
│       ├── chain.rs        # Block append / lookup with PoW validation
│       ├── mempool.rs      # Pending transaction pool
│       ├── execution.rs    # Transaction execution, gas accounting
│       ├── miner.rs        # PoW mining loop
│       ├── consensus.rs    # BFT state machine scaffolding
│       ├── sync.rs         # Block sync manager
│       ├── rpc.rs          # JSON-RPC 2.0 server (jsonrpsee)
│       ├── network.rs      # P2P stub
│       └── evm.rs          # EVM stub
├── wallet/             # Runnable wallet binary
│   └── src/main.rs         # Key generation, signing, RLP output
├── cli/                # CLI binary (WIP)
├── docs/               # Specs, architecture, contribution guide
├── explorer/           # Block explorer (frontend + backend)
├── sdk/                # Client SDKs
├── tests/              # Integration / fuzz / perf tests
├── scripts/            # Deployment and maintenance scripts
├── genesis/            # Genesis configs (currently empty — inline in protocol)
└── docker/             # Dockerfiles and docker-compose
```

---

## Protocol Layer (`protocol/`)

### Hashing

**File:** `protocol/src/hash.rs`

Every hash in BYCON is a **Keccak-256** digest, exactly as in Ethereum.

```rust
pub type Hash = H256;   // 32-byte big-endian value

pub fn keccak256(data: &[u8]) -> Hash {
    // uses tiny_keccak::Keccak::v256()
}
```

- Output size: **256 bits / 32 bytes**
- Used for: block header hashing, transaction signing, address derivation, Merkle tree nodes, code hashes, storage roots
- Library: `tiny_keccak` (pure Rust, no OpenSSL dependency)

---

### Accounts

**File:** `protocol/src/account.rs`

Mirrors Ethereum's account model exactly:

```
Account {
    nonce        : u64     // tx count; prevents replay attacks
    balance      : U256    // BYC balance in smallest unit (wei-equivalent)
    code_hash    : H256    // keccak256(contract bytecode), or EMPTY_CODE_HASH for EOAs
    storage_root : H256    // Merkle root of contract storage trie, or EMPTY_STORAGE_ROOT
}
```

Two sentinel constants (hardcoded, verified by unit test):

| Constant | Value | Meaning |
|---|---|---|
| `EMPTY_CODE_HASH` | `keccak256(b"")` = `0xc5d246...a470` | marks an EOA (no code) |
| `EMPTY_STORAGE_ROOT` | `keccak256(RLP([]))` = `0x56e81f...b421` | marks empty storage |

Serialization: **RLP** (via `rlp_derive`) for on-disk storage, **JSON** (via `serde`) for RPC responses.

---

### Transactions & Signatures

**File:** `protocol/src/transaction.rs`

A transaction carries:

```
Transaction {
    nonce      : u64    // sender's current nonce
    gas_price  : U256   // price per gas unit in wei-equivalent
    gas_limit  : u64    // max gas the sender is willing to spend
    to         : Option<Address>  // None = contract deployment
    value      : U256   // BYC amount to transfer
    data       : Vec<u8>// calldata or contract bytecode
    v          : u64    // signature recovery id + chain_id encoding
    r          : U256   // ECDSA signature R (32 bytes)
    s          : U256   // ECDSA signature S (32 bytes)
}
```

#### Signing (EIP-155)

1. RLP-encode `[nonce, gas_price, gas_limit, to, value, data, chain_id, 0, 0]` (9 fields with chain ID)
2. `hash = keccak256(RLP_bytes)`
3. Sign with `secp256k1::sign_ecdsa_recoverable(hash, secret_key)` → `(recovery_id, [r_bytes || s_bytes])`
4. Compute `v`:
   ```
   v = recovery_id + chain_id × 2 + 35     (EIP-155, replay-protected)
   v = recovery_id + 27                    (legacy, no chain ID)
   ```
5. Store `v`, `r = U256::from_big_endian(data[0..32])`, `s = U256::from_big_endian(data[32..64])`

#### Sender Recovery

Reverse of signing:
```
chain_id = (v - 35) / 2           if v >= 35
rec_id   = v - (chain_id×2 + 35)  if v >= 35
rec_id   = v - 27                 if v < 35
hash     = keccak256(RLP(tx fields, chain_id?, 0, 0))
pubkey   = secp256k1::recover(hash, sig, rec_id)
address  = keccak256(pubkey_uncompressed[1..])[12..32]  // right 20 bytes
```

---

### Receipts & Logs

**File:** `protocol/src/receipt.rs`

```
Receipt {
    tx_hash              : H256           // hash of the originating tx
    success              : bool           // true = success, false = reverted
    gas_used             : u64            // gas consumed by this tx
    cumulative_gas_used  : u64            // running total in the block
    contract_address     : Option<Address>// set if this tx deployed a contract
    logs                 : Vec<Log>       // events emitted during execution
    effective_gas_price  : U256           // gas_price paid
}

Log {
    address : Address    // emitting contract
    topics  : Vec<H256>  // up to 4 indexed fields (event signature + args)
    data    : Vec<u8>    // ABI-encoded non-indexed fields
}
```

Receipts are RLP-encoded and hashed into `BlockHeader.receipts_root` via the Merkle tree, allowing light clients to verify transaction outcomes without downloading full blocks.

---

### Blocks

**File:** `protocol/src/block.rs`

```
BlockHeader {
    parent_hash       : H256     // hash of previous block header
    number            : u64      // block height
    nonce             : u64      // PoW nonce
    state_root        : H256     // Merkle root of world state (H256::zero for now)
    transactions_root : H256     // Merkle root of all txs in this block
    receipts_root     : H256     // Merkle root of all receipts
    timestamp         : u64      // Unix epoch seconds
    proposer          : Address  // coinbase / miner address
}

Block {
    header       : BlockHeader
    transactions : Vec<Transaction>
}
```

`Block::hash()` = `keccak256(RLP(header))` — **only the header is hashed**, not the full transaction list (same model as Ethereum).

---

### Merkle Tree

**File:** `protocol/src/trie.rs`

Phase 1 uses a **simplified binary Merkle tree** (not a full Merkle-Patricia Trie):

```
Algorithm:
  1. leaf[i] = keccak256(RLP(item[i]))
  2. while len(level) > 1:
       for i in 0..len step 2:
         left  = level[i]
         right = level[i+1]  OR level[i] if i is last (odd duplication)
         next_level.push(keccak256(left || right))   // 64-byte concat
  3. root = level[0]

  Empty list → Hash::zero()
```

**Used for:**
- `compute_transactions_root(txs)` → `BlockHeader.transactions_root`
- `compute_receipts_root(receipts)` → `BlockHeader.receipts_root`

**Important:** Order matters. `merkle_root([A, B]) ≠ merkle_root([B, A])`. Transaction ordering is preserved from the mempool's priority queue output.

A full **Merkle-Patricia Trie (MPT)** will replace this in Phase 2 for Ethereum compatibility.

---

### Genesis

**File:** `protocol/src/genesis.rs`

`get_genesis_block()` returns block number `0`. The node inserts it on first boot if the chain DB is empty. The genesis block skips the PoW check in `Blockchain::append_block()`.

---

## Node Layer (`node/`)

### State Database

**File:** `node/src/state.rs`

The **world state** maps `Address → Account`, backed by [sled](https://github.com/spacejam/sled) (an embedded key-value store written in Rust).

```
Key layout:  b"acct_" || address_bytes   (25 bytes total)
Value:       RLP-encoded Account
```

Operations:
- `get_account(addr)` — returns `Account::default()` (zero balance, zero nonce) for unknown addresses (matches Ethereum semantics)
- `put_account(addr, account)` — serializes via RLP and writes
- `delete_account(addr)` — removes the key
- `increment_nonce(addr)` — atomic read-modify-write on the nonce field

> **Note:** There is currently no Merkle root computed over the state; `BlockHeader.state_root` is always `H256::zero()`. A full MPT-backed state trie is planned for Phase 2.

---

### Blockchain Storage

**File:** `node/src/chain.rs`

Blocks are stored in a separate sled instance:

```
Key layout:  b"blk_" || height_as_big_endian_u64   (12 bytes)
Value:       bincode-serialized BlockRecord { block: Block }
```

`append_block()` enforces the PoW rule for all blocks with `number > 0`:

```
if block.hash().as_bytes()[0] != 0x00 {
    return Err(ChainError::InvalidPoW);
}
```

`latest_height()` scans the sled index in reverse to find the highest key — O(1) due to sled's B-tree ordering.

---

### Mempool

**File:** `node/src/mempool.rs`

An in-memory `HashMap<TxKey, Transaction>` where `TxKey = (sender_address, nonce)`.

**Admission rules:**
1. `tx.nonce >= current_nonce` — rejects stale nonces
2. If slot `(sender, nonce)` already occupied: new `gas_price` must be **strictly greater** to replace
3. Pool size must be below `max_size` (set to **1024** at startup in `main.rs`)

**Ordering for mining:** `ordered_txs()` sorts by:
```
PRIMARY:    gas_price DESC    (highest paying first — priority ordering)
SECONDARY:  nonce ASC         (correct execution order per sender)
```

---

### Execution Engine

**File:** `node/src/execution.rs`

The core of BYCON's state transition function. Executes one transaction atomically:

#### Step-by-step execution

```
1.  sender = secp256k1_recover(tx)
2.  assert tx.gas_limit >= 21,000          (BASE_TX_GAS — minimum for any tx)
3.  assert tx.nonce == sender.nonce        (prevents replay)
4.  max_gas_cost = tx.gas_price × tx.gas_limit          (U256 saturating mul)
    upfront_cost = max_gas_cost + tx.value              (U256 saturating add)
    assert sender.balance >= upfront_cost
5.  sender.nonce   += 1
    sender.balance -= max_gas_cost         (pre-pay all gas)
6.  assert sender.balance >= tx.value      (after gas, can we afford the transfer?)
    sender.balance -= tx.value
7a. if tx.to is Some(addr):
        recipient.balance += tx.value      (simple BYC transfer)
7b. if tx.to is None OR tx.data non-empty:
        evm.execute_contract(...)          (contract call / deploy — stub)
8.  gas_used    = 21,000                   (hardcoded; no EVM opcode metering yet)
    gas_refund  = tx.gas_limit - gas_used
    refund_val  = gas_refund × tx.gas_price
    sender.balance += refund_val           (return unspent gas)
9.  miner_reward = gas_used × tx.gas_price
    miner.balance += miner_reward
10. emit Receipt { success: true, gas_used: 21000, cumulative_gas_used, ... }
```

All arithmetic is `U256` with **saturating semantics** — no panics on overflow, values clamp at `2^256 - 1`.

---

### Miner (PoW)

**File:** `node/src/miner.rs`

Runs as an infinite `tokio::spawn` background task, polling every **500 ms**.

#### Mining loop

```
1. Lock mempool; skip if empty
2. Pull ordered_txs() from mempool
3. Execute each tx via ExecutionEngine
   → track cumulative_gas, collect receipts, push valid txs
4. Skip block if no valid txs executed
5. Compute:
     transactions_root = merkle_root(RLP(tx) for each tx)
     receipts_root     = merkle_root(RLP(receipt) for each receipt)
6. Construct BlockHeader with nonce = 0
7. PoW loop:
     loop {
       hash = keccak256(RLP(header))
       if hash[0] == 0x00 { break }   ← target: first byte is zero
       header.nonce += 1
     }
8. Wrap header + txs into Block, append to Blockchain
```

**PoW difficulty:** The target requires the first byte of the block hash to be `0x00`. Since each byte value is equally probable under keccak256:
- Probability of success per attempt = **1 / 256 ≈ 0.39%**
- Expected attempts to find a valid nonce ≈ **256**
- This is a **demo-grade difficulty** — fast enough to mine in milliseconds on any CPU

---

### Consensus Engine (BFT)

**File:** `node/src/consensus.rs`

Scaffolding for a **Tendermint-style BFT** consensus protocol. Not yet wired to the mining or P2P layers.

State machine per block height:

```
Propose → PreVote → PreCommit → Commit → (next height, back to Propose)
```

- **Propose:** designated proposer broadcasts a block
- **PreVote:** validators broadcast pre-votes for the proposed block hash
- **PreCommit:** after seeing **2/3+ weighted pre-votes**, validators broadcast pre-commits
- **Commit:** after **2/3+ weighted pre-commits**, block is finalized

Vote struct carries a `Vec<u8>` for a **BLS signature** (field present, verification not yet implemented).

> **Status:** `handle_vote()` transitions are `TODO`. Signature verification, proposer rotation, and stake-weight quorum calculation are not yet implemented.

---

### Sync Manager

**File:** `node/src/sync.rs`

Manages catching up to the best known chain tip across peers.

```
Batch size: 64 blocks per tick

Algorithm:
  - Track local_tip (our highest block number)
  - Track target_height (max tip across all connected peers)
  - is_synced() = local_tip >= target_height
  - best_peer() = peer with highest tip_height > local_tip
  - next_range() = [local_tip+1 .. min(local_tip+64, target_height)]
  - tick() requests that range from best_peer, advances local_tip
```

> **Status:** `tick()` currently advances `local_tip` arithmetically without actually fetching blocks from peers. Full block download and validation is a P2P layer TODO.

---

### JSON-RPC Server

**File:** `node/src/rpc.rs`

Listens on **`http://127.0.0.1:8545`** (standard Ethereum JSON-RPC port). Uses [jsonrpsee](https://github.com/paritytech/jsonrpsee) for async RPC dispatch.

Implemented methods:

| Method | Returns | Description |
|---|---|---|
| `eth_blockNumber` | `"0x..."` (hex u64) | Current chain height |
| `eth_getBalance` | `"0x..."` (hex U256) | BYC balance of an address |
| `eth_sendRawTransaction` | `"0x..."` (tx hash hex) | Decode RLP tx, recover sender, add to mempool |

**`eth_sendRawTransaction` flow:**
1. Hex-decode the input bytes
2. RLP-decode into `Transaction`
3. `keccak256(RLP(tx))` → tx hash
4. `secp256k1_recover(tx)` → sender address
5. Lookup sender's current nonce in StateDb
6. `mempool.add(sender, tx, current_nonce)` — returns `MempoolError` if rejected

The server runs in a detached `tokio::spawn` so it doesn't block the main task.

---

### P2P Network

**File:** `node/src/network.rs`

> **Status:** Stub. `start_p2p()` is called in `main()` but is not yet implemented. Full libp2p or custom TCP-based peer discovery and block propagation is planned.

---

## Wallet (`wallet/`)

**File:** `wallet/src/main.rs`

A standalone binary for key management and transaction construction.

```
Wallet {
    secret_key : secp256k1::SecretKey   // 32 random bytes
    address    : Address                // H160, derived from public key
}
```

**Address derivation:**
```
pubkey_uncompressed = secp256k1_pubkey.serialize_uncompressed()  // 65 bytes, prefix 0x04
address = keccak256(pubkey_uncompressed[1..])[12..32]            // drop prefix, right 20 bytes
```

**Running the wallet binary:**
```bash
cargo run -p wallet
```

Output:
```
BYCON Wallet
  Address : 0x<20-byte-hex>
  Secret  : 0x<32-byte-hex>

Signed Transaction (RLP hex):
  0x<rlp-encoded-signed-tx>

Send it to the node:
  curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":["0x..."],"id":1}'
```

The wallet supports importing keys from raw 32-byte secret key arrays and fully implements EIP-155 chain ID signing.

---

## CLI (`cli/`)

Planned CLI binary for node management, wallet operations, and chain queries. Currently scaffolded; not yet fully implemented.

---

## Mathematical Operations & Formulas

| Operation | Formula | Source |
|---|---|---|
| Block hash | `keccak256(RLP(BlockHeader))` | `block.rs` |
| Tx signing hash (EIP-155) | `keccak256(RLP([nonce, gas_price, gas_limit, to, value, data, chain_id, 0, 0]))` | `transaction.rs` |
| Signature v-value (EIP-155) | `v = recovery_id + chain_id × 2 + 35` | `transaction.rs` |
| Signature v-value (legacy) | `v = recovery_id + 27` | `transaction.rs` |
| Sender address from pubkey | `address = keccak256(pubkey_uncompressed[1..])[12..32]` | `transaction.rs` |
| Upfront gas cost | `max_gas_cost = gas_price × gas_limit` (U256 saturating) | `execution.rs` |
| Total upfront cost | `upfront = max_gas_cost + value` (U256 saturating) | `execution.rs` |
| Gas refund to sender | `refund = (gas_limit − gas_used) × gas_price` | `execution.rs` |
| Miner block reward | `reward = gas_used × gas_price` | `execution.rs` |
| Merkle leaf hash | `keccak256(RLP(item))` | `trie.rs` |
| Merkle internal node | `keccak256(left_hash ‖ right_hash)` (64 bytes) | `trie.rs` |
| PoW validity check | `keccak256(RLP(header))[0] == 0x00` | `miner.rs`, `chain.rs` |
| PoW success probability | `1 / 256 ≈ 0.39%` per attempt | — |
| PoW expected attempts | `E[attempts] = 256` | — |
| Sync batch range | `[local_tip+1 .. min(local_tip+64, target)]` | `sync.rs` |

---

## Hard Limits & Constants

| Constant | Value | Source |
|---|---|---|
| `BASE_TX_GAS` | `21,000` | `execution.rs` |
| PoW target | `hash[0] == 0x00` (1 leading zero byte) | `miner.rs`, `chain.rs` |
| Mempool capacity | `1,024` transactions | `main.rs` |
| Sync batch size | `64` blocks per tick | `sync.rs` |
| RPC listen address | `127.0.0.1:8545` | `rpc.rs` |
| Miner poll interval | `500 ms` | `miner.rs` |
| Signature R length | `32 bytes` (big-endian U256) | `transaction.rs` |
| Signature S length | `32 bytes` (big-endian U256) | `transaction.rs` |
| Address length | `20 bytes` (H160) | `transaction.rs` |
| Hash length | `32 bytes` (H256) | `hash.rs` |

---

## Data Type Bounds

| Field | Rust Type | Min | Max |
|---|---|---|---|
| `balance` | `U256` | `0` | `2^256 − 1` (~1.16 × 10^77) |
| `gas_price` | `U256` | `0` | `2^256 − 1` |
| `value` | `U256` | `0` | `2^256 − 1` |
| `gas_limit` | `u64` | `21,000` (enforced) | `18,446,744,073,709,551,615` |
| `nonce` (account) | `u64` | `0` | `18,446,744,073,709,551,615` |
| `nonce` (tx) | `u64` | `0` | `18,446,744,073,709,551,615` |
| `block.number` | `u64` | `0` | `18,446,744,073,709,551,615` |
| `block.nonce` (PoW) | `u64` | `0` | `18,446,744,073,709,551,615` |
| `timestamp` | `u64` | `0` | `18,446,744,073,709,551,615` (year ~584 billion) |
| `chain_id` | `u64` | `0` | `18,446,744,073,709,551,615` |

All `U256` arithmetic in the execution engine uses **saturating operations** — no panics, values clamp at the type max.

---

## Known Stubs / Roadmap

| Component | Current State | Planned |
|---|---|---|
| **EVM** | Stub — contract calls are no-ops; deployment uses `Address::random()` instead of `keccak256(sender ‖ nonce)` | Full opcode interpreter |
| **State Root** | `H256::zero()` hardcoded in every block header | Full Merkle-Patricia Trie over world state |
| **Trie** | Simple binary Merkle tree (Phase 1) | Ethereum-compatible MPT (Phase 2) |
| **BFT Consensus** | State machine scaffolding only; `handle_vote()` is a no-op | Full Tendermint: proposer rotation, BLS aggregate signatures, 2/3+ stake quorum |
| **P2P Network** | `start_p2p()` stub | libp2p peer discovery, block/tx gossip |
| **Sync** | Advances tip counter without downloading blocks | Full block download, validation, fork choice |
| **PoW Difficulty** | Fixed 1-byte target, no adjustment | Dynamic difficulty adjustment algorithm |
| **Gas Metering** | `gas_used` hardcoded to `21,000` | Per-opcode metering in EVM |
| **CLI** | Scaffolded | Full node management + wallet commands |
| **Explorer** | Directory exists, empty | Block/tx/account explorer UI |
| **SDK** | Directory exists | Rust + JS SDKs |

---

## Local Setup & Running

**Prerequisites:** Rust 1.75+ and Cargo.

```bash
# Clone the repo
git clone https://github.com/<your-org>/bycon
cd bycon

# Build everything
cargo build

# Run all unit tests
cargo test

# Run the node (starts mining + RPC on :8545)
cargo run -p node

# Run the wallet (generates a keypair and prints a signed tx)
cargo run -p wallet
```

### Interacting with the running node

```bash
# Get current block height
curl -s -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# Get balance of an address
curl -s -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getBalance","params":["0x<address>","latest"],"id":1}'

# Send a signed transaction (use wallet binary to generate the hex)
curl -s -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":["0x<rlp-hex>"],"id":1}'
```

---

## Running Without Local Setup

### Option 1: GitHub Codespaces (Recommended)
1. Navigate to the repository on GitHub.
2. Click the green **Code** button → **Codespaces** tab → **Create codespace on main**.
3. GitHub provisions a full dev environment with Rust pre-installed. Run `cargo build` immediately.

### Option 2: Gitpod
Prefix the repo URL: `https://gitpod.io/#https://github.com/<org>/bycon`

### Option 3: Replit
Go to [Replit](https://replit.com/) → **Create Repl** → **Import from GitHub** → paste the URL → choose Rust template.

---

## Testing

```bash
# Run all tests across the workspace
cargo test

# Run tests for a single crate
cargo test -p protocol
cargo test -p node

# Run a specific test by name
cargo test -p node test_successful_transfer
```

The following modules have unit tests:

| Module | Tests Cover |
|---|---|
| `protocol/account` | EMPTY_CODE_HASH correctness, EOA default, RLP round-trip |
| `protocol/trie` | Empty root, single-item root, determinism, order sensitivity, 4-item tree |
| `protocol/receipt` | Success/failure fields, RLP encoding |
| `node/state` | Missing-account default, put/get round-trip, nonce increment |
| `node/mempool` | Add/len, underpriced rejection, pool-full, nonce-too-low, prune, ordering |
| `node/execution` | Successful transfer (balance math), invalid nonce |
| `node/sync` | Sync state, target update, peer removal, batch range, tick, full sync |
| `wallet` | Non-zero address generation, secret key round-trip, sign+recover |

---

## Contributing

Please see `docs/CONTRIBUTING.md` for coding standards and the contribution process.

- All new features must have corresponding unit tests.
- Run `cargo clippy` and `cargo fmt` before submitting a PR.
- Stub / TODO items are tracked in the roadmap table above — pick one and send a PR!
