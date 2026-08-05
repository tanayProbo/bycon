use crate::hash::{keccak256, Hash};
use crate::transaction::{Address, Transaction};
use rlp_derive::{RlpDecodable, RlpEncodable};

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct BlockHeader {
    pub parent_hash: Hash,
    pub state_root: Hash,
    pub transactions_root: Hash,
    pub receipts_root: Hash,
    pub timestamp: u64,
    pub proposer: Address,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let stream = rlp::encode(self);
        keccak256(&stream)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }
}
