//! Markable storage for events visible through a [`RuntimeContext`](crate::RuntimeContext).

use core::{
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};

use mech_core::{MResult, MechError};

use crate::{RuntimeEvent, RuntimeInvalidOperationError};

static NEXT_OWNER_ID: AtomicU64 = AtomicU64::new(1);

fn next_owner_id() -> NonZeroU64 {
    let owner_id = NEXT_OWNER_ID
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .expect("runtime context event owner IDs exhausted");
    NonZeroU64::new(owner_id).expect("runtime context event owner IDs start at one")
}

#[derive(Debug, PartialEq, Eq)]
struct RuntimeContextEventMarkPosition {
    visible_start: usize,
    storage_len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimeContextEventMark {
    owner_id: NonZeroU64,
    generation: u64,
    position: Arc<RuntimeContextEventMarkPosition>,
}

#[derive(Debug)]
pub(crate) struct RuntimeContextEvents {
    storage: Vec<RuntimeEvent>,
    visible_start: usize,
    owner_id: NonZeroU64,
    generation: u64,
    active_marks: Mutex<Vec<Weak<RuntimeContextEventMarkPosition>>>,
}

impl RuntimeContextEvents {
    pub(crate) fn new() -> Self {
        Self {
            storage: Vec::new(),
            visible_start: 0,
            owner_id: next_owner_id(),
            generation: 0,
            active_marks: Mutex::new(Vec::new()),
        }
    }

    pub(crate) fn mark(&self) -> RuntimeContextEventMark {
        let position = Arc::new(RuntimeContextEventMarkPosition {
            visible_start: self.visible_start,
            storage_len: self.storage.len(),
        });
        self.active_marks
            .lock()
            .expect("runtime context event mark registry poisoned")
            .push(Arc::downgrade(&position));
        RuntimeContextEventMark {
            owner_id: self.owner_id,
            generation: self.generation,
            position,
        }
    }

    pub(crate) fn push(&mut self, event: RuntimeEvent) {
        self.storage.push(event);
    }

    pub(crate) fn restore(&mut self, mark: &RuntimeContextEventMark) -> MResult<()> {
        if !self.accepts_mark(mark) {
            return self.invalid_mark("mark generation is stale or belongs to another context");
        }
        if mark.position.visible_start > mark.position.storage_len {
            return self.invalid_mark("mark visibility starts beyond its storage length");
        }
        if mark.position.storage_len > self.storage.len() {
            return self.invalid_mark("mark storage length exceeds current physical storage");
        }
        self.storage.truncate(mark.position.storage_len);
        self.visible_start = mark.position.visible_start;
        Ok(())
    }

    pub(crate) fn accepts_mark(&self, mark: &RuntimeContextEventMark) -> bool {
        mark.owner_id == self.owner_id && mark.generation == self.generation
    }

    pub(crate) fn retain_last(&mut self, max: usize) {
        self.visible_start = self.storage.len().saturating_sub(max);
        self.compact_after_active_marks();
    }

    /// Amortizes hidden-tail removal before another rollback mark is captured.
    ///
    /// Marks protect the complete physical prefix that existed when they were
    /// captured. Entries hidden after the newest live mark are transaction-era
    /// history and can be removed without changing any rollback position.
    #[cfg(any(test, feature = "source"))]
    pub(crate) fn prepare_checkpoint(&mut self) {
        self.compact_after_active_marks();
    }

    pub(crate) fn visible(&self) -> &[RuntimeEvent] {
        &self.storage[self.visible_start..]
    }

