//! Membership changes: joint consensus (C_old, C_new) transitions.

use std::collections::HashSet;

/// Cluster membership configuration.
#[derive(Debug, Clone)]
pub struct MembershipConfig {
    pub nodes: HashSet<u64>,
    pub version: u64,
}

impl MembershipConfig {
    pub fn new(nodes: Vec<u64>) -> Self {
        Self {
            nodes: nodes.into_iter().collect(),
            version: 0,
        }
    }

    pub fn contains(&self, node_id: u64) -> bool {
        self.nodes.contains(&node_id)
    }

    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    pub fn majority(&self) -> usize {
        self.size() / 2 + 1
    }
}

/// Joint consensus state for membership transitions.
#[derive(Debug, Clone)]
pub struct JointConsensus {
    pub old_config: MembershipConfig,
    pub new_config: MembershipConfig,
    pub phase: ConsensusPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusPhase {
    /// Only C_old is active
    OldOnly,
    /// Both C_old and C_new must agree (joint consensus)
    Joint,
    /// Transition complete, only C_new is active
    NewOnly,
}

impl JointConsensus {
    pub fn new(old_config: MembershipConfig, new_config: MembershipConfig) -> Self {
        Self {
            old_config,
            new_config,
            phase: ConsensusPhase::OldOnly,
        }
    }

    /// Enter joint consensus phase.
    pub fn enter_joint(&mut self) {
        self.phase = ConsensusPhase::Joint;
    }

    /// Complete transition to new config.
    pub fn complete(&mut self) {
        self.phase = ConsensusPhase::NewOnly;
    }

    /// Check if a majority in both configs agree (during joint phase).
    pub fn is_joint_majority(&self, old_votes: &[u64], new_votes: &[u64]) -> bool {
        let old_count = old_votes.iter().filter(|id| self.old_config.contains(**id)).count();
        let new_count = new_votes.iter().filter(|id| self.new_config.contains(**id)).count();
        old_count >= self.old_config.majority() && new_count >= self.new_config.majority()
    }

    /// Get the effective configuration based on phase.
    pub fn effective_config(&self) -> &MembershipConfig {
        match self.phase {
            ConsensusPhase::NewOnly => &self.new_config,
            _ => &self.old_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_membership_contains() {
        let config = MembershipConfig::new(vec![1, 2, 3]);
        assert!(config.contains(1));
        assert!(!config.contains(4));
    }

    #[test]
    fn test_majority_calculation() {
        let config = MembershipConfig::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(config.majority(), 3);
    }

    #[test]
    fn test_joint_consensus_initial() {
        let old = MembershipConfig::new(vec![1, 2, 3]);
        let new = MembershipConfig::new(vec![1, 2, 3, 4, 5]);
        let jc = JointConsensus::new(old, new);
        assert_eq!(jc.phase, ConsensusPhase::OldOnly);
    }

    #[test]
    fn test_enter_joint_phase() {
        let old = MembershipConfig::new(vec![1, 2, 3]);
        let new = MembershipConfig::new(vec![1, 2, 3, 4, 5]);
        let mut jc = JointConsensus::new(old, new);
        jc.enter_joint();
        assert_eq!(jc.phase, ConsensusPhase::Joint);
    }

    #[test]
    fn test_joint_majority_both_agree() {
        let old = MembershipConfig::new(vec![1, 2, 3]);
        let new = MembershipConfig::new(vec![1, 2, 3, 4, 5]);
        let mut jc = JointConsensus::new(old, new);
        jc.enter_joint();
        let old_votes = vec![1, 2];
        let new_votes = vec![1, 2, 3];
        assert!(jc.is_joint_majority(&old_votes, &new_votes));
    }

    #[test]
    fn test_joint_majority_old_blocks() {
        let old = MembershipConfig::new(vec![1, 2, 3]);
        let new = MembershipConfig::new(vec![1, 2, 3, 4, 5]);
        let mut jc = JointConsensus::new(old, new);
        jc.enter_joint();
        let old_votes = vec![1]; // only 1 of 3 old nodes
        let new_votes = vec![1, 2, 3, 4];
        assert!(!jc.is_joint_majority(&old_votes, &new_votes));
    }

    #[test]
    fn test_complete_transition() {
        let old = MembershipConfig::new(vec![1, 2, 3]);
        let new = MembershipConfig::new(vec![1, 2, 3, 4, 5]);
        let mut jc = JointConsensus::new(old, new);
        jc.enter_joint();
        jc.complete();
        assert_eq!(jc.phase, ConsensusPhase::NewOnly);
        assert_eq!(jc.effective_config().size(), 5);
    }

    #[test]
    fn test_effective_config_old_phase() {
        let old = MembershipConfig::new(vec![1, 2, 3]);
        let new = MembershipConfig::new(vec![1, 2, 3, 4, 5]);
        let jc = JointConsensus::new(old, new);
        assert_eq!(jc.effective_config().size(), 3);
    }

    #[test]
    fn test_membership_version() {
        let mut config = MembershipConfig::new(vec![1, 2, 3]);
        assert_eq!(config.version, 0);
        config.version = 1;
        assert_eq!(config.version, 1);
    }
}
