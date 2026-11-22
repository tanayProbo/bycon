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

/// Minimum target time to mine a single block: 3 hours (10,800 seconds).
pub const TARGET_BLOCK_TIME_SECS: u64 = 10_800;

/// Block reward halves every 2,100 blocks (~262.5 days at 3h/block), capping total supply at 12,600 BYC.
pub const HALVING_INTERVAL_BLOCKS: u64 = 2_100;

/// Initial block reward: 3 BYC (3 * 10^18 wei).
pub const INITIAL_BLOCK_REWARD_WEI: u128 = 3_000_000_000_000_000_000;

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let stream = rlp::encode(self);
        keccak256(&stream)
    }

    /// Dynamic difficulty escalation:
    /// Starts at 34 leading zero bits (~17.1 billion 8-round Keccak hashes, ~3 hours on CPU)
    /// and increases by +1 leading zero bit (2x harder) every 100 blocks.
    pub fn required_zero_bits(block_number: u64) -> u32 {
        let bonus = (block_number / 100).min(60) as u32;
        34 + bonus
    }

    /// Deflationary block reward with Bitcoin-style halving every 2,100 blocks.
    /// Starts at 3 BYC, then 1.5 BYC, 0.75 BYC, etc. Max total supply = 12,600 BYC.
    pub fn block_reward_wei(block_number: u64) -> u128 {
        let halvings = block_number / HALVING_INTERVAL_BLOCKS;
        if halvings >= 64 {
            0
        } else {
            INITIAL_BLOCK_REWARD_WEI >> halvings
        }
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

    /// Checks whether a 256-bit hash has at least `bits` leading zero bits.
    pub fn has_leading_zero_bits(hash: &Hash, bits: u32) -> bool {
        let bytes = hash.as_bytes();
        let full_bytes = (bits / 8) as usize;
        let rem_bits = bits % 8;

        for b in bytes.iter().take(full_bytes.min(32)) {
            if *b != 0 {
                return false;
            }
        }
        if full_bytes < 32 && rem_bits > 0 {
            let mask = 0xFFu8 << (8 - rem_bits);
            if (bytes[full_bytes] & mask) != 0 {
                return false;
            }
        }
        true
    }

    /// Returns true if the layered PoW hash meets the escalating difficulty target for this block height.
    pub fn meets_pow_target(&self) -> bool {
        let pow = self.pow_hash();
        Self::has_leading_zero_bits(&pow, Self::required_zero_bits(self.number))
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
