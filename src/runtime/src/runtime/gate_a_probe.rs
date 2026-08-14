//! Deterministic counters for the Gate A runtime cost baseline.
//!
//! These counters are intentionally attached to coordinator sites rather than
//! general `Clone` implementations. They are absent from ordinary builds.

use std::cell::RefCell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GateACostSnapshot {
    pub context_event_snapshot_count: u64,
    pub context_event_snapshot_items: u64,
    pub context_event_compaction_count: u64,
    pub context_event_compaction_moved_items: u64,
    pub context_event_visible_len: u64,
    pub context_event_physical_len: u64,
    pub runtime_transaction_savepoint_clone_count: u64,
    pub runtime_transaction_savepoint_items: u64,
    pub in_memory_store_clone_count: u64,
    pub in_memory_store_cloned_records: u64,
    pub in_memory_store_prepare_duration_ns: u64,
    pub in_memory_store_apply_duration_ns: u64,
    pub commit_runtime_call_count: u64,
    pub program_checkpoint_count: u64,
    pub reactive_journal_cell_count: u64,
    pub events_appended: u64,
    pub transactions_committed: u64,
}

thread_local! {
    static GATE_A_COSTS: RefCell<GateACostSnapshot> =
        RefCell::new(GateACostSnapshot::default());
}

fn add(field: impl FnOnce(&mut GateACostSnapshot)) {
    GATE_A_COSTS.with(|costs| field(&mut costs.borrow_mut()));
}

pub fn reset_gate_a_costs() {
    GATE_A_COSTS.with(|costs| *costs.borrow_mut() = GateACostSnapshot::default());
}

pub fn gate_a_cost_snapshot() -> GateACostSnapshot {
    GATE_A_COSTS.with(|costs| *costs.borrow())
}

pub(crate) fn record_context_event_snapshot(items: usize) {
    add(|costs| {
        costs.context_event_snapshot_count = costs.context_event_snapshot_count.saturating_add(1);
        costs.context_event_snapshot_items = costs
            .context_event_snapshot_items
            .saturating_add(items as u64);
    });
}

pub(crate) fn record_context_event_compaction(moved_items: usize) {
    add(|costs| {
        costs.context_event_compaction_count =
            costs.context_event_compaction_count.saturating_add(1);
        costs.context_event_compaction_moved_items = costs
            .context_event_compaction_moved_items
            .saturating_add(moved_items as u64);
    });
}

pub(crate) fn record_context_event_lengths(visible: usize, physical: usize) {
    add(|costs| {
        costs.context_event_visible_len = visible as u64;
        costs.context_event_physical_len = physical as u64;
    });
}

pub(crate) fn record_runtime_transaction_savepoint_clone(items: usize) {
    add(|costs| {
        costs.runtime_transaction_savepoint_clone_count = costs
            .runtime_transaction_savepoint_clone_count
            .saturating_add(1);
        costs.runtime_transaction_savepoint_items = costs
            .runtime_transaction_savepoint_items
            .saturating_add(items as u64);
    });
}

pub(crate) fn record_in_memory_store_clone(records: usize) {
    add(|costs| {
        costs.in_memory_store_clone_count = costs.in_memory_store_clone_count.saturating_add(1);
        costs.in_memory_store_cloned_records = costs
            .in_memory_store_cloned_records
            .saturating_add(records as u64);
    });
}

pub(crate) fn record_in_memory_store_prepare_duration(duration: std::time::Duration) {
    let elapsed = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
    add(|costs| {
        costs.in_memory_store_prepare_duration_ns = costs
            .in_memory_store_prepare_duration_ns
            .saturating_add(elapsed);
    });
}

pub(crate) fn record_in_memory_store_apply_duration(duration: std::time::Duration) {
    let elapsed = u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX);
    add(|costs| {
        costs.in_memory_store_apply_duration_ns = costs
            .in_memory_store_apply_duration_ns
            .saturating_add(elapsed);
    });
}

pub(crate) fn record_commit_runtime_call() {
    add(|costs| {
        costs.commit_runtime_call_count = costs.commit_runtime_call_count.saturating_add(1);
    });
}

pub(crate) fn record_program_checkpoint() {
    add(|costs| {
        costs.program_checkpoint_count = costs.program_checkpoint_count.saturating_add(1);
    });
}

pub(crate) fn record_reactive_journal_cells(cells: usize) {
    add(|costs| {
        costs.reactive_journal_cell_count = costs
            .reactive_journal_cell_count
            .saturating_add(cells as u64);
    });
}

pub(crate) fn record_event_appended() {
    add(|costs| {
        costs.events_appended = costs.events_appended.saturating_add(1);
    });
}

pub(crate) fn record_transaction_committed() {
    add(|costs| {
        costs.transactions_committed = costs.transactions_committed.saturating_add(1);
    });
}

#[cfg(test)]
mod tests {
    use super::{
        GateACostSnapshot, gate_a_cost_snapshot, record_commit_runtime_call,
        record_context_event_compaction, record_context_event_lengths,
        record_context_event_snapshot, record_event_appended,
        record_in_memory_store_apply_duration, record_in_memory_store_clone,
        record_in_memory_store_prepare_duration, record_program_checkpoint,
        record_reactive_journal_cells, record_runtime_transaction_savepoint_clone,
        record_transaction_committed, reset_gate_a_costs,
    };

    #[test]
    fn counters_are_thread_local_and_resettable() {
        reset_gate_a_costs();
        record_context_event_snapshot(3);
        record_context_event_compaction(2);
        record_context_event_lengths(5, 7);
        record_runtime_transaction_savepoint_clone(5);
        record_in_memory_store_clone(7);
        record_in_memory_store_prepare_duration(std::time::Duration::from_nanos(13));
        record_in_memory_store_apply_duration(std::time::Duration::from_nanos(17));
        record_commit_runtime_call();
        record_program_checkpoint();
        record_reactive_journal_cells(11);
        record_event_appended();
        record_transaction_committed();

        assert_eq!(
            gate_a_cost_snapshot(),
            GateACostSnapshot {
                context_event_snapshot_count: 1,
                context_event_snapshot_items: 3,
                context_event_compaction_count: 1,
                context_event_compaction_moved_items: 2,
                context_event_visible_len: 5,
                context_event_physical_len: 7,
                runtime_transaction_savepoint_clone_count: 1,
                runtime_transaction_savepoint_items: 5,
                in_memory_store_clone_count: 1,
                in_memory_store_cloned_records: 7,
                in_memory_store_prepare_duration_ns: 13,
                in_memory_store_apply_duration_ns: 17,
                commit_runtime_call_count: 1,
                program_checkpoint_count: 1,
                reactive_journal_cell_count: 11,
                events_appended: 1,
                transactions_committed: 1,
            },
        );

        reset_gate_a_costs();
        assert_eq!(gate_a_cost_snapshot(), GateACostSnapshot::default());
    }

    #[test]
    fn counters_do_not_cross_threads() {
        reset_gate_a_costs();
        record_event_appended();
        let other = std::thread::spawn(gate_a_cost_snapshot).join().unwrap();
        assert_eq!(other, GateACostSnapshot::default());
        assert_eq!(gate_a_cost_snapshot().events_appended, 1);
    }
}
