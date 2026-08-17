//! In-memory transaction mempool.
use std::collections::HashMap;
use protocol::transaction::{Address, Transaction};
use ethereum_types::U256;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TxKey { pub sender: Address, pub nonce: u64 }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MempoolError {
    PoolFull,
    Underpriced,
    NonceTooLow { expected: u64, got: u64 },
}
impl std::fmt::Display for MempoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MempoolError::PoolFull => write!(f, "mempool is full"),
            MempoolError::Underpriced => write!(f, "transaction underpriced"),
            MempoolError::NonceTooLow { expected, got } =>
                write!(f, "nonce too low: expected >= {}, got {}", expected, got),
        }
    }
}
impl std::error::Error for MempoolError {}

pub struct Mempool {
    pub max_size: usize,
    pending: HashMap<TxKey, Transaction>,
}

impl Mempool {
    pub fn new(max_size: usize) -> Self { Self { max_size, pending: HashMap::new() } }

    pub fn add(&mut self, sender: Address, tx: Transaction, current_nonce: u64) -> Result<bool, MempoolError> {
        if tx.nonce < current_nonce {
            return Err(MempoolError::NonceTooLow { expected: current_nonce, got: tx.nonce });
        }
        let key = TxKey { sender, nonce: tx.nonce };
        if let Some(existing) = self.pending.get(&key) {
            if tx.gas_price <= existing.gas_price { return Err(MempoolError::Underpriced); }
        } else if self.pending.len() >= self.max_size {
            return Err(MempoolError::PoolFull);
        }
        self.pending.insert(key, tx);
        Ok(true)
    }

    pub fn remove(&mut self, sender: &Address, nonce: u64) -> Option<Transaction> {
        self.pending.remove(&TxKey { sender: *sender, nonce })
    }

    pub fn len(&self) -> usize { self.pending.len() }
    pub fn is_empty(&self) -> bool { self.pending.is_empty() }

    pub fn ordered_txs(&self) -> Vec<&Transaction> {
        let mut txs: Vec<&Transaction> = self.pending.values().collect();
        txs.sort_by(|a, b| b.gas_price.cmp(&a.gas_price).then(a.nonce.cmp(&b.nonce)));
        txs
    }

    pub fn prune_sender(&mut self, sender: &Address, confirmed_nonce: u64) {
        self.pending.retain(|k, _| !(k.sender == *sender && k.nonce <= confirmed_nonce));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tx(nonce: u64, gp: u64) -> Transaction {
        Transaction { nonce, gas_price: U256::from(gp), gas_limit: 21_000,
            to: Some(Address::zero()), value: U256::zero(), data: vec![],
            v: 0, r: U256::zero(), s: U256::zero() }
    }
    #[test]
    fn add_and_len() {
        let mut p = Mempool::new(10);
        p.add(Address::from_low_u64_be(1), tx(0, 1000), 0).unwrap();
        assert_eq!(p.len(), 1);
    }
    #[test]
    fn underpriced_rejected() {
        let mut p = Mempool::new(10);
        let s = Address::from_low_u64_be(1);
        p.add(s, tx(0, 2000), 0).unwrap();
        assert_eq!(p.add(s, tx(0, 1000), 0), Err(MempoolError::Underpriced));
    }
    #[test]
    fn pool_full() {
        let mut p = Mempool::new(1);
        p.add(Address::from_low_u64_be(1), tx(0, 100), 0).unwrap();
        assert_eq!(p.add(Address::from_low_u64_be(2), tx(0, 200), 0), Err(MempoolError::PoolFull));
    }
    #[test]
    fn nonce_too_low() {
        let mut p = Mempool::new(10);
        assert!(matches!(p.add(Address::from_low_u64_be(1), tx(0, 100), 5), Err(MempoolError::NonceTooLow { .. })));
    }
    #[test]
    fn prune_sender() {
        let mut p = Mempool::new(10);
        let s = Address::from_low_u64_be(1);
        for i in 0..3 { p.add(s, tx(i, 100), 0).unwrap(); }
        p.prune_sender(&s, 1);
        assert_eq!(p.len(), 1);
    }
    #[test]
    fn ordered_by_gas_price() {
        let mut p = Mempool::new(10);
        p.add(Address::from_low_u64_be(1), tx(0, 100), 0).unwrap();
        p.add(Address::from_low_u64_be(2), tx(0, 500), 0).unwrap();
        assert_eq!(p.ordered_txs()[0].gas_price, U256::from(500u64));
    }
}
