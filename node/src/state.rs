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
const STORAGE_PREFIX: &[u8] = b"stor_";
const CODE_PREFIX: &[u8] = b"code_";

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

    pub fn state_root(&self) -> Result<protocol::hash::Hash, StateError> {
        let mut mpt = protocol::mpt::Mpt::new();
        for item in self.db.scan_prefix(ACCOUNT_PREFIX) {
            let (k, v) = item?;
            let addr_bytes = &k[ACCOUNT_PREFIX.len()..];
            let key_hash = protocol::hash::keccak256(addr_bytes);
            mpt.insert(key_hash.as_bytes(), &v);
        }
        Ok(mpt.root())
    }

    fn storage_key(addr: &Address, key: &protocol::hash::Hash) -> Vec<u8> {
        let mut k = STORAGE_PREFIX.to_vec();
        k.extend_from_slice(addr.as_bytes());
        k.extend_from_slice(key.as_bytes());
        k
    }

    fn storage_prefix(addr: &Address) -> Vec<u8> {
        let mut p = STORAGE_PREFIX.to_vec();
        p.extend_from_slice(addr.as_bytes());
        p
    }

    pub fn set_storage(&self, addr: &Address, key: protocol::hash::Hash, value: protocol::hash::Hash) -> Result<(), StateError> {
        self.db.insert(Self::storage_key(addr, &key), value.as_bytes())?;
        
        let root = self.compute_storage_root(addr)?;
        let mut acc = self.get_account(addr)?;
        acc.storage_root = root;
        self.put_account(addr, &acc)?;
        Ok(())
    }

    pub fn get_storage(&self, addr: &Address, key: protocol::hash::Hash) -> Result<protocol::hash::Hash, StateError> {
        match self.db.get(Self::storage_key(addr, &key))? {
            Some(b) => Ok(protocol::hash::Hash::from_slice(&b)),
            None => Ok(protocol::hash::Hash::zero()),
        }
    }

    fn compute_storage_root(&self, addr: &Address) -> Result<protocol::hash::Hash, StateError> {
        let mut mpt = protocol::mpt::Mpt::new();
        let prefix = Self::storage_prefix(addr);
        for item in self.db.scan_prefix(&prefix) {
            let (k, v) = item?;
            let key_hash_bytes = &k[prefix.len()..];
            let trie_key = protocol::hash::keccak256(key_hash_bytes);
            mpt.insert(trie_key.as_bytes(), &v);
        }
        Ok(mpt.root())
    }

    fn code_key(hash: &protocol::hash::Hash) -> Vec<u8> {
        let mut k = CODE_PREFIX.to_vec();
        k.extend_from_slice(hash.as_bytes());
        k
    }

    pub fn set_code(&self, addr: &Address, code: &[u8]) -> Result<(), StateError> {
        let hash = protocol::hash::keccak256(code);
        self.db.insert(Self::code_key(&hash), code)?;
        let mut acc = self.get_account(addr)?;
        acc.code_hash = hash;
        self.put_account(addr, &acc)?;
        Ok(())
    }

    pub fn get_code(&self, hash: &protocol::hash::Hash) -> Result<Vec<u8>, StateError> {
        if hash == &protocol::account::EMPTY_CODE_HASH {
            return Ok(Vec::new());
        }
        match self.db.get(Self::code_key(hash))? {
            Some(b) => Ok(b.to_vec()),
            None => Ok(Vec::new()),
        }
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
