use sled::Db;
use protocol::block::Block;
use protocol::hash::Hash;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use crate::state::StateDb;

#[derive(Error, Debug)]
pub enum ChainError {
    #[error("Database error: {0}")] Db(#[from] sled::Error),
    #[error("Serialization error: {0}")] Serde(#[from] bincode::Error),
    #[error("Block not found")] BlockNotFound,
    #[error("Invalid Proof of Work")] InvalidPoW,
    #[error("State error: {0}")] State(#[from] crate::state::StateError),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockRecord {
    pub block: Block,
    // Additional metadata could be added here
}

pub struct Blockchain {
    db: Db,
    state: StateDb,
}

impl Blockchain {
    pub fn new(path: &str) -> Result<Self, ChainError> {
        let db = sled::open(path)?;
        let state = StateDb::new(&format!("{}/state", path))?;
        Ok(Self { db, state })
    }

    fn block_key(height: u64) -> Vec<u8> {
        let mut k = b"blk_".to_vec();
        k.extend_from_slice(&height.to_be_bytes());
        k
    }

    pub fn append_block(&self, block: &Block) -> Result<(), ChainError> {
        // Skip PoW check for genesis block (height 0)
        if block.header.number > 0 {
            let hash = block.hash();
            if hash.as_bytes()[0] != 0 {
                return Err(ChainError::InvalidPoW);
            }
        }

        let height = block.header.number;
        let record = BlockRecord { block: block.clone() };
        let bytes = bincode::serialize(&record)?;
        self.db.insert(Self::block_key(height), bytes)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn get_block(&self, height: u64) -> Result<Block, ChainError> {
        match self.db.get(Self::block_key(height))? {
            Some(bytes) => {
                let record: BlockRecord = bincode::deserialize(&bytes)?;
                Ok(record.block)
            }
            None => Err(ChainError::BlockNotFound),
        }
    }

    pub fn latest_height(&self) -> Result<u64, ChainError> {
        if let Some(Ok((key, _))) = self.db.iter().rev().next() {
            let h_bytes = &key[4..]; // skip "blk_"
            let mut arr = [0u8; 8];
            arr.copy_from_slice(h_bytes);
            Ok(u64::from_be_bytes(arr))
        } else {
            Ok(0)
        }
    }

    pub fn head(&self) -> Result<Block, ChainError> {
        let height = self.latest_height()?;
        if height == 0 {
            Err(ChainError::BlockNotFound)
        } else {
            self.get_block(height)
        }
    }
}
