use protocol::block::Block;
use primitive_types::H256;
use protocol::transaction::Address;

/// Represents the current state of the BFT consensus engine for a given height.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsensusState {
    /// Waiting for a block proposal from the designated proposer.
    Propose,
    /// Validators broadcast pre-votes for a proposed block.
    PreVote,
    /// Validators broadcast pre-commits after seeing 2/3+ pre-votes.
    PreCommit,
    /// The block has received 2/3+ pre-commits and is finalized.
    Commit,
}

/// A vote cast by a validator during the PreVote or PreCommit phase.
#[derive(Debug, Clone)]
pub struct Vote {
    pub validator: Address,
    pub block_hash: H256,
    pub height: u64,
    pub round: u32,
    pub signature: Vec<u8>, // BLS signature
}

/// Represents the consensus engine managing the BFT state machine.
pub struct ConsensusEngine {
    pub current_height: u64,
    pub current_round: u32,
    pub state: ConsensusState,
    pub active_validators: Vec<Address>,
}

impl ConsensusEngine {
    pub fn new(genesis_validators: Vec<Address>) -> Self {
        Self {
            current_height: 1,
            current_round: 0,
            state: ConsensusState::Propose,
            active_validators: genesis_validators,
        }
    }

    /// Handles an incoming block proposal.
    pub fn handle_proposal(&mut self, block: &Block) -> Result<(), &'static str> {
        if self.state != ConsensusState::Propose {
            return Err("Not in Propose state");
        }
        
        // TODO: Verify block signature, proposer turn, and block contents
        
        // Transition to PreVote after receiving a valid proposal
        self.state = ConsensusState::PreVote;
        Ok(())
    }

    /// Handles an incoming vote (PreVote or PreCommit).
    pub fn handle_vote(&mut self, vote: &Vote) -> Result<(), &'static str> {
        // TODO: Verify vote signature and validator stake weight.
        // If 2/3+ stake weight is reached for PreVote, transition to PreCommit.
        // If 2/3+ stake weight is reached for PreCommit, transition to Commit.
        
        Ok(())
    }

    /// Finalizes the block once the Commit state is reached.
    pub fn commit_block(&mut self, block: &Block) {
        assert_eq!(self.state, ConsensusState::Commit);
        
        // Block is finalized. Move to the next height.
        self.current_height += 1;
        self.current_round = 0;
        self.state = ConsensusState::Propose;
    }
}
