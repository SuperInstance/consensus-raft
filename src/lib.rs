//! # consensus-raft
//!
//! A Raft consensus protocol simulation implementing leader election,
//! log replication, commitment, and membership changes.
//!
//! ## Modules
//! - `election` — Leader election with timeout-based voting
//! - `log` — Replicated log entries and consistency checking
//! - `commit` — Log commitment and advancement
//! - `membership` — Cluster membership changes (C_old/C_new)
//! - `state` — Node state management (follower, candidate, leader)

pub mod election;
pub mod log;
pub mod commit;
pub mod membership;
pub mod state;

pub use election::{ElectionState, NodeRole, start_election, request_vote};
pub use log::{LogEntry, LogStore, LogIndex, LogTerm};
pub use commit::{CommitTracker, advance_commit_index};
pub use membership::{MembershipConfig, JointConsensus};
pub use state::{RaftNode, NodeId, Term};
