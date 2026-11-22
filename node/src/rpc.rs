//! JSON-RPC 2.0 server — Ethereum-compatible API.
//!
//! Listens on `http://127.0.0.1:8545`.
//!
//! ## Implemented methods
//! | Method                        | Description                              |
//! |-------------------------------|------------------------------------------|
//! | `eth_blockNumber`             | Current chain tip height (hex)           |
//! | `eth_getBalance`              | BYC balance for an address (hex U256)    |
//! | `eth_sendRawTransaction`      | Decode, validate, and submit a signed tx |
//! | `eth_getBlockByNumber`        | Block object by height or "latest"       |
//! | `eth_getTransactionByHash`    | Transaction object by canonical tx hash  |
//! | `eth_getTransactionCount`     | Account nonce (hex u64)                  |
//! | `eth_gasPrice`                | Current suggested gas price (hex U256)   |

use std::sync::Arc;
use tokio::sync::Mutex;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use jsonrpsee::types::error::ErrorObject;
use serde_json::{json, Value};
use rlp;
use tracing::info;
use std::net::SocketAddr;

use protocol::hash::Hash;
use protocol::transaction::{Address, Transaction};
use crate::chain::Blockchain;
use crate::mempool::Mempool;
use crate::state::StateDb;

// ── Helper ───────────────────────────────────────────────────────────────────

fn rpc_err(msg: impl ToString) -> ErrorObject<'static> {
    ErrorObject::owned(-32000, msg.to_string(), None::<()>)
}

fn parse_address(hex: &str) -> Result<Address, ErrorObject<'static>> {
    let bytes = hex::decode(hex.trim_start_matches("0x"))
        .map_err(|e| rpc_err(format!("invalid address: {e}")))?;
    if bytes.len() != 20 {
        return Err(rpc_err("address must be 20 bytes"));
    }
    Ok(Address::from_slice(&bytes))
}

fn parse_hash(hex: &str) -> Result<Hash, ErrorObject<'static>> {
    let bytes = hex::decode(hex.trim_start_matches("0x"))
        .map_err(|e| rpc_err(format!("invalid hash: {e}")))?;
    if bytes.len() != 32 {
        return Err(rpc_err("hash must be 32 bytes"));
    }
    Ok(Hash::from_slice(&bytes))
}

/// Serialize a `Transaction` to a JSON object, including its canonical hash
/// and block context (block_hash, block_number, transaction_index).
fn tx_to_json(tx: &Transaction, block_height: u64, tx_idx: u64) -> Value {
    let tx_hash = tx.tx_hash();
    let mut r_bytes = [0u8; 32];
    let mut s_bytes = [0u8; 32];
    tx.r.to_big_endian(&mut r_bytes);
    tx.s.to_big_endian(&mut s_bytes);
    let mut gas_price_bytes = [0u8; 32];
    tx.gas_price.to_big_endian(&mut gas_price_bytes);
    let mut value_bytes = [0u8; 32];
    tx.value.to_big_endian(&mut value_bytes);

    json!({
        "hash":             format!("0x{}", hex::encode(tx_hash.as_bytes())),
        "nonce":            format!("0x{:x}", tx.nonce),
        "blockNumber":      format!("0x{:x}", block_height),
        "transactionIndex": format!("0x{:x}", tx_idx),
        "from":             null, // recovered off-chain; too expensive here without caching
        "to":               tx.to.map(|a| format!("0x{:x}", a)),
        "value":            format!("0x{}", hex::encode(value_bytes)),
        "gas":              format!("0x{:x}", tx.gas_limit),
        "gasPrice":         format!("0x{}", hex::encode(gas_price_bytes)),
        "input":            format!("0x{}", hex::encode(&tx.data)),
        "v":                format!("0x{:x}", tx.v),
        "r":                format!("0x{}", hex::encode(r_bytes)),
        "s":                format!("0x{}", hex::encode(s_bytes)),
    })
}

// ── RPC trait ────────────────────────────────────────────────────────────────

#[rpc(server)]
pub trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn block_number(&self) -> RpcResult<String>;

    #[method(name = "eth_getBalance")]
    async fn get_balance(&self, address: String, block: String) -> RpcResult<String>;

    #[method(name = "eth_sendRawTransaction")]
    async fn send_raw_transaction(&self, tx_bytes: String) -> RpcResult<String>;

    #[method(name = "eth_getBlockByNumber")]
    async fn get_block_by_number(&self, block_param: String, include_txs: bool) -> RpcResult<Value>;

    #[method(name = "eth_getTransactionByHash")]
    async fn get_transaction_by_hash(&self, tx_hash: String) -> RpcResult<Value>;

    #[method(name = "eth_getTransactionCount")]
    async fn get_transaction_count(&self, address: String, block: String) -> RpcResult<String>;

    #[method(name = "eth_gasPrice")]
    async fn gas_price(&self) -> RpcResult<String>;
}

// ── Implementation ───────────────────────────────────────────────────────────

pub struct EthApiServerImpl {
    chain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
    state: Arc<Mutex<StateDb>>,
}

#[jsonrpsee::core::async_trait]
impl EthApiServer for EthApiServerImpl {
    // ── eth_blockNumber ──────────────────────────────────────────────────────

    async fn block_number(&self) -> RpcResult<String> {
        let chain = self.chain.lock().await;
        let height = chain.latest_height().map_err(|e| rpc_err(e))?;
        Ok(format!("0x{:x}", height))
    }

    // ── eth_getBalance ───────────────────────────────────────────────────────