    pub(crate) fn visible_len(&self) -> usize {
        self.storage.len() - self.visible_start
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.visible_len() == 0
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, RuntimeEvent> {
        self.visible().iter()
    }

    pub(crate) fn clear(&mut self) {
        self.storage.clear();
        self.visible_start = 0;
        self.bump_local_generation();
    }

    #[cfg(test)]
    pub(crate) fn drain_visible(&mut self) -> Vec<RuntimeEvent> {
        let visible_start = self.visible_start;
        let mut storage = std::mem::take(&mut self.storage);
        if visible_start > 0 {
            storage.drain(..visible_start);
        }
        self.visible_start = 0;
        self.bump_local_generation();
        storage
    }

    /// Finalizes a transaction scope and amortizes removal of the hidden prefix.
    ///
    /// After a completed scope, physical storage remains below twice the visible
    /// retention plus the bounded events appended by the just-completed operation.
    /// Zero retention always leaves physical storage empty.
    pub(crate) fn finish_transaction_scope(&mut self) -> MResult<()> {
        if self.visible_start > self.storage.len() {
            return self.invalid_mark("visible storage boundary is invalid before compaction");
        }

        let hidden = self.visible_start;
        let visible = self.visible_len();
        if hidden > 0 && (visible == 0 || hidden >= visible) {
            #[cfg(any(test, feature = "runtime_bench_probes"))]
            let moved = visible;
            self.storage.drain(..hidden);
            self.visible_start = 0;
            #[cfg(any(test, feature = "runtime_bench_probes"))]
            crate::runtime::gate_a_probe::record_context_event_compaction(moved);
        }
        self.bump_local_generation();
        Ok(())
    }

    #[cfg(any(test, feature = "runtime_bench_probes"))]
    pub(crate) fn physical_len(&self) -> usize {
        self.storage.len()
    }

    fn bump_local_generation(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("runtime context event generation exhausted");
        self.active_marks
            .get_mut()
            .expect("runtime context event mark registry poisoned")
            .clear();
    }

    fn compact_after_active_marks(&mut self) {
        let Some(protected_len) = ({
            let mut marks = self
                .active_marks
                .lock()
                .expect("runtime context event mark registry poisoned");
            marks.retain(|mark| mark.strong_count() > 0);
            marks
                .iter()
                .filter_map(Weak::upgrade)
                .filter(|position| position.storage_len <= self.storage.len())
                .map(|position| position.storage_len)
                .max()
        }) else {
            return;
        };

        if self.visible_start <= protected_len {
            return;
        }

        let hidden_after_marks = self.visible_start - protected_len;
        let visible = self.visible_len();
        if visible > 0 && hidden_after_marks < visible {
            return;
        }

        #[cfg(any(test, feature = "runtime_bench_probes"))]
        let moved = self.storage.len() - self.visible_start;
        self.storage.drain(protected_len..self.visible_start);
        self.visible_start = protected_len;
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_context_event_compaction(moved);
    }

    #[cfg(feature = "runtime_bench_probes")]
    pub(crate) fn reserve_benchmark_append(&mut self) {
        self.storage.reserve(1);
    }

    fn invalid_mark<T>(&self, reason: impl Into<String>) -> MResult<T> {
        Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "RuntimeContextEvents::restore",
                reason: reason.into(),
            },
            None,
        ))
    }
}

impl Clone for RuntimeContextEvents {
    fn clone(&self) -> Self {
        Self {
            storage: self.visible().to_vec(),
            visible_start: 0,
            owner_id: next_owner_id(),
            generation: 0,
            active_marks: Mutex::new(Vec::new()),
        }
    }
}

impl Default for RuntimeContextEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for RuntimeContextEvents {
    fn eq(&self, other: &Self) -> bool {
        self.visible() == other.visible()
    }
}

impl Eq for RuntimeContextEvents {}

impl Deref for RuntimeContextEvents {
    type Target = [RuntimeEvent];

    fn deref(&self) -> &Self::Target {
        self.visible()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::gate_a_probe::{gate_a_cost_snapshot, reset_gate_a_costs};
    use crate::{EventId, RuntimeEventKind};

    fn event(id: u128) -> RuntimeEvent {
        RuntimeEvent::new(EventId(id), id as u64, RuntimeEventKind::RuntimeTickStarted)
    }

    #[test]
    fn context_event_empty_mark_and_restore() {
        let mut events = RuntimeContextEvents::new();
        let mark = events.mark();
        events.restore(&mark).unwrap();
        assert!(events.visible().is_empty());
    }

    #[test]
    fn context_event_append_then_restore() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        let mark = events.mark();
        events.push(event(2));
        events.restore(&mark).unwrap();
        assert_eq!(events.visible(), [event(1)]);
    }

