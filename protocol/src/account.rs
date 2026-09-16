use crate::hash::{keccak256, Hash};
use ethereum_types::U256;
use rlp_derive::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

/// The keccak256 hash of empty bytes — the code_hash of a plain (non-contract) account.
pub const EMPTY_CODE_HASH: Hash = Hash::from([
    0xc5, 0xd2, 0x46, 0x01, 0x86, 0xf7, 0x23, 0x3c,
    0x92, 0x7e, 0x7d, 0xb2, 0xdc, 0xc7, 0x03, 0xc0,
    0xe5, 0x00, 0xb6, 0x53, 0xca, 0x82, 0x27, 0x3b,
    0x7b, 0xfa, 0xd8, 0x04, 0x5d, 0x85, 0xa4, 0x70,
]);

/// The keccak256 hash of the RLP encoding of an empty list — the storage_root of an account
/// with no contract storage slots set.
pub const EMPTY_STORAGE_ROOT: Hash = Hash::from([
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6,
    0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
    0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0,
    0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
]);

/// An Ethereum-style account in the world state.
///
/// - Externally Owned Accounts (EOAs) have `code_hash == EMPTY_CODE_HASH`
///   and `storage_root == EMPTY_STORAGE_ROOT`.
/// - Contract accounts have real values for both.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Account {
    /// Number of transactions sent from this account (used to prevent replay attacks).
    pub nonce: u64,
    /// BYC balance in the smallest unit (wei equivalent).
    pub balance: U256,
    /// keccak256 of the account's contract bytecode. `EMPTY_CODE_HASH` for EOAs.
    pub code_hash: Hash,
    /// Merkle root of the contract's storage trie. `EMPTY_STORAGE_ROOT` for EOAs.
    pub storage_root: Hash,
}

impl Default for Account {
    fn default() -> Self {
        Self {
            nonce: 0,
            balance: U256::zero(),
            code_hash: EMPTY_CODE_HASH,
            storage_root: EMPTY_STORAGE_ROOT,
        }
    }
}

impl Account {
    /// Create a new externally owned account (EOA) with a given balance and zero nonce.
    pub fn new_eoa(balance: U256) -> Self {
        Self {
            balance,
            ..Default::default()
        }
    }

    /// Returns true if this account is an Externally Owned Account (not a smart contract).
    pub fn is_eoa(&self) -> bool {
        self.code_hash == EMPTY_CODE_HASH
    }

    /// Encode the account to RLP bytes for storage.
    pub fn encode(&self) -> Vec<u8> {
        rlp::encode(self).to_vec()
    }

    /// Decode an account from RLP bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, rlp::DecoderError> {
        rlp::decode(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_code_hash_value() {
        // keccak256("") = 0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470
        let computed = keccak256(&[]);
        assert_eq!(computed, EMPTY_CODE_HASH, "EMPTY_CODE_HASH constant is incorrect");
    }

    #[test]
    fn test_eoa_default() {
        let acc = Account::default();
        assert!(acc.is_eoa());
        assert_eq!(acc.nonce, 0);
        assert_eq!(acc.balance, U256::zero());
    }

    #[test]
    fn test_rlp_roundtrip() {
        let acc = Account::new_eoa(U256::from(1_000_000u64));
        let encoded = acc.encode();
        let decoded = Account::decode(&encoded).unwrap();
        assert_eq!(acc, decoded);
    }
}
