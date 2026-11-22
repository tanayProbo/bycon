//! Blockchain storage backed by sled.
//!
//! ## Key layout
//! | Prefix     | Key                                | Value                          |
//! |------------|------------------------------------|--------------------------------|
//! | `blk_`     | `blk_` + height as big-endian u64  | `bincode(BlockRecord)`         |
//! | `txidx_`   | `txidx_` + tx_hash (32 bytes)      | `bincode((block_height, idx))` |

use sled::Db;
use protocol::block::Block;
use protocol::hash::Hash;
use protocol::transaction::Transaction;
use thiserror::Error;
use serde::{Serialize, Deserialize};

#[derive(Error, Debug)]
pub enum ChainError {
    #[error("Database error: {0}")]
    Db(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] bincode::Error),
    #[error("Block not found")]
    BlockNotFound,
    #[error("Invalid Proof of Work")]
    InvalidPoW,
}

/// Wrapper around a `Block` stored in the chain database.
/// Additional metadata (e.g. total-difficulty) can be added here later.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockRecord {
    pub block: Block,
}

pub struct Blockchain {
    db: Db,
}

impl Blockchain {
    pub fn new(path: &str) -> Result<Self, ChainError> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    // ── Key builders ────────────────────────────────────────────────────────

    fn block_key(height: u64) -> Vec<u8> {
        let mut k = b"blk_".to_vec();
        k.extend_from_slice(&height.to_be_bytes());
        k
    }

    fn tx_index_key(hash: &Hash) -> Vec<u8> {
        let mut k = b"txidx_".to_vec();
        k.extend_from_slice(hash.as_bytes());
        k
    }

    // ── Write ────────────────────────────────────────────────────────────────

    /// Append a validated block to the chain.
    ///
    /// Enforces the PoW target for all non-genesis blocks and builds a
    /// transaction hash → (block_height, tx_index) secondary index so that
    /// transactions can be retrieved in O(1) by their canonical hash.
    pub fn append_block(&self, block: &Block) -> Result<(), ChainError> {
        // PoW check (skip for genesis block at height 0)
        if block.header.number > 0 {
            let hash = block.hash();
            let b = hash.as_bytes();
            if b[0] != 0 || b[1] != 0 || (b[2] & 0xF0) != 0 {
                return Err(ChainError::InvalidPoW);
            }
        }

        let height = block.header.number;
        let record = BlockRecord { block: block.clone() };
        let bytes = bincode::serialize(&record)?;
        self.db.insert(Self::block_key(height), bytes)?;

        // Build tx-hash secondary index
        for (idx, tx) in block.transactions.iter().enumerate() {
            let tx_hash = tx.tx_hash();
            let index_val = bincode::serialize(&(height, idx as u64))?;
            self.db.insert(Self::tx_index_key(&tx_hash), index_val)?;
        }

        self.db.flush()?;
        Ok(())
    }

    // ── Read ─────────────────────────────────────────────────────────────────

    /// Retrieve a block by its height.
    pub fn get_block(&self, height: u64) -> Result<Block, ChainError> {
        match self.db.get(Self::block_key(height))? {
            Some(bytes) => {
                let record: BlockRecord = bincode::deserialize(&bytes)?;
                Ok(record.block)
            }
            None => Err(ChainError::BlockNotFound),
        }
    }

    /// Retrieve a transaction (and its block context) by its canonical hash.
    ///
    /// Returns `(transaction, block_height, tx_index_within_block)`.
    pub fn get_tx_by_hash(&self, hash: &Hash) -> Result<(Transaction, u64, u64), ChainError> {
        match self.db.get(Self::tx_index_key(hash))? {
            Some(bytes) => {
                let (height, idx): (u64, u64) = bincode::deserialize(&bytes)?;
                let block = self.get_block(height)?;
                let tx = block
                    .transactions
                    .into_iter()
                    .nth(idx as usize)
                    .ok_or(ChainError::BlockNotFound)?;
                Ok((tx, height, idx))
            }
            None => Err(ChainError::BlockNotFound),
        }
    }

    /// Returns `true` if no blocks have been appended yet (empty chain).
    pub fn is_empty(&self) -> bool {
        self.db.scan_prefix(b"blk_").next().is_none()
    }

    /// Returns the height of the highest block in the chain, or `0` if the
    /// chain is empty (including when only the genesis block exists).
    ///
    /// Scans only the `blk_` prefix in reverse so that `txidx_` keys
    /// (which sort lexicographically after `blk_`) do not interfere.
    pub fn latest_height(&self) -> Result<u64, ChainError> {
        if let Some(item) = self.db.scan_prefix(b"blk_").rev().next() {
            let (key, _) = item?;
            let h_bytes = &key[4..]; // skip "blk_" prefix (4 bytes)
            let mut arr = [0u8; 8];
            arr.copy_from_slice(h_bytes);
            Ok(u64::from_be_bytes(arr))
        } else {
            Ok(0)
        }
    }

    /// Returns the highest block in the chain.
    ///
    /// Unlike the previous implementation, this correctly returns the genesis
    /// block (height 0) when it is the only block, instead of `BlockNotFound`.
    pub fn head(&self) -> Result<Block, ChainError> {
        if self.is_empty() {
            return Err(ChainError::BlockNotFound);
        }
        let height = self.latest_height()?;
        self.get_block(height)
    }
}
