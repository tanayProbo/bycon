use ethereum_types::H256;
use tiny_keccak::{Hasher, Keccak};

pub type Hash = H256;

pub fn keccak256(data: &[u8]) -> Hash {
    let mut keccak = Keccak::v256();
    let mut output = [0u8; 32];
    keccak.update(data);
    keccak.finalize(&mut output);
    Hash::from(output)
}
