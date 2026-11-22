use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc::UnboundedSender, Mutex};
use tracing::info;

use protocol::block::{Block, BlockHeader};
use protocol::hash::Hash;
use protocol::transaction::Address;
use protocol::trie::{compute_transactions_root, compute_receipts_root};
use ethereum_types::U256;

/// Block mining reward: 3 BYC (3 * 10^18 wei).
const BLOCK_REWARD_WEI: u128 = 3_000_000_000_000_000_000;

use crate::chain::Blockchain;
use crate::mempool::Mempool;
use crate::state::StateDb;
use crate::execution::ExecutionEngine;

pub struct Miner {
    chain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
    state_db: Arc<Mutex<StateDb>>,
    coinbase: Address,
    block_tx: Option<UnboundedSender<Block>>,
}

impl Miner {
    pub fn new(
        chain: Arc<Mutex<Blockchain>>,
        mempool: Arc<Mutex<Mempool>>,
        state_db: Arc<Mutex<StateDb>>,
        coinbase: Address,
        block_tx: Option<UnboundedSender<Block>>,
    ) -> Self {
        Self { chain, mempool, state_db, coinbase, block_tx }
    }

    pub async fn start(&self) {
        info!("Miner task started with PoW enabled");
        loop {
            // We sleep briefly so we don't lock up the CPU instantly if mempool is empty
            tokio::time::sleep(Duration::from_millis(500)).await;
            self.mine_block().await;
        }
    }

    async fn mine_block(&self) {
        let mut chain = self.chain.lock().await;
        let mut mempool = self.mempool.lock().await;
        let state_db = self.state_db.lock().await;
        
        let latest_height = chain.latest_height().unwrap_or(0);
        let parent_hash = chain.head().map(|b| b.hash()).unwrap_or(Hash::zero());

        let mut txs = Vec::new();
        let mut receipts = Vec::new();
        let mut engine = ExecutionEngine::new(&state_db);

        let mut cumulative_gas = 0;
        let ordered_txs = mempool.ordered_txs().into_iter().cloned().collect::<Vec<_>>();
        
        for tx in ordered_txs {
            if let Ok(receipt) = engine.execute_transaction(&tx, self.coinbase, cumulative_gas) {
                if receipt.success {
                    cumulative_gas += receipt.gas_used;
                    txs.push(tx.clone());
                    receipts.push(receipt);
                    mempool.remove(&tx.recover_sender().unwrap_or_default(), tx.nonce);
                }
            }
        }

        // Credit PoW block reward (3 BYC) to the miner's coinbase account
        if let Ok(mut miner_acc) = state_db.get_account(&self.coinbase) {
            miner_acc.balance = miner_acc.balance.saturating_add(U256::from(BLOCK_REWARD_WEI));
            let _ = state_db.put_account(&self.coinbase, &miner_acc);
        }

        // Rigor: Calculate actual Merkle Roots
        let transactions_root = compute_transactions_root(&txs);
        let receipts_root = compute_receipts_root(&receipts);

        let state_root = state_db.state_root().unwrap_or(Hash::zero());

        let mut header = BlockHeader {
            parent_hash,
            number: latest_height + 1,
            nonce: 0,
            state_root,
            transactions_root,
            receipts_root,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            proposer: self.coinbase,
        };

        // Rigor: Layered Ethash-inspired Proof of Work Loop
        info!("Mining block {} with {} txs...", header.number, txs.len());
        while !header.meets_pow_target() {
            header.nonce += 1;
        }

        let new_block = Block {
            header,
            transactions: txs,
        };

        if let Ok(()) = chain.append_block(&new_block) {
            info!("Mined block {} (nonce: {}) -> hash: {:?}", new_block.header.number, new_block.header.nonce, new_block.hash().as_bytes());
            if let Some(tx) = &self.block_tx {
                let _ = tx.send(new_block);
            }
        }
    }
}
