//! Node state management: RaftNode combining all state components.

use crate::election::ElectionState;
use crate::log::LogStore;
use crate::commit::CommitTracker;

pub type NodeId = u64;
pub type Term = u64;

/// Complete Raft node state.
#[derive(Debug)]
pub struct RaftNode {
    pub id: NodeId,
    pub election: ElectionState,
    pub log: LogStore,
    pub commit: CommitTracker,
}

impl RaftNode {
    pub fn new(id: NodeId, election_timeout_ms: u64) -> Self {
        Self {
            id,
            election: ElectionState::new(id, election_timeout_ms),
            log: LogStore::new(),
            commit: CommitTracker::new(),
        }
    }

    /// Current term of this node.
    pub fn current_term(&self) -> Term {
        self.election.current_term
    }

    /// Is this node the leader?
    pub fn is_leader(&self) -> bool {
        self.election.role == crate::election::NodeRole::Leader
    }

    /// Append a command to the log (leader only in real Raft).
    pub fn append_entry(&mut self, command: Vec<u8>) -> u64 {
        self.log.append(self.election.current_term, command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_node() {
        let node = RaftNode::new(1, 150);
        assert_eq!(node.id, 1);
        assert_eq!(node.current_term(), 0);
        assert!(!node.is_leader());
    }

    #[test]
    fn test_node_election_and_append() {
        let mut node = RaftNode::new(1, 150);
        crate::election::start_election(&mut node.election);
        node.election.receive_vote(2);
        node.election.receive_vote(3);
        node.election.become_leader();
        assert!(node.is_leader());

        let idx = node.append_entry(vec![42]);
        assert_eq!(idx, 1);
    }
}
