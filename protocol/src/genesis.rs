use crate::block::{Block, BlockHeader};
use crate::hash::Hash;
use crate::transaction::Address;

pub fn get_genesis_block() -> Block {
    Block {
        header: BlockHeader {
            parent_hash: Hash::zero(),
            number: 0,
            state_root: Hash::zero(),
            transactions_root: Hash::zero(),
            receipts_root: Hash::zero(),
            timestamp: 0,
            proposer: Address::zero(),
        },
        transactions: vec![],
    }
}
