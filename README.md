# consensus-raft

A Raft consensus protocol simulation implementing leader election, log replication, commitment, and membership changes.

## Features

- **Leader Election** — Timeout-based voting with term progression and split vote handling
- **Log Replication** — In-memory log store with consistency checking and truncation
- **Commit Tracking** — Commit index advancement with majority-based safety
- **Membership Changes** — Joint consensus (C_old/C_new) transitions
- **Node State** — Complete Raft node combining all state components

## Modules

| Module | Description |
|--------|-------------|
| `election` | Leader election with timeout-based voting |
| `log` | Replicated log entries and consistency checking |
| `commit` | Log commitment and advancement |
| `membership` | Cluster membership changes (C_old/C_new) |
| `state` | Node state management (follower, candidate, leader) |

## Usage

```rust
use consensus_raft::state::RaftNode;
use consensus_raft::election::start_election;

let mut node = RaftNode::new(1, 150);
start_election(&mut node.election);
node.election.receive_vote(2);
node.election.receive_vote(3);
node.election.become_leader();
node.append_entry(vec![42]);
```

## Testing

```bash
cargo test    # 42 tests
cargo clippy  # zero warnings
```

## License

MIT
