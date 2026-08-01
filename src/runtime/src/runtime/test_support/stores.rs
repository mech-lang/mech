use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{InMemoryAppendEventFailureKind, InMemoryStore};

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

#[derive(Clone, Debug)]
pub(crate) struct AppendEventFailureProbe {
    failures: Arc<Mutex<VecDeque<InMemoryAppendEventFailureKind>>>,
}

impl AppendEventFailureProbe {
    pub(crate) fn new() -> (InMemoryStore, Self) {
        let failures = Arc::new(Mutex::new(VecDeque::new()));
        let store = InMemoryStore::new().with_append_event_failures_for_test(failures.clone());
        (store, Self { failures })
    }

    pub(crate) fn fail_next_transaction_aborted(&self) {
        self.failures
            .lock()
            .unwrap()
            .push_back(InMemoryAppendEventFailureKind::TransactionAborted);
    }

    pub(crate) fn fail_next_effect_aborted(&self) {
        self.failures
            .lock()
            .unwrap()
            .push_back(InMemoryAppendEventFailureKind::EffectAborted);
    }
}
