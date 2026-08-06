use crate::hash::Hash;
use crate::transaction::Address;
use ethereum_types::U256;
use rlp_derive::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

/// A single log event emitted by a smart contract during transaction execution.
/// Equivalent to an Ethereum EVM `LOG` opcode output.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Log {
    /// The contract address that emitted this log.
    pub address: Address,
    /// Up to 4 indexed topics (event signature + indexed parameters).
    pub topics: Vec<Hash>,
    /// ABI-encoded non-indexed event data.
    pub data: Vec<u8>,
}

/// The result of executing a single transaction.
///
/// Receipts are stored in each block and their root hash is committed to
/// `BlockHeader.receipts_root`, allowing light clients to verify outcomes.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Receipt {
    /// keccak256 hash of the transaction that produced this receipt.
    pub tx_hash: Hash,
    /// `true` if the transaction executed successfully, `false` if it reverted.
    pub success: bool,
    /// Total gas used by this transaction.
    pub gas_used: u64,
    /// Cumulative gas used in the block up to and including this transaction.
    pub cumulative_gas_used: u64,
    /// The contract address created by this transaction, if it was a contract deployment.
    pub contract_address: Option<Address>,
    /// Log events emitted during execution.
    pub logs: Vec<Log>,
    /// The effective gas price paid (base_fee + priority_fee).
    pub effective_gas_price: U256,
}

impl Receipt {
    /// Create a successful receipt.
    pub fn success(
        tx_hash: Hash,
        gas_used: u64,
        cumulative_gas_used: u64,
        logs: Vec<Log>,
        contract_address: Option<Address>,
        effective_gas_price: U256,
    ) -> Self {
        Self {
            tx_hash,
            success: true,
            gas_used,
            cumulative_gas_used,
            contract_address,
            logs,
            effective_gas_price,
        }
    }

    /// Create a failed (reverted) receipt.
    pub fn failure(
        tx_hash: Hash,
        gas_used: u64,
        cumulative_gas_used: u64,
        effective_gas_price: U256,
    ) -> Self {
        Self {
            tx_hash,
            success: false,
            gas_used,
            cumulative_gas_used,
            contract_address: None,
            logs: vec![],
            effective_gas_price,
        }
    }

    /// Encode the receipt to RLP bytes.
    pub fn encode(&self) -> Vec<u8> {
        rlp::encode(self).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Hash;
    use ethereum_types::U256;

    #[test]
    fn test_success_receipt_fields() {
        let tx_hash = Hash::zero();
        let receipt = Receipt::success(tx_hash, 21_000, 21_000, vec![], None, U256::from(1_000u64));
        assert!(receipt.success);
        assert_eq!(receipt.gas_used, 21_000);
        assert!(receipt.logs.is_empty());
        assert!(receipt.contract_address.is_none());
    }

    #[test]
    fn test_failure_receipt_has_no_logs() {
        let receipt = Receipt::failure(Hash::zero(), 21_000, 21_000, U256::from(1u64));
        assert!(!receipt.success);
        assert!(receipt.logs.is_empty());
        assert!(receipt.contract_address.is_none());
    }

    #[test]
    fn test_receipt_rlp_encode() {
        let receipt = Receipt::success(Hash::zero(), 21_000, 21_000, vec![], None, U256::from(1u64));
        let encoded = receipt.encode();
        assert!(!encoded.is_empty());
    }
}
