//! Commit tracking: commit index advancement, safety checks.

use crate::log::LogStore;
use crate::state::Term;

/// Tracks commit index across the cluster.
#[derive(Debug, Clone)]
pub struct CommitTracker {
    pub commit_index: u64,
    pub last_applied: u64,
}

impl Default for CommitTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitTracker {
    pub fn new() -> Self {
        Self {
            commit_index: 0,
            last_applied: 0,
        }
    }

    /// Advance the commit index to the highest index stored on a majority of servers.
    pub fn try_advance(&mut self, match_indices: &[u64], cluster_size: usize, current_term: Term, log: &LogStore) -> bool {
        let majority = cluster_size / 2 + 1;
        let mut advanced = false;

        // Try indices from high to low
        let max_index = match_indices.iter().copied().max().unwrap_or(0);
        for n in (self.commit_index + 1..=max_index).rev() {
            // Count how many servers have this index
            let count = match_indices.iter().filter(|&&idx| idx >= n).count();
            if count >= majority {
                // Only commit entries from current term
                if let Some(entry) = log.get(n) {
                    if entry.term == current_term {
                        self.commit_index = n;
                        advanced = true;
                        break;
                    }
                }
            }
        }
        advanced
    }

    /// Apply committed entries up to commit_index.
    pub fn apply_entries(&mut self, log: &LogStore) -> Vec<Vec<u8>> {
        let mut applied = Vec::new();
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            if let Some(entry) = log.get(self.last_applied) {
                applied.push(entry.command.clone());
            }
        }
        applied
    }

    /// Check if a read at the given index is linearizable (committed).
    pub fn is_committed(&self, index: u64) -> bool {
        index <= self.commit_index
    }
}

/// Advance commit index on the leader based on follower match indices.
pub fn advance_commit_index(
    tracker: &mut CommitTracker,
    match_indices: &[u64],
    cluster_size: usize,
    current_term: Term,
    log: &LogStore,
) -> bool {
    tracker.try_advance(match_indices, cluster_size, current_term, log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogStore;

    #[test]
    fn test_initial_commit_state() {
        let tracker = CommitTracker::new();
        assert_eq!(tracker.commit_index, 0);
        assert_eq!(tracker.last_applied, 0);
    }

    #[test]
    fn test_advance_with_majority() {
        let mut log = LogStore::new();
        log.append(1, vec![1]);
        log.append(1, vec![2]);
        log.append(1, vec![3]);

        let mut tracker = CommitTracker::new();
        // 5 nodes: leader + 4 followers. match_indices[0] = leader
        let match_indices = vec![3, 3, 2, 0, 0];
        let advanced = tracker.try_advance(&match_indices, 5, 1, &log);
        assert!(advanced);
        assert!(tracker.commit_index >= 2);
    }

    #[test]
    fn test_no_advance_without_majority() {
        let mut log = LogStore::new();
        log.append(1, vec![1]);
        log.append(1, vec![2]);

        let mut tracker = CommitTracker::new();
        let match_indices = vec![2, 1, 0, 0, 0];
        let advanced = tracker.try_advance(&match_indices, 5, 1, &log);
        assert!(!advanced);
    }

    #[test]
    fn test_apply_entries() {
        let mut log = LogStore::new();
        log.append(1, vec![10]);
        log.append(1, vec![20]);

        let mut tracker = CommitTracker::new();
        tracker.commit_index = 2;
        let applied = tracker.apply_entries(&log);
        assert_eq!(applied.len(), 2);
        assert_eq!(applied[0], vec![10]);
        assert_eq!(applied[1], vec![20]);
        assert_eq!(tracker.last_applied, 2);
    }

    #[test]
    fn test_is_committed() {
        let mut tracker = CommitTracker::new();
        tracker.commit_index = 5;
        assert!(tracker.is_committed(3));
        assert!(tracker.is_committed(5));
        assert!(!tracker.is_committed(6));
    }

    #[test]
    fn test_only_commit_current_term() {
        let mut log = LogStore::new();
        log.append(1, vec![1]); // term 1
        log.append(1, vec![2]); // term 1
        log.append(2, vec![3]); // term 2

        let mut tracker = CommitTracker::new();
        let match_indices = vec![3, 3, 3, 3, 3];
        // Current term is 2, so only index 3 (term 2) triggers commit
        let advanced = tracker.try_advance(&match_indices, 5, 2, &log);
        assert!(advanced);
        assert_eq!(tracker.commit_index, 3);
    }

    #[test]
    fn test_incremental_advance() {
        let mut log = LogStore::new();
        for _ in 0..5 {
            log.append(1, vec![]);
        }

        let mut tracker = CommitTracker::new();
        let match_indices = vec![3, 3, 3, 0, 0];
        tracker.try_advance(&match_indices, 5, 1, &log);
        let first = tracker.commit_index;

        let match_indices2 = vec![5, 5, 4, 0, 0];
        tracker.try_advance(&match_indices2, 5, 1, &log);
        assert!(tracker.commit_index > first);
    }
}
