use crate::hash::{keccak256, Hash};
use ethereum_types::{H160, U256};
use rlp_derive::{RlpEncodable, RlpDecodable};
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, Secp256k1, SecretKey,
};
use serde::{Deserialize, Serialize};

pub type Address = H160;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub v: u64,
    pub r: U256,
    pub s: U256,
}

#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable, Serialize, Deserialize)]
pub struct Transaction {
    pub nonce: u64,
    pub gas_price: U256,
    pub gas_limit: u64,
    pub to: Option<Address>,
    pub value: U256,
    pub data: Vec<u8>,
    pub v: u64,
    pub r: U256,
    pub s: U256,
}

impl Transaction {
    /// Calculate the hash of the transaction for signing (EIP-155).
    /// If chain_id is Some, it includes it for EIP-155 replay protection.
    pub fn hash_for_signing(&self, chain_id: Option<u64>) -> Hash {
        let mut stream = rlp::RlpStream::new();
        stream.begin_list(if chain_id.is_some() { 9 } else { 6 });
        stream.append(&self.nonce);
        stream.append(&self.gas_price);
        stream.append(&self.gas_limit);
        if let Some(addr) = self.to {
            stream.append(&addr);
        } else {
            stream.append_empty_data();
        }
        stream.append(&self.value);
        stream.append(&self.data);

        if let Some(id) = chain_id {
            stream.append(&id);
            stream.append(&0u8);
            stream.append(&0u8);
        }

        keccak256(&stream.out())
    }

    /// Sign the transaction using secp256k1 and optionally EIP-155 chain_id.
    pub fn sign(&mut self, secret_key: &SecretKey, chain_id: Option<u64>) -> Result<(), secp256k1::Error> {
        let hash = self.hash_for_signing(chain_id);
        let msg = Message::from_digest_slice(hash.as_bytes())?;
        
        let secp = Secp256k1::signing_only();
        let sig = secp.sign_ecdsa_recoverable(&msg, secret_key);
        let (rec_id, data) = sig.serialize_compact();
        
        let mut v = rec_id.to_i32() as u64;
        if let Some(id) = chain_id {
            v += id * 2 + 35;
        } else {
            v += 27;
        }

        self.v = v;
        self.r = U256::from_big_endian(&data[0..32]);
        self.s = U256::from_big_endian(&data[32..64]);

        Ok(())
    }

    /// Recover the sender address from the signature.
    pub fn recover_sender(&self) -> Result<Address, secp256k1::Error> {
        let (chain_id, rec_id_val) = if self.v >= 35 {
            let chain_id = (self.v - 35) / 2;
            (Some(chain_id), self.v - (chain_id * 2 + 35))
        } else {
            (None, self.v.saturating_sub(27))
        };

        let hash = self.hash_for_signing(chain_id);
        let msg = Message::from_digest_slice(hash.as_bytes())?;

        let rec_id = RecoveryId::from_i32(rec_id_val as i32)?;
        
        let mut data = [0u8; 64];
        self.r.to_big_endian(&mut data[0..32]);
        self.s.to_big_endian(&mut data[32..64]);

        let sig = RecoverableSignature::from_compact(&data, rec_id)?;
        
        let secp = Secp256k1::verification_only();
        let pubkey = secp.recover_ecdsa(&msg, &sig)?;
        
        // Ethereum address is right 20 bytes of keccak256 of uncompressed pubkey (excluding prefix byte)
        let serialized = pubkey.serialize_uncompressed();
        let hash = keccak256(&serialized[1..]);
        Ok(Address::from_slice(&hash.as_bytes()[12..32]))
    }
}
