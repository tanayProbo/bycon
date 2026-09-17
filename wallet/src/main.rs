//! BYCON Wallet — secp256k1 key management and EIP-155 transaction signing.
use protocol::transaction::{Address, Transaction};
use secp256k1::{Secp256k1, SecretKey, PublicKey};
use ethereum_types::U256;
use tiny_keccak::{Hasher, Keccak};

pub struct Wallet {
    secret_key: SecretKey,
    pub address: Address,
}

impl Wallet {
    pub fn generate() -> Self {
        let secp = Secp256k1::new();
        let (sk, pk) = secp.generate_keypair(&mut rand::thread_rng());
        Self { secret_key: sk, address: pubkey_to_address(&pk) }
    }

    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Result<Self, secp256k1::Error> {
        let sk = SecretKey::from_slice(bytes)?;
        let secp = Secp256k1::new();
        let pk = PublicKey::from_secret_key(&secp, &sk);
        Ok(Self { secret_key: sk, address: pubkey_to_address(&pk) })
    }

    pub fn sign_transaction(&self, tx: &mut Transaction, chain_id: Option<u64>) -> Result<(), secp256k1::Error> {
        tx.sign(&self.secret_key, chain_id)
    }

    pub fn secret_bytes(&self) -> [u8; 32] { self.secret_key.secret_bytes() }
}

pub fn pubkey_to_address(pk: &PublicKey) -> Address {
    let raw = pk.serialize_uncompressed();
    let hash = keccak256(&raw[1..]);
    Address::from_slice(&hash[12..])
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut k = Keccak::v256();
    let mut out = [0u8; 32];
    k.update(data); k.finalize(&mut out); out
}

fn main() {
    let wallet = Wallet::generate();
    let secret_hex = hex::encode(wallet.secret_bytes());
    println!("BYCON Wallet");
    println!("  Address : 0x{:x}", wallet.address);
    println!("  Secret  : 0x{}", secret_hex);
    println!();

    // Build a sample signed transaction (send 0 BYC to zero address, chain_id=1)
    let mut tx = Transaction {
        nonce: 0,
        gas_price: U256::zero(), // 0 gwei so we don't need initial balance
        gas_limit: 21_000,
        to: Some(Address::from_low_u64_be(1)),   // send to address 0x1
        value: U256::from(0u64),
        data: vec![],
        v: 0, r: U256::zero(), s: U256::zero(),
    };
    wallet.sign_transaction(&mut tx, Some(1)).expect("signing failed");

    let rlp_bytes = rlp::encode(&tx);
    let rlp_hex = hex::encode(&rlp_bytes);

    println!("Signed Transaction (RLP hex):");
    println!("  0x{}", rlp_hex);
    println!();
    println!("Send it to the node:");
    println!(
        r#"  curl -s -X POST http://localhost:8545 \
    -H "Content-Type: application/json" \
    -d '{{"jsonrpc":"2.0","method":"eth_sendRawTransaction","params":["0x{}"],"id":1}}'"#,
        rlp_hex
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generate_nonzero_address() {
        assert_ne!(Wallet::generate().address, Address::zero());
    }
    #[test]
    fn secret_key_roundtrip() {
        let w = Wallet::generate();
        let w2 = Wallet::from_secret_bytes(&w.secret_bytes()).unwrap();
        assert_eq!(w.address, w2.address);
    }
    #[test]
    fn sign_and_recover() {
        let w = Wallet::generate();
        let mut tx = Transaction { nonce: 0, gas_price: U256::from(1_000_000_000u64),
            gas_limit: 21_000, to: Some(Address::zero()), value: U256::zero(),
            data: vec![], v: 0, r: U256::zero(), s: U256::zero() };
        w.sign_transaction(&mut tx, Some(1)).unwrap();
        assert!(tx.v >= 35);
        assert_eq!(tx.recover_sender().unwrap(), w.address);
    }
}
