pub mod hash;
pub mod transaction;
pub mod block;
pub mod genesis;
pub mod account;
pub mod receipt;
pub mod trie;

pub fn init() {
    println!("Protocol crate initialized");
}
