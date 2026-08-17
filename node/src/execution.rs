//! Transaction execution engine.
//!
//! Applies transactions to the state database. Without a full EVM, this handles
//! standard balance transfers, nonce increments, and basic gas accounting.

use protocol::transaction::{Address, Transaction};
use protocol::receipt::Receipt;
use crate::state::{StateDb, StateError};
use ethereum_types::U256;
use thiserror::Error;

const BASE_TX_GAS: u64 = 21_000;

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("State error: {0}")]
    State(#[from] StateError),
    #[error("Signature error: {0}")]
    Signature(#[from] secp256k1::Error),
    #[error("Invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },
    #[error("Insufficient balance for gas: need {need}, have {have}")]
    InsufficientGasFunds { need: U256, have: U256 },
    #[error("Insufficient balance for transfer: need {need}, have {have}")]
    InsufficientFunds { need: U256, have: U256 },
    #[error("Gas limit too low: need at least {need}")]
    IntrinsicGas { need: u64 },
}

pub struct ExecutionEngine<'a> {
    state: &'a StateDb,
}

impl<'a> ExecutionEngine<'a> {
    pub fn new(state: &'a StateDb) -> Self {
        Self { state }
    }

    /// Execute a single transaction and apply state changes atomically.
    pub fn execute_transaction(
        &mut self,
        tx: &Transaction,
        miner: Address,
        cumulative_gas_used: u64,
    ) -> Result<Receipt, ExecutionError> {
        // 1. Recover sender
        let sender = tx.recover_sender()?;
        let mut sender_acc = self.state.get_account(&sender)?;

        // 2. Check intrinsic gas limit
        if tx.gas_limit < BASE_TX_GAS {
            return Err(ExecutionError::IntrinsicGas { need: BASE_TX_GAS });
        }

        // 3. Check nonce
        if tx.nonce != sender_acc.nonce {
            return Err(ExecutionError::InvalidNonce {
                expected: sender_acc.nonce,
                got: tx.nonce,
            });
        }

        // 4. Check upfront cost (value + gas_limit * gas_price)
        let max_gas_cost = tx.gas_price.saturating_mul(U256::from(tx.gas_limit));
        let upfront_cost = tx.value.saturating_add(max_gas_cost);

        if sender_acc.balance < upfront_cost {
            return Err(ExecutionError::InsufficientGasFunds {
                need: upfront_cost,
                have: sender_acc.balance,
            });
        }

        // --- State Transitions Begin ---

        // 5. Deduct gas, increment nonce
        sender_acc.nonce += 1;
        sender_acc.balance -= max_gas_cost;

        // Note: For now, if there's not enough to transfer, we revert.
        // In EVM, intrinsic gas is charged, then it reverts during execution.
        if sender_acc.balance < tx.value {
            // Revert state (no-op since we haven't written)
            // But we still charge gas for the attempt in a real EVM.
            // For simplicity, we just fail early here before committing.
            return Err(ExecutionError::InsufficientFunds {
                need: tx.value,
                have: sender_acc.balance,
            });
        }

        // 6. Transfer value
        sender_acc.balance -= tx.value;
        self.state.put_account(&sender, &sender_acc)?;

        if let Some(to) = tx.to {
            let mut recipient_acc = self.state.get_account(&to)?;
            recipient_acc.balance += tx.value;
            self.state.put_account(&to, &recipient_acc)?;
        } else {
            // Contract creation stub: 
            // In a real EVM, we would deploy the bytecode in `tx.data` and create an address.
            // For now, we just burn the value.
        }

        // 7. Gas refunds and miner reward
        // Since we don't have EVM execution, gas used is exactly BASE_TX_GAS.
        let gas_used = BASE_TX_GAS;
        let gas_refund = tx.gas_limit - gas_used;
        let refund_value = U256::from(gas_refund).saturating_mul(tx.gas_price);

        // Refund sender
        let mut sender_acc = self.state.get_account(&sender)?;
        sender_acc.balance += refund_value;
        self.state.put_account(&sender, &sender_acc)?;

        // Reward miner
        let miner_reward = U256::from(gas_used).saturating_mul(tx.gas_price);
        let mut miner_acc = self.state.get_account(&miner)?;
        miner_acc.balance += miner_reward;
        self.state.put_account(&miner, &miner_acc)?;

        // --- State Transitions End ---

        // 8. Generate Receipt
        let receipt = Receipt::success(
            tx.hash_for_signing(None), // placeholder for actual tx hash
            gas_used,
            cumulative_gas_used + gas_used,
            vec![],
            None,
            tx.gas_price,
        );

        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::account::Account;
    use secp256k1::Secp256k1;

    fn temp_db() -> StateDb {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos();
        StateDb::new(&format!("test_exec_state_{}", n)).unwrap()
    }

    fn generate_funded_wallet(db: &StateDb, balance: u64) -> (secp256k1::SecretKey, Address) {
        let secp = Secp256k1::new();
        let (sk, pk) = secp.generate_keypair(&mut rand::thread_rng());
        let uncompressed = pk.serialize_uncompressed();
        let hash = protocol::hash::keccak256(&uncompressed[1..]);
        let addr = Address::from_slice(&hash.as_bytes()[12..32]);

        let mut acc = Account::default();
        acc.balance = U256::from(balance);
        db.put_account(&addr, &acc).unwrap();

        (sk, addr)
    }

    #[test]
    fn test_successful_transfer() {
        let db = temp_db();
        let mut engine = ExecutionEngine::new(&db);

        let (sk1, addr1) = generate_funded_wallet(&db, 1_000_000);
        let (_, addr2) = generate_funded_wallet(&db, 0);
        let miner = Address::from_low_u64_be(99);

        let mut tx = Transaction {
            nonce: 0,
            gas_price: U256::from(10u64),
            gas_limit: 21_000,
            to: Some(addr2),
            value: U256::from(50_000u64),
            data: vec![],
            v: 0, r: U256::zero(), s: U256::zero(),
        };
        tx.sign(&sk1, Some(1)).unwrap();

        let receipt = engine.execute_transaction(&tx, miner, 0).unwrap();
        assert!(receipt.success);
        assert_eq!(receipt.gas_used, 21_000);

        // Check balances
        let final_sender = db.get_account(&addr1).unwrap();
        let final_recipient = db.get_account(&addr2).unwrap();
        let final_miner = db.get_account(&miner).unwrap();

        assert_eq!(final_recipient.balance, U256::from(50_000u64));
        assert_eq!(final_sender.nonce, 1);
        
        let gas_cost = 21_000 * 10;
        assert_eq!(final_sender.balance, U256::from(1_000_000 - 50_000 - gas_cost));
        assert_eq!(final_miner.balance, U256::from(gas_cost));
    }

    #[test]
    fn test_invalid_nonce() {
        let db = temp_db();
        let mut engine = ExecutionEngine::new(&db);
        let (sk1, addr2) = generate_funded_wallet(&db, 1_000_000);
        
        let mut tx = Transaction {
            nonce: 1, // Expected is 0
            gas_price: U256::from(10u64),
            gas_limit: 21_000,
            to: Some(addr2),
            value: U256::zero(),
            data: vec![],
            v: 0, r: U256::zero(), s: U256::zero(),
        };
        tx.sign(&sk1, Some(1)).unwrap();

        let result = engine.execute_transaction(&tx, Address::zero(), 0);
        assert!(matches!(result, Err(ExecutionError::InvalidNonce { .. })));
    }
}
