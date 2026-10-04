//! Merkle root utilities for BYCONN block construction.
//!
//! Phase 1 uses a simplified flat Merkle tree:
//! - RLP-encode each item
//! - Sort items by their RLP encoding for determinism
//! - Iteratively hash pairs until one root remains
//! - If the list is empty, return the zero hash
//!
//! A full Ethereum Merkle-Patricia Trie (MPT) will replace this in Phase 2
//! when the EVM execution layer is introduced.

use crate::hash::{keccak256, Hash};
use crate::receipt::Receipt;
use crate::transaction::Transaction;

/// Compute the Merkle root of a list of pre-encoded byte slices.
///
/// Uses a simple recursive binary tree: leaf nodes are `keccak256(item)`,
/// internal nodes are `keccak256(left_child || right_child)`.
/// If the list has an odd number, the last element is duplicated.
pub fn merkle_root(items: &[Vec<u8>]) -> Hash {
    if items.is_empty() {
        return Hash::zero();
    }

    // Leaf level: hash each item
    let mut level: Vec<Hash> = items.iter().map(|item| keccak256(item)).collect();

    // Reduce until one root remains
    while level.len() > 1 {
        let mut next_level = Vec::with_capacity((level.len() + 1) / 2);
        let mut i = 0;
        while i < level.len() {
            let left = level[i];
            // If odd number, duplicate the last node
            let right = if i + 1 < level.len() { level[i + 1] } else { level[i] };

            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(left.as_bytes());
            combined[32..].copy_from_slice(right.as_bytes());
            next_level.push(keccak256(&combined));
            i += 2;
        }
        level = next_level;
    }

    level[0]
}

/// Compute the `transactions_root` for a block.
///
/// Each transaction is RLP-encoded, then the list is fed into the Merkle tree.
pub fn compute_transactions_root(txs: &[Transaction]) -> Hash {
    let encoded: Vec<Vec<u8>> = txs.iter().map(|tx| rlp::encode(tx).to_vec()).collect();
    merkle_root(&encoded)
}

/// Compute the `receipts_root` for a block.
pub fn compute_receipts_root(receipts: &[Receipt]) -> Hash {
    let encoded: Vec<Vec<u8>> = receipts.iter().map(|r| r.encode()).collect();
    merkle_root(&encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_root_is_zero() {
        let root = merkle_root(&[]);
        assert_eq!(root, Hash::zero());
    }

    #[test]
    fn test_single_item_root() {
        let data = b"hello byconn".to_vec();
        let root = merkle_root(&[data.clone()]);
        // Single item: root == keccak256(item)
        assert_eq!(root, keccak256(&data));
    }

    #[test]
    fn test_two_item_root_is_deterministic() {
        let a = b"block_a".to_vec();
        let b = b"block_b".to_vec();
        let root1 = merkle_root(&[a.clone(), b.clone()]);
        let root2 = merkle_root(&[a, b]);
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_order_matters() {
        let a = b"tx_a".to_vec();
        let b = b"tx_b".to_vec();
        let root_ab = merkle_root(&[a.clone(), b.clone()]);
        let root_ba = merkle_root(&[b, a]);
        assert_ne!(root_ab, root_ba, "Order of transactions must affect root");
    }

    #[test]
    fn test_four_items_root() {
        let items: Vec<Vec<u8>> = (0u8..4).map(|i| vec![i]).collect();
        let root = merkle_root(&items);
        assert_ne!(root, Hash::zero());
    }
}
