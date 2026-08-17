//! World-state database backed by sled.
use sled::Db;
use protocol::account::Account;
use protocol::transaction::Address;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateError {
    #[error("Database error: {0}")] Db(#[from] sled::Error),
    #[error("RLP decode error: {0}")] Decode(#[from] rlp::DecoderError),
}

const ACCOUNT_PREFIX: &[u8] = b"acct_";

pub struct StateDb { db: Db }

impl StateDb {
    pub fn new(path: &str) -> Result<Self, StateError> {
        Ok(Self { db: sled::open(path)? })
    }

    fn account_key(addr: &Address) -> Vec<u8> {
        let mut k = ACCOUNT_PREFIX.to_vec();
        k.extend_from_slice(addr.as_bytes());
        k
    }

    pub fn get_account(&self, addr: &Address) -> Result<Account, StateError> {
        match self.db.get(Self::account_key(addr))? {
            Some(b) => Ok(Account::decode(&b)?),
            None => Ok(Account::default()),
        }
    }

    pub fn put_account(&self, addr: &Address, account: &Account) -> Result<(), StateError> {
        self.db.insert(Self::account_key(addr), account.encode())?;
        Ok(())
    }

    pub fn delete_account(&self, addr: &Address) -> Result<(), StateError> {
        self.db.remove(Self::account_key(addr))?;
        Ok(())
    }

    pub fn increment_nonce(&self, addr: &Address) -> Result<u64, StateError> {
        let mut acc = self.get_account(addr)?;
        acc.nonce += 1;
        self.put_account(addr, &acc)?;
        Ok(acc.nonce)
    }

    pub fn flush(&self) -> Result<(), StateError> { self.db.flush()?; Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_types::U256;

    fn tmp() -> StateDb {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos();
        StateDb::new(&format!("test_state_{}", n)).unwrap()
    }

    #[test]
    fn missing_account_is_default() {
        let db = tmp();
        let acc = db.get_account(&Address::zero()).unwrap();
        assert_eq!(acc.nonce, 0);
        assert_eq!(acc.balance, U256::zero());
    }

    #[test]
    fn put_and_get_roundtrip() {
        let db = tmp();
        let addr = Address::from_low_u64_be(42);
        db.put_account(&addr, &Account::new_eoa(U256::from(999u64))).unwrap();
        assert_eq!(db.get_account(&addr).unwrap().balance, U256::from(999u64));
    }

    #[test]
    fn nonce_increments() {
        let db = tmp();
        let addr = Address::from_low_u64_be(1);
        assert_eq!(db.increment_nonce(&addr).unwrap(), 1);
        assert_eq!(db.increment_nonce(&addr).unwrap(), 2);
    }
}
