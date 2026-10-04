//! EVM execution stub — Phase 1.
//!
//! Contract calls are logged but not yet executed.
//! Contract *creation* now derives addresses deterministically via
//! `keccak256(RLP([sender, nonce]))[12..]`, matching Ethereum's CREATE formula.
//! A full opcode interpreter is planned for Phase 3.

use protocol::transaction::Address;
use protocol::hash::keccak256;
use crate::state::{StateDb, StateError};
use tracing::info;

/// A stub for the Ethereum Virtual Machine (EVM).
pub struct Evm<'a> {
    state: &'a StateDb,
}

impl<'a> Evm<'a> {
    pub fn new(state: &'a StateDb) -> Self {
        Self { state }
    }

    /// Simulates executing a smart contract call or deployment.
    ///
    /// # Arguments
    /// * `sender`       — the transaction sender address
    /// * `sender_nonce` — the sender's nonce *from the transaction* (before increment),
    ///                    used to deterministically derive the contract address on creation
    /// * `contract`     — `Some(addr)` for a call, `None` for contract creation
    /// * `data`         — calldata or deployment bytecode
    ///
    /// Returns `Some(new_address)` on contract creation, `None` on a plain call.
    pub fn execute_contract(
        &self,
        sender: &Address,
        sender_nonce: u64,
        contract: Option<&Address>,
        data: &[u8],
    ) -> Result<Option<Address>, StateError> {
        if let Some(c) = contract {
            info!(
                "EVM: Calling contract {} from {} with {} bytes of data",
                c, sender, data.len()
            );
            // Phase 3: load bytecode for `c` from code-db and interpret opcodes.
            Ok(None)
        } else {
            // --- Contract creation ---
            // Ethereum CREATE address = keccak256(RLP([sender, nonce]))[12..32]
            let mut stream = rlp::RlpStream::new_list(2);
            stream.append(sender);
            stream.append(&sender_nonce);
            let hash = keccak256(&stream.out());
            let new_address = Address::from_slice(&hash.as_bytes()[12..32]);

            info!(
                "EVM: Deploying new contract at {} (sender={}, nonce={})",
                new_address, sender, sender_nonce
            );

            // Initialise the contract account (code_hash will be set in Phase 3
            // once we actually store and execute bytecode).
            let acc = self.state.get_account(&new_address).unwrap_or_default();
            self.state.put_account(&new_address, &acc)?;

            Ok(Some(new_address))
        }
    }
}
