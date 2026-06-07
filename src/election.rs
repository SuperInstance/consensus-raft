//! Leader election: timeout-based voting, term progression, split vote handling.

use crate::state::{NodeId, Term};

/// Possible roles a Raft node can assume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Follower,
    Candidate,
    Leader,
}

/// Election state tracking for a node.
#[derive(Debug, Clone)]
pub struct ElectionState {
    pub node_id: NodeId,
    pub current_term: Term,
    pub voted_for: Option<NodeId>,
    pub role: NodeRole,
    pub votes_received: Vec<NodeId>,
    pub election_timeout_ms: u64,
    pub leader_id: Option<NodeId>,
}

impl ElectionState {
    pub fn new(node_id: NodeId, election_timeout_ms: u64) -> Self {
        Self {
            node_id,
            current_term: 0,
            voted_for: None,
            role: NodeRole::Follower,
            votes_received: Vec::new(),
            election_timeout_ms,
            leader_id: None,
        }
    }

    /// Increment term, become candidate, vote for self.
    pub fn become_candidate(&mut self) -> Term {
        self.current_term += 1;
        self.role = NodeRole::Candidate;
        self.voted_for = Some(self.node_id);
        self.votes_received = vec![self.node_id];
        self.leader_id = None;
        self.current_term
    }

    /// Become leader (after winning election).
    pub fn become_leader(&mut self) {
        self.role = NodeRole::Leader;
        self.leader_id = Some(self.node_id);
    }

    /// Step down to follower for a given term.
    pub fn step_down(&mut self, term: Term, leader_id: Option<NodeId>) {
        if term > self.current_term {
            self.current_term = term;
        }
        self.role = NodeRole::Follower;
        self.voted_for = None;
        self.votes_received.clear();
        self.leader_id = leader_id;
    }

    /// Record a vote received from another node.
    pub fn receive_vote(&mut self, from: NodeId) {
        if self.role == NodeRole::Candidate && !self.votes_received.contains(&from) {
            self.votes_received.push(from);
        }
    }

    /// Check if we have a majority of votes.
    pub fn has_majority(&self, cluster_size: usize) -> bool {
        let majority = cluster_size / 2 + 1;
        self.votes_received.len() >= majority
    }
}

/// Start a new election: increment term, become candidate, vote for self.
pub fn start_election(state: &mut ElectionState) -> Term {
    state.become_candidate()
}

/// Process a vote request from a candidate. Returns true if vote is granted.
pub fn request_vote(
    state: &mut ElectionState,
    candidate_id: NodeId,
    candidate_term: Term,
) -> bool {
    if candidate_term < state.current_term {
        return false;
    }
    if candidate_term > state.current_term {
        state.step_down(candidate_term, None);
    }
    // Can vote if we haven't voted yet, or already voted for this candidate
    let can_vote = state.voted_for.is_none() || state.voted_for == Some(candidate_id);
    if can_vote {
        state.voted_for = Some(candidate_id);
        return true;
    }
    false
}

/// Process an append entries (heartbeat) from a leader.
pub fn receive_heartbeat(
    state: &mut ElectionState,
    leader_id: NodeId,
    leader_term: Term,
) -> bool {
    if leader_term < state.current_term {
        return false;
    }
    state.step_down(leader_term, Some(leader_id));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state_is_follower() {
        let state = ElectionState::new(1, 150);
        assert_eq!(state.role, NodeRole::Follower);
        assert_eq!(state.current_term, 0);
        assert!(state.voted_for.is_none());
    }

    #[test]
    fn test_become_candidate_increments_term() {
        let mut state = ElectionState::new(1, 150);
        let term = start_election(&mut state);
        assert_eq!(term, 1);
        assert_eq!(state.role, NodeRole::Candidate);
        assert_eq!(state.voted_for, Some(1));
    }

    #[test]
    fn test_candidate_wins_with_majority() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state);
        state.receive_vote(2);
        state.receive_vote(3);
        assert!(state.has_majority(5));
    }

    #[test]
    fn test_candidate_loses_without_majority() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state);
        state.receive_vote(2);
        assert!(!state.has_majority(5));
    }

    #[test]
    fn test_become_leader_after_majority() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state);
        state.receive_vote(2);
        state.receive_vote(3);
        state.become_leader();
        assert_eq!(state.role, NodeRole::Leader);
    }

    #[test]
    fn test_step_down_on_higher_term() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state);
        state.step_down(5, Some(3));
        assert_eq!(state.role, NodeRole::Follower);
        assert_eq!(state.current_term, 5);
        assert_eq!(state.leader_id, Some(3));
    }

    #[test]
    fn test_vote_reject_lower_term() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state); // term 1
        let granted = request_vote(&mut state, 2, 0); // term 0 < 1
        assert!(!granted);
    }

    #[test]
    fn test_vote_grant_for_higher_term() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state); // term 1
        let granted = request_vote(&mut state, 2, 2); // term 2 > 1
        assert!(granted);
        assert_eq!(state.voted_for, Some(2));
    }

    #[test]
    fn test_heartbeat_from_valid_leader() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state);
        let ok = receive_heartbeat(&mut state, 3, 2);
        assert!(ok);
        assert_eq!(state.role, NodeRole::Follower);
        assert_eq!(state.leader_id, Some(3));
    }

    #[test]
    fn test_heartbeat_reject_lower_term() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state); // term 1
        let ok = receive_heartbeat(&mut state, 3, 0);
        assert!(!ok);
    }

    #[test]
    fn test_split_vote_scenario() {
        // 5-node cluster, 2 candidates split votes
        let mut node1 = ElectionState::new(1, 150);
        let mut node2 = ElectionState::new(2, 200);
        start_election(&mut node1); // term 1
        start_election(&mut node2); // term 2

        // Node1 gets votes from {1, 3}, Node2 gets votes from {2, 4}
        node1.receive_vote(3);
        node2.receive_vote(4);

        // Neither has majority of 5
        assert!(!node1.has_majority(5));
        assert!(!node2.has_majority(5));
    }

    #[test]
    fn test_duplicate_vote_ignored() {
        let mut state = ElectionState::new(1, 150);
        start_election(&mut state);
        state.receive_vote(2);
        state.receive_vote(2); // duplicate
        assert_eq!(state.votes_received.len(), 2); // self + node2
    }

    #[test]
    fn test_term_progression() {
        let mut state = ElectionState::new(1, 150);
        assert_eq!(state.current_term, 0);
        start_election(&mut state);
        assert_eq!(state.current_term, 1);
        start_election(&mut state);
        assert_eq!(state.current_term, 2);
    }
}