    async fn get_balance(&self, address: String, _block: String) -> RpcResult<String> {
        let addr = parse_address(&address)?;
        let state = self.state.lock().await;
        let acc = state.get_account(&addr).map_err(|e| rpc_err(e))?;
        let mut buf = [0u8; 32];
        acc.balance.to_big_endian(&mut buf);
        Ok(format!("0x{}", hex::encode(buf)))
    }

    // ── eth_sendRawTransaction ───────────────────────────────────────────────

    async fn send_raw_transaction(&self, tx_bytes: String) -> RpcResult<String> {
        let bytes = hex::decode(tx_bytes.trim_start_matches("0x"))
            .map_err(|e| rpc_err(e))?;

        let tx: Transaction = rlp::decode(&bytes)
            .map_err(|e| rpc_err(e))?;

        // Compute canonical tx hash BEFORE moving tx into the mempool
        let tx_hash = tx.tx_hash();

        let sender = tx.recover_sender()
            .map_err(|e| rpc_err(format!("signature recovery failed: {e}")))?;

        let state = self.state.lock().await;
        let current_nonce = state
            .get_account(&sender)
            .map(|a| a.nonce)
            .unwrap_or(0);
        drop(state);

        let mut mempool = self.mempool.lock().await;
        mempool
            .add(sender, tx, current_nonce)
            .map_err(|e| rpc_err(e))?;

        Ok(format!("0x{}", hex::encode(tx_hash.as_bytes())))
    }

    // ── eth_getBlockByNumber ─────────────────────────────────────────────────

    async fn get_block_by_number(&self, block_param: String, include_txs: bool) -> RpcResult<Value> {
        let chain = self.chain.lock().await;

        let height = match block_param.as_str() {
            "latest" | "pending" => chain.latest_height().map_err(|e| rpc_err(e))?,
            "earliest" => 0,
            hex => {
                u64::from_str_radix(hex.trim_start_matches("0x"), 16)
                    .map_err(|_| rpc_err(format!("invalid block param: {hex}")))?
            }
        };

        let block = chain.get_block(height).map_err(|e| rpc_err(e))?;
        let block_hash = block.hash();

        let txs_value: Value = if include_txs {
            let tx_jsons: Vec<Value> = block
                .transactions
                .iter()
                .enumerate()
                .map(|(idx, tx)| tx_to_json(tx, height, idx as u64))
                .collect();
            json!(tx_jsons)
        } else {
            // Return only tx hashes
            let hashes: Vec<String> = block
                .transactions
                .iter()
                .map(|tx| format!("0x{}", hex::encode(tx.tx_hash().as_bytes())))
                .collect();
            json!(hashes)
        };

        Ok(json!({
            "number":           format!("0x{:x}", block.header.number),
            "hash":             format!("0x{}", hex::encode(block_hash.as_bytes())),
            "parentHash":       format!("0x{}", hex::encode(block.header.parent_hash.as_bytes())),
            "nonce":            format!("0x{:x}", block.header.nonce),
            "stateRoot":        format!("0x{}", hex::encode(block.header.state_root.as_bytes())),
            "transactionsRoot": format!("0x{}", hex::encode(block.header.transactions_root.as_bytes())),
            "receiptsRoot":     format!("0x{}", hex::encode(block.header.receipts_root.as_bytes())),
            "timestamp":        format!("0x{:x}", block.header.timestamp),
            "miner":            format!("0x{:x}", block.header.proposer),
            "transactions":     txs_value,
        }))
    }

    // ── eth_getTransactionByHash ─────────────────────────────────────────────

    async fn get_transaction_by_hash(&self, tx_hash_str: String) -> RpcResult<Value> {
        let hash = parse_hash(&tx_hash_str)?;
        let chain = self.chain.lock().await;
        match chain.get_tx_by_hash(&hash) {
            Ok((tx, height, idx)) => Ok(tx_to_json(&tx, height, idx)),
            Err(_) => Ok(Value::Null), // Ethereum returns null for unknown hashes
        }
    }

    // ── eth_getTransactionCount ──────────────────────────────────────────────

    async fn get_transaction_count(&self, address: String, _block: String) -> RpcResult<String> {
        let addr = parse_address(&address)?;
        let state = self.state.lock().await;
        let acc = state.get_account(&addr).map_err(|e| rpc_err(e))?;
        Ok(format!("0x{:x}", acc.nonce))
    }

    // ── eth_gasPrice ─────────────────────────────────────────────────────────

    /// Returns a suggested gas price.
    ///
    /// Currently a hardcoded 1 Gwei (1_000_000_000 wei).
    /// Phase 6 will replace this with dynamic EIP-1559 base-fee tracking.
    async fn gas_price(&self) -> RpcResult<String> {
        Ok(format!("0x{:x}", 1_000_000_000u64)) // 1 Gwei
    }
}

// ── Server bootstrap ─────────────────────────────────────────────────────────

pub async fn start_server(
    chain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
    state: Arc<Mutex<StateDb>>,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let bind_addr = std::env::var("BYCONN_RPC_ADDR").unwrap_or_else(|_| "0.0.0.0:8545".to_string());
    let server = ServerBuilder::default()
        .build(&bind_addr)
        .await?;

    let addr = server.local_addr()?;
    let module = EthApiServerImpl { chain, mempool, state }.into_rpc();
    let handle = server.start(module);

    info!("JSON-RPC Server started on http://{}", addr);

    // Spawn a task to keep the server alive without blocking the caller
    tokio::spawn(handle.stopped());

    Ok(addr)
}
