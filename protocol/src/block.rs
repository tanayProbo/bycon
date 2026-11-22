use crate::hash::{keccak256, Hash};
use crate::transaction::{Address, Transaction};
use rlp_derive::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct BlockHeader {
    pub parent_hash: Hash,
    pub number: u64,
    pub nonce: u64,
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

    /// Layered Ethash-inspired Proof-of-Work hash:
    /// 1. Compute header seed = keccak256(RLP(header))
    /// 2. Run 8 sequential mixing rounds binding (mix || parent_hash || nonce || round)
    /// 3. Finalize pow_hash = keccak256(seed || mix)
    pub fn pow_hash(&self) -> Hash {
        let seed = self.hash();
        let mut mix = seed;
        let nonce_bytes = self.nonce.to_be_bytes();

        for round in 0u8..8 {
            let mut buf = [0u8; 73];
            buf[0..32].copy_from_slice(mix.as_bytes());
            buf[32..64].copy_from_slice(self.parent_hash.as_bytes());
            buf[64..72].copy_from_slice(&nonce_bytes);
            buf[72] = round;
            mix = keccak256(&buf);
        }

        let mut final_buf = [0u8; 64];
        final_buf[0..32].copy_from_slice(seed.as_bytes());
        final_buf[32..64].copy_from_slice(mix.as_bytes());
        keccak256(&final_buf)
    }

    /// Returns true if the layered PoW hash meets the 20-bit leading-zero target (0x00000...).
    pub fn meets_pow_target(&self) -> bool {
        let pow = self.pow_hash();
        let b = pow.as_bytes();
        b[0] == 0 && b[1] == 0 && (b[2] & 0xF0) == 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }
}
