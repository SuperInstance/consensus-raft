//! Replicated log: entries, storage, consistency checks.

pub type LogIndex = u64;
pub type LogTerm = u64;

/// A single log entry in the Raft log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub index: LogIndex,
    pub term: LogTerm,
    pub command: Vec<u8>,
}

/// In-memory log store for Raft.
#[derive(Debug, Clone)]
pub struct LogStore {
    entries: Vec<LogEntry>,
}

impl Default for LogStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LogStore {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Append an entry to the log.
    pub fn append(&mut self, term: LogTerm, command: Vec<u8>) -> LogIndex {
        let index = self.entries.len() as LogIndex + 1;
        self.entries.push(LogEntry { index, term, command });
        index
    }

    /// Get entry at 1-based index.
    pub fn get(&self, index: LogIndex) -> Option<&LogEntry> {
        if index == 0 || index as usize > self.entries.len() {
            return None;
        }
        Some(&self.entries[(index - 1) as usize])
    }

    /// Get the last log index.
    pub fn last_index(&self) -> LogIndex {
        self.entries.len() as LogIndex
    }

    /// Get the term of the last entry (0 if empty).
    pub fn last_term(&self) -> LogTerm {
        self.entries.last().map(|e| e.term).unwrap_or(0)
    }

    /// Truncate log from index onwards (for conflicting entries).
    pub fn truncate_from(&mut self, index: LogIndex) {
        if index == 0 {
            self.entries.clear();
        } else {
            self.entries.truncate((index - 1) as usize);
        }
    }

    /// Check if log is at least as up-to-date as another (for vote granting).
    /// A log is more up-to-date if its last index is higher, or if same index
    /// but higher term.
    pub fn is_up_to_date(&self, last_index: LogIndex, last_term: LogTerm) -> bool {
        if self.last_term() != last_term {
            self.last_term() <= last_term
        } else {
            self.last_index() <= last_index
        }
    }

    /// Get entries in range [start, end] inclusive (1-based).
    pub fn get_range(&self, start: LogIndex, end: LogIndex) -> Vec<LogEntry> {
        let s = start.saturating_sub(1) as usize;
        let e = std::cmp::min(end as usize, self.entries.len());
        if s >= e {
            return Vec::new();
        }
        self.entries[s..e].to_vec()
    }

    /// Check consistency: verify entry at prev_index has prev_term.
    pub fn is_consistent(&self, prev_index: LogIndex, prev_term: LogTerm) -> bool {
        if prev_index == 0 {
            return true;
        }
        match self.get(prev_index) {
            Some(e) => e.term == prev_term,
            None => false,
        }
    }

    /// Number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the log empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_and_get() {
        let mut log = LogStore::new();
        let idx = log.append(1, vec![1, 2, 3]);
        assert_eq!(idx, 1);
        let entry = log.get(1).unwrap();
        assert_eq!(entry.term, 1);
        assert_eq!(entry.command, vec![1, 2, 3]);
    }

    #[test]
    fn test_sequential_indices() {
        let mut log = LogStore::new();
        assert_eq!(log.append(1, vec![]), 1);
        assert_eq!(log.append(1, vec![]), 2);
        assert_eq!(log.append(2, vec![]), 3);
    }

    #[test]
    fn test_last_index_and_term() {
        let mut log = LogStore::new();
        assert_eq!(log.last_index(), 0);
        assert_eq!(log.last_term(), 0);
        log.append(1, vec![]);
        log.append(2, vec![]);
        assert_eq!(log.last_index(), 2);
        assert_eq!(log.last_term(), 2);
    }

    #[test]
    fn test_truncate() {
        let mut log = LogStore::new();
        log.append(1, vec![1]);
        log.append(1, vec![2]);
        log.append(2, vec![3]);
        log.truncate_from(3);
        assert_eq!(log.last_index(), 2);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn test_is_up_to_date_same_term_higher_index() {
        let mut log = LogStore::new();
        log.append(1, vec![]);
        log.append(1, vec![]);
        assert!(log.is_up_to_date(3, 1)); // other has higher index, same term
    }

    #[test]
    fn test_is_up_to_date_higher_term() {
        let mut log = LogStore::new();
        log.append(1, vec![]);
        log.append(1, vec![]);
        assert!(log.is_up_to_date(1, 2)); // other has higher term
    }

    #[test]
    fn test_not_up_to_date_higher_local_term() {
        let mut log = LogStore::new();
        log.append(5, vec![]);
        assert!(!log.is_up_to_date(1, 1)); // other has lower term
    }

    #[test]
    fn test_get_range() {
        let mut log = LogStore::new();
        log.append(1, vec![1]);
        log.append(1, vec![2]);
        log.append(2, vec![3]);
        log.append(2, vec![4]);
        let range = log.get_range(2, 3);
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].index, 2);
        assert_eq!(range[1].index, 3);
    }

    #[test]
    fn test_consistency_check_pass() {
        let mut log = LogStore::new();
        log.append(1, vec![]);
        log.append(2, vec![]);
        assert!(log.is_consistent(1, 1));
        assert!(log.is_consistent(2, 2));
    }

    #[test]
    fn test_consistency_check_fail() {
        let mut log = LogStore::new();
        log.append(1, vec![]);
        log.append(2, vec![]);
        assert!(!log.is_consistent(1, 3)); // wrong term
        assert!(!log.is_consistent(5, 1)); // missing index
    }

    #[test]
    fn test_empty_log_consistency() {
        let log = LogStore::new();
        assert!(log.is_consistent(0, 0));
    }
}