    #[test]
    fn context_event_retention_then_restore_reveals_hidden_prefix() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        events.push(event(2));
        events.push(event(3));
        events.retain_last(3);
        let mark = events.mark();
        events.push(event(4));
        events.retain_last(3);
        events.push(event(5));
        events.retain_last(3);
        assert_eq!(events.visible(), [event(3), event(4), event(5)]);
        events.restore(&mark).unwrap();
        assert_eq!(events.visible(), [event(1), event(2), event(3)]);
    }

    #[test]
    fn context_event_zero_retention_hides_everything() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        events.retain_last(0);
        assert!(events.visible().is_empty());
        assert_eq!(events.physical_len(), 1);
    }

    #[test]
    fn hidden_prefix_smaller_than_visible_suffix_does_not_compact() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        events.push(event(2));
        events.push(event(3));
        events.push(event(4));
        events.retain_last(3);
        reset_gate_a_costs();
        events.finish_transaction_scope().unwrap();

        assert_eq!(events.visible(), [event(2), event(3), event(4)]);
        assert_eq!(events.physical_len(), 4);
        assert_eq!(gate_a_cost_snapshot().context_event_compaction_count, 0);
    }

    #[test]
    fn hidden_prefix_equal_to_visible_suffix_compacts() {
        let mut events = RuntimeContextEvents::new();
        for id in 1..=4 {
            events.push(event(id));
        }
        events.retain_last(2);
        reset_gate_a_costs();
        events.finish_transaction_scope().unwrap();

        assert_eq!(events.visible(), [event(3), event(4)]);
        assert_eq!(events.physical_len(), 2);
        let costs = gate_a_cost_snapshot();
        assert_eq!(costs.context_event_compaction_count, 1);
        assert_eq!(costs.context_event_compaction_moved_items, 2);
    }

    #[test]
    fn compaction_counts_only_the_moved_visible_suffix() {
        let mut events = RuntimeContextEvents::new();
        for id in 1..=5 {
            events.push(event(id));
        }
        events.retain_last(2);
        reset_gate_a_costs();
        events.finish_transaction_scope().unwrap();

        assert_eq!(events.visible(), [event(4), event(5)]);
        let costs = gate_a_cost_snapshot();
        assert_eq!(costs.context_event_compaction_count, 1);
        assert_eq!(costs.context_event_compaction_moved_items, 2);
    }

    #[test]
    fn zero_retention_compacts_to_zero_physical_entries() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        events.retain_last(0);
        reset_gate_a_costs();
        events.finish_transaction_scope().unwrap();

        assert!(events.visible().is_empty());
        assert_eq!(events.physical_len(), 0);
        let costs = gate_a_cost_snapshot();
        assert_eq!(costs.context_event_compaction_count, 1);
        assert_eq!(costs.context_event_compaction_moved_items, 0);
    }

    #[test]
    fn scope_completion_invalidates_marks_without_changing_owner() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        let mark = events.mark();
        events.finish_transaction_scope().unwrap();

        assert_eq!(events.owner_id, mark.owner_id);
        assert_eq!(events.generation, mark.generation + 1);
        let error = events.restore(&mark).unwrap_err();
        assert_eq!(error.kind_name(), "RuntimeInvalidOperation");
    }

    #[test]
    fn active_mark_bounds_hidden_tail_without_losing_rollback_prefix() {
        let mut events = RuntimeContextEvents::new();
        for id in 1..=3 {
            events.push(event(id));
        }
        let baseline = events.visible().to_vec();
        let mark = events.mark();

        for id in 4..=1_003 {
            events.push(event(id));
            events.retain_last(3);
            assert!(events.physical_len() <= mark.position.storage_len + 6);
        }

        events.restore(&mark).unwrap();
        assert_eq!(events.visible(), baseline);
    }

    #[test]
    fn newest_mark_protects_operation_baseline_while_later_events_compact() {
        let mut events = RuntimeContextEvents::new();
        for id in 1..=3 {
            events.push(event(id));
        }
        let _transaction_mark = events.mark();
        for id in 4..=9 {
            events.push(event(id));
            events.retain_last(3);
        }
        events.prepare_checkpoint();
        let operation_mark = events.mark();
        let operation_baseline = events.visible().to_vec();

        for id in 10..=1_009 {
            events.push(event(id));
            events.retain_last(3);
            assert!(events.physical_len() <= operation_mark.position.storage_len + 6);
        }

        events.restore(&operation_mark).unwrap();
        assert_eq!(events.visible(), operation_baseline);
    }

    #[test]
    fn context_event_clone_contains_only_visible_events_and_has_a_distinct_owner() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        events.push(event(2));
        events.retain_last(1);
        let mark = events.mark();
        let mut cloned = events.clone();
        assert_eq!(cloned.visible(), [event(2)]);
        assert_ne!(cloned.owner_id, events.owner_id);
        assert!(cloned.restore(&mark).is_err());
        assert_eq!(cloned.physical_len(), 1);
    }

    #[test]
    fn mark_from_another_context_is_rejected() {
        let events = RuntimeContextEvents::new();
        let mark = events.mark();
        let mut other = RuntimeContextEvents::new();
        assert!(other.restore(&mark).is_err());
    }

    #[test]
    fn context_event_drain_transfers_visible_events_and_clears_storage() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        events.push(event(2));
        events.retain_last(1);
        assert_eq!(events.drain_visible(), [event(2)]);
        assert!(events.visible().is_empty());
        assert_eq!(events.physical_len(), 0);
    }

    #[test]
    fn context_event_clear_resets_visibility() {
        let mut events = RuntimeContextEvents::new();
        events.push(event(1));
        events.retain_last(0);
        events.clear();
        events.push(event(2));
        assert_eq!(events.visible(), [event(2)]);
        assert_eq!(events.physical_len(), 1);
    }
}
