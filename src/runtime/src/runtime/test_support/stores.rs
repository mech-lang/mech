use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::InMemoryStore;

#[derive(Clone, Debug)]
pub(crate) struct StoreCommitProbe {
    calls: Arc<AtomicUsize>,
}

impl StoreCommitProbe {
    pub(crate) fn new() -> (InMemoryStore, Self) {
        let calls = Arc::new(AtomicUsize::new(0));
        let store = InMemoryStore::new().with_commit_runtime_counter_for_test(calls.clone());
        (store, Self { calls })
    }

    pub(crate) fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}
