use std::sync::Arc;
use tokio::sync::Mutex;
use jsonrpsee::server::ServerBuilder;
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use tracing::info;
use std::net::SocketAddr;

use protocol::transaction::{Address, Transaction};
use crate::chain::Blockchain;
use crate::mempool::Mempool;
use crate::state::StateDb;

#[rpc(server)]
pub trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn block_number(&self) -> RpcResult<String>;

    #[method(name = "eth_getBalance")]
    async fn get_balance(&self, address: String, block: String) -> RpcResult<String>;

    #[method(name = "eth_sendRawTransaction")]
    async fn send_raw_transaction(&self, tx_bytes: String) -> RpcResult<String>;
}

pub struct EthApiServerImpl {
    chain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
    state: Arc<Mutex<StateDb>>,
}

#[jsonrpsee::core::async_trait]
impl EthApiServer for EthApiServerImpl {
    async fn block_number(&self) -> RpcResult<String> {
        let chain = self.chain.lock().await;
        let height = chain.latest_height().unwrap_or(0);
        Ok(format!("0x{:x}", height))
    }

    async fn get_balance(&self, address: String, _block: String) -> RpcResult<String> {
        let state = self.state.lock().await;
        // Parse hex address
        let addr_bytes = hex::decode(address.trim_start_matches("0x"))
            .map_err(|e| jsonrpsee::types::error::ErrorObject::owned(-32000, e.to_string(), None::<()>))?;
        let addr = Address::from_slice(&addr_bytes);

        let acc = state.get_account(&addr)
            .map_err(|e| jsonrpsee::types::error::ErrorObject::owned(-32000, e.to_string(), None::<()>))?;
        
        // Return hex string of U256 balance
        Ok(format!("0x{:x}", acc.balance))
    }

    async fn send_raw_transaction(&self, tx_bytes: String) -> RpcResult<String> {
        let bytes = hex::decode(tx_bytes.trim_start_matches("0x"))
            .map_err(|e| jsonrpsee::types::error::ErrorObject::owned(-32000, e.to_string(), None::<()>))?;
        
        let tx: Transaction = rlp::decode(&bytes)
            .map_err(|e| jsonrpsee::types::error::ErrorObject::owned(-32000, e.to_string(), None::<()>))?;

        let tx_hash = tx.hash_for_signing(None);

        let sender = tx.recover_sender()
            .map_err(|e| jsonrpsee::types::error::ErrorObject::owned(-32000, e.to_string(), None::<()>))?;
            
        let state = self.state.lock().await;
        let current_nonce = state.get_account(&sender).map(|a| a.nonce).unwrap_or(0);
        drop(state);

        let mut mempool = self.mempool.lock().await;
        mempool.add(sender, tx, current_nonce)
            .map_err(|e| jsonrpsee::types::error::ErrorObject::owned(-32000, e.to_string(), None::<()>))?;

        Ok(format!("0x{}", hex::encode(tx_hash.as_bytes())))
    }
}

pub async fn start_server(
    chain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
    state: Arc<Mutex<StateDb>>,
) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let server = ServerBuilder::default()
        .build("127.0.0.1:8545")
        .await?;

    let addr = server.local_addr()?;
    let module = EthApiServerImpl { chain, mempool, state }.into_rpc();
    
    let handle = server.start(module);
    
    info!("JSON-RPC Server started on http://{}", addr);
    
    // Spawn a task so it doesn't block
    tokio::spawn(handle.stopped());

    Ok(addr)
}
