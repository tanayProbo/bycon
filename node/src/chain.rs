use protocol::block::Block;
use protocol::hash::Hash;

#[derive(Debug)]
pub enum ChainError {
    InvalidParentHash,
    InvalidBlockHash,
}

pub struct Blockchain {
    pub blocks: Vec<Block>,
}

impl Blockchain {
    pub fn new(genesis: Block) -> Self {
        Self {
            blocks: vec![genesis],
        }
    }

    pub fn head(&self) -> &Block {
        self.blocks.last().unwrap()
    }

    pub fn add_block(&mut self, block: Block) -> Result<(), ChainError> {
        let head = self.head();
        
        if block.header.parent_hash != head.hash() {
            return Err(ChainError::InvalidParentHash);
        }

        self.blocks.push(block);
        Ok(())
    }
}
