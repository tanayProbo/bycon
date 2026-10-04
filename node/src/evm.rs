//! EVM execution — Phase 3.
//!
//! A basic stack-based EVM interpreter supporting core opcodes:
//! PUSH1, POP, ADD, SUB, MUL, MLOAD, MSTORE, SLOAD, SSTORE, RETURN.
//! Also handles contract creation vs contract calls.

use protocol::transaction::Address;
use protocol::hash::{keccak256, Hash};
use crate::state::{StateDb, StateError};
use ethereum_types::U256;
use tracing::info;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EvmError {
    #[error("State error: {0}")]
    State(#[from] StateError),
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Stack overflow")]
    StackOverflow,
    #[error("Invalid opcode: {0:#x}")]
    InvalidOpcode(u8),
    #[error("Out of bounds memory access")]
    MemoryOutOfBounds,
    #[error("Revert")]
    Revert,
}

// Opcodes
const ADD: u8 = 0x01;
const MUL: u8 = 0x02;
const SUB: u8 = 0x03;
const POP: u8 = 0x50;
const MLOAD: u8 = 0x51;
const MSTORE: u8 = 0x52;
const SLOAD: u8 = 0x54;
const SSTORE: u8 = 0x55;
const PUSH1: u8 = 0x60;
const RETURN: u8 = 0xf3;

pub struct Evm<'a> {
    state: &'a StateDb,
}

impl<'a> Evm<'a> {
    pub fn new(state: &'a StateDb) -> Self {
        Self { state }
    }

    /// Simulates executing a smart contract call or deployment.
    pub fn execute_contract(
        &self,
        sender: &Address,
        sender_nonce: u64,
        contract: Option<&Address>,
        data: &[u8],
    ) -> Result<Option<Address>, EvmError> {
        let (target, is_creation) = if let Some(c) = contract {
            (*c, false)
        } else {
            // Contract creation
            let mut stream = rlp::RlpStream::new_list(2);
            stream.append(sender);
            stream.append(&sender_nonce);
            let hash = keccak256(&stream.out());
            let new_address = Address::from_slice(&hash.as_bytes()[12..32]);

            // Initialize account
            let acc = self.state.get_account(&new_address).unwrap_or_default();
            self.state.put_account(&new_address, &acc)?;

            (new_address, true)
        };

        let bytecode = if is_creation {
            data.to_vec()
        } else {
            let acc = self.state.get_account(&target)?;
            self.state.get_code(&acc.code_hash)?
        };

        if bytecode.is_empty() {
            return Ok(if is_creation { Some(target) } else { None });
        }

        info!("EVM: Executing {} bytes of bytecode at target {}", bytecode.len(), target);

        // EVM execution context
        let mut pc = 0;
        let mut stack: Vec<U256> = Vec::with_capacity(1024);
        let mut memory: Vec<u8> = vec![0; 4096]; // 4KB fixed memory for simplicity
        let mut return_data = Vec::new();

        macro_rules! pop {
            () => {
                stack.pop().ok_or(EvmError::StackUnderflow)?
            };
        }
        macro_rules! push {
            ($val:expr) => {
                if stack.len() >= 1024 { return Err(EvmError::StackOverflow); }
                stack.push($val);
            };
        }

        while pc < bytecode.len() {
            let op = bytecode[pc];
            pc += 1;

            match op {
                ADD => {
                    let a = pop!();
                    let b = pop!();
                    push!(a.overflowing_add(b).0);
                }
                MUL => {
                    let a = pop!();
                    let b = pop!();
                    push!(a.overflowing_mul(b).0);
                }
                SUB => {
                    let a = pop!();
                    let b = pop!();
                    push!(a.overflowing_sub(b).0);
                }
                POP => {
                    pop!();
                }
                MLOAD => {
                    let offset = pop!().as_usize();
                    if offset + 32 > memory.len() { return Err(EvmError::MemoryOutOfBounds); }
                    let val = U256::from_big_endian(&memory[offset..offset+32]);
                    push!(val);
                }
                MSTORE => {
                    let offset = pop!().as_usize();
                    let val = pop!();
                    if offset + 32 > memory.len() { return Err(EvmError::MemoryOutOfBounds); }
                    val.to_big_endian(&mut memory[offset..offset+32]);
                }
                SLOAD => {
                    let key_val = pop!();
                    let mut key_bytes = [0u8; 32];
                    key_val.to_big_endian(&mut key_bytes);
                    let key_hash = Hash::from_slice(&key_bytes);

                    let val_hash = self.state.get_storage(&target, key_hash)?;
                    let val = U256::from_big_endian(val_hash.as_bytes());
                    push!(val);
                }
                SSTORE => {
                    let key_val = pop!();
                    let val_val = pop!();
                    
                    let mut key_bytes = [0u8; 32];
                    key_val.to_big_endian(&mut key_bytes);
                    let key_hash = Hash::from_slice(&key_bytes);

                    let mut val_bytes = [0u8; 32];
                    val_val.to_big_endian(&mut val_bytes);
                    let val_hash = Hash::from_slice(&val_bytes);

                    self.state.set_storage(&target, key_hash, val_hash)?;
                }
                PUSH1 => {
                    if pc >= bytecode.len() { return Err(EvmError::InvalidOpcode(PUSH1)); }
                    let val = U256::from(bytecode[pc]);
                    pc += 1;
                    push!(val);
                }
                RETURN => {
                    let offset = pop!().as_usize();
                    let size = pop!().as_usize();
                    if offset + size > memory.len() { return Err(EvmError::MemoryOutOfBounds); }
                    return_data = memory[offset..offset+size].to_vec();
                    break;
                }
                _ => {
                    return Err(EvmError::InvalidOpcode(op));
                }
            }
        }

        if is_creation && !return_data.is_empty() {
            // Save the returned runtime bytecode
            self.state.set_code(&target, &return_data)?;
        }

        Ok(if is_creation { Some(target) } else { None })
    }
}
