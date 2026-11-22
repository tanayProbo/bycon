use std::collections::{HashMap, HashSet};
use protocol::block::Block;
use ethereum_types::H256;
use protocol::transaction::Address;

/// Represents the current state of the BFT consensus engine for a given height.
#[derive(Debug, Clone, PartialEq, Eq)]
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

/// Type of BFT vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoteType {
    PreVote,
    PreCommit,
}

/// A vote cast by a validator during the PreVote or PreCommit phase.
#[derive(Debug, Clone)]
pub struct Vote {
    pub validator: Address,
    pub block_hash: H256,
    pub height: u64,
    pub round: u32,
    pub vote_type: VoteType,
    pub signature: Vec<u8>,
}

/// Represents the consensus engine managing the BFT state machine.
pub struct ConsensusEngine {
    pub current_height: u64,
    pub current_round: u32,
    pub state: ConsensusState,
    pub active_validators: Vec<Address>,
    pub proposed_block_hash: Option<H256>,
    pre_votes: HashMap<(u64, u32, H256), HashSet<Address>>,
    pre_commits: HashMap<(u64, u32, H256), HashSet<Address>>,
}

impl ConsensusEngine {
    pub fn new(genesis_validators: Vec<Address>) -> Self {
        Self {
            current_height: 1,
            current_round: 0,
            state: ConsensusState::Propose,
            active_validators: genesis_validators,
            proposed_block_hash: None,
            pre_votes: HashMap::new(),
            pre_commits: HashMap::new(),
        }
    }

    /// Returns the 2/3+ quorum threshold for the active validator set.
    pub fn quorum_threshold(&self) -> usize {
        if self.active_validators.is_empty() {
            return 0;
        }
        (self.active_validators.len() * 2) / 3 + 1
    }

    /// Returns the expected proposer for the current height and round.
    pub fn expected_proposer(&self) -> Option<Address> {
        if self.active_validators.is_empty() {
            return None;
        }
        let idx = ((self.current_height as usize) + (self.current_round as usize)) % self.active_validators.len();
        Some(self.active_validators[idx])
    }

    /// Handles an incoming block proposal.
    pub fn handle_proposal(&mut self, block: &Block) -> Result<(), &'static str> {
        if self.state != ConsensusState::Propose {
            return Err("Not in Propose state");
        }
        if block.header.number != self.current_height {
            return Err("Block height mismatch");
        }
        if let Some(expected) = self.expected_proposer() {
            if block.header.proposer != expected {
                return Err("Unexpected block proposer");
            }
        }

        self.proposed_block_hash = Some(block.hash());
        self.state = ConsensusState::PreVote;
        Ok(())
    }

    /// Handles an incoming vote (PreVote or PreCommit).
    pub fn handle_vote(&mut self, vote: &Vote) -> Result<(), &'static str> {
        if vote.height != self.current_height || vote.round != self.current_round {
            return Err("Vote height or round mismatch");
        }
        if !self.active_validators.contains(&vote.validator) {
            return Err("Unknown validator");
        }
        if vote.signature.is_empty() {
            return Err("Empty vote signature");
        }

        let key = (vote.height, vote.round, vote.block_hash);
        let threshold = self.quorum_threshold();

        match vote.vote_type {
            VoteType::PreVote => {
                if self.state != ConsensusState::PreVote {
                    return Err("Not in PreVote state");
                }
                let voters = self.pre_votes.entry(key).or_default();
                voters.insert(vote.validator);
                if voters.len() >= threshold {
                    self.state = ConsensusState::PreCommit;
                }
            }
            VoteType::PreCommit => {
                if self.state != ConsensusState::PreCommit {
                    return Err("Not in PreCommit state");
                }
                let voters = self.pre_commits.entry(key).or_default();
                voters.insert(vote.validator);
                if voters.len() >= threshold {
                    self.state = ConsensusState::Commit;
                }
            }
        }

        Ok(())
    }

    /// Finalizes the block once the Commit state is reached.
    pub fn commit_block(&mut self, block: &Block) -> Result<(), &'static str> {
        if self.state != ConsensusState::Commit {
            return Err("Not in Commit state");
        }
        if block.header.number != self.current_height {
            return Err("Block height mismatch on commit");
        }

        self.current_height += 1;
        self.current_round = 0;
        self.state = ConsensusState::Propose;
        self.proposed_block_hash = None;
        self.pre_votes.clear();
        self.pre_commits.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::block::BlockHeader;

    fn dummy_block(height: u64, proposer: Address) -> Block {
        Block {
            header: BlockHeader {
                parent_hash: H256::zero(),
                number: height,
                nonce: 0,
                state_root: H256::zero(),
                transactions_root: H256::zero(),
                receipts_root: H256::zero(),
                timestamp: 0,
                proposer,
            },
            transactions: vec![],
        }
    }

    #[test]
    fn bft_full_round_reaches_commit() {
        let v1 = Address::from_low_u64_be(1);
        let v2 = Address::from_low_u64_be(2);
        let v3 = Address::from_low_u64_be(3);
        let mut engine = ConsensusEngine::new(vec![v1, v2, v3]);

        let proposer = engine.expected_proposer().unwrap();
        let block = dummy_block(1, proposer);
        let bhash = block.hash();

        assert!(engine.handle_proposal(&block).is_ok());
        assert_eq!(engine.state, ConsensusState::PreVote);

        for v in [v1, v2, v3] {
            engine.handle_vote(&Vote {
                validator: v,
                block_hash: bhash,
                height: 1,
                round: 0,
                vote_type: VoteType::PreVote,
                signature: vec![1],
            }).unwrap();
        }
        assert_eq!(engine.state, ConsensusState::PreCommit);

        for v in [v1, v2, v3] {
            engine.handle_vote(&Vote {
                validator: v,
                block_hash: bhash,
                height: 1,
                round: 0,
                vote_type: VoteType::PreCommit,
                signature: vec![1],
            }).unwrap();
        }
        assert_eq!(engine.state, ConsensusState::Commit);
        assert!(engine.commit_block(&block).is_ok());
        assert_eq!(engine.current_height, 2);
        assert_eq!(engine.state, ConsensusState::Propose);
    }
}
