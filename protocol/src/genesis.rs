use crate::block::{Block, BlockHeader};
use crate::hash::Hash;
use crate::transaction::Address;

pub fn get_genesis_block() -> Block {
    Block {
        header: BlockHeader {
            parent_hash: Hash::zero(),
            state_root: Hash::zero(),
            transactions_root: Hash::zero(),
            receipts_root: Hash::zero(),
            timestamp: 1672531200,
            proposer: Address::zero(),
        },
        transactions: vec![],
    }
}
