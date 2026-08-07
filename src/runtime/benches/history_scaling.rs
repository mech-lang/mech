use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeSet;
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mech_runtime::{GateACostSnapshot, MechStore, gate_a_cost_snapshot, reset_gate_a_costs};

mod support;
use support::history::{
    HistoryTurnFixture, minimal_commit, mixed_store_and_commit, retained_store,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static REPORTED_SAMPLES: OnceLock<Mutex<BTreeSet<(String, usize)>>> = OnceLock::new();

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, old, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy, Debug, Default)]
struct AllocationSnapshot {
    allocations: u64,
    deallocations: u64,
    allocated_bytes: u64,
}

fn reset_allocations() {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    DEALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
}

fn allocation_snapshot() -> AllocationSnapshot {
    AllocationSnapshot {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
        allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
    }
}

fn report(
    operation: &str,
    history: usize,
    probes: GateACostSnapshot,
    allocations: AllocationSnapshot,
) {
    let mut reported = REPORTED_SAMPLES
        .get_or_init(|| Mutex::new(BTreeSet::new()))
        .lock()
        .unwrap();
    if !reported.insert((operation.to_string(), history)) {
        return;
    }
    drop(reported);
    emit_report(operation, history, probes, allocations);
}

fn report_latest(
    operation: &str,
    history: usize,
    probes: GateACostSnapshot,
    allocations: AllocationSnapshot,
) {
    emit_report(operation, history, probes, allocations);
}

fn emit_report(
    operation: &str,
    history: usize,
    probes: GateACostSnapshot,
    allocations: AllocationSnapshot,
) {
    eprintln!(
        "GATE_A_SAMPLE {{\"operation\":\"{operation}\",\"history\":{history},\"allocation_count\":{},\"deallocation_count\":{},\"allocated_bytes\":{},\"context_event_snapshot_count\":{},\"context_event_snapshot_items\":{},\"context_event_compaction_count\":{},\"context_event_compaction_moved_items\":{},\"context_event_visible_len\":{},\"context_event_physical_len\":{},\"runtime_transaction_savepoint_clone_count\":{},\"runtime_transaction_savepoint_items\":{},\"in_memory_store_clone_count\":{},\"in_memory_store_cloned_records\":{},\"commit_runtime_call_count\":{},\"program_checkpoint_count\":{},\"reactive_journal_cell_count\":{},\"events_appended\":{},\"transactions_committed\":{}}}",
        allocations.allocations,
        allocations.deallocations,
        allocations.allocated_bytes,
        probes.context_event_snapshot_count,
        probes.context_event_snapshot_items,
        probes.context_event_compaction_count,
        probes.context_event_compaction_moved_items,
        probes.context_event_visible_len,
        probes.context_event_physical_len,
        probes.runtime_transaction_savepoint_clone_count,
        probes.runtime_transaction_savepoint_items,
        probes.in_memory_store_clone_count,
        probes.in_memory_store_cloned_records,
        probes.commit_runtime_call_count,
        probes.program_checkpoint_count,
        probes.reactive_journal_cell_count,
        probes.events_appended,
        probes.transactions_committed,
    );
}

fn with_context_event_lengths(
    mut probes: GateACostSnapshot,
    visible: usize,
    physical: usize,
) -> GateACostSnapshot {
    probes.context_event_visible_len = visible as u64;
    probes.context_event_physical_len = physical as u64;
    probes
}

fn direct_store_histories() -> Vec<usize> {
    let mut histories = vec![0, 1_000, 10_000, 100_000];
    if std::env::var_os("MECH_GATE_A_EXTENDED").is_some() {
        histories.push(1_000_000);
    }
    histories
}

fn scale_single_operation(elapsed: Duration, iterations: u64) -> Duration {
    elapsed.mul_f64(iterations as f64)
}

fn scale_measured_operations(
    elapsed: Duration,
    requested_iterations: u64,
    measured_operations: u64,
) -> Duration {
    elapsed.mul_f64(requested_iterations as f64 / measured_operations as f64)
}

fn full_turn_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a/full_turn_history");
    for history in [0usize, 32, 1_024, 16_384] {
        group.bench_function(BenchmarkId::from_parameter(history), |b| {
            b.iter_custom(|iterations| {
                let mut fixture = HistoryTurnFixture::with_accepted_turns(history);
                reset_gate_a_costs();
                reset_allocations();
                let started = Instant::now();
                fixture.accept_turn();
                let elapsed = started.elapsed();
                let allocations = allocation_snapshot();
                let (visible, physical) = fixture.context_event_lengths();
                let probes = with_context_event_lengths(gate_a_cost_snapshot(), visible, physical);
                black_box(&fixture.context);
                report("full_turn", history, probes, allocations);
                scale_single_operation(elapsed, iterations)
            });
        });
    }
    group.finish();
}

fn direct_event_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a/direct_event_history");
    for history in [0usize, 32, 1_024, 16_384] {
        group.bench_function(BenchmarkId::from_parameter(history), |b| {
            b.iter_custom(|iterations| {
                let mut fixture = HistoryTurnFixture::new();
                fixture.populate_context_events(history);
                reset_gate_a_costs();
                reset_allocations();
                let started = Instant::now();
                black_box(
                    fixture
                        .runtime
                        .gate_a_emit_representative_event(&mut fixture.context)
                        .unwrap(),
                );
                let elapsed = started.elapsed();
                let allocations = allocation_snapshot();
                let (visible, physical) = fixture.context_event_lengths();
                let probes = with_context_event_lengths(gate_a_cost_snapshot(), visible, physical);
                report("direct_event", history, probes, allocations);
                scale_single_operation(elapsed, iterations)
            });
        });
    }
    group.finish();
}

fn direct_store_history(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a/direct_store_history");
    for history in direct_store_histories() {
        group.bench_function(BenchmarkId::from_parameter(history), |b| {
            b.iter_custom(|iterations| {
                let mut store = retained_store(history);
                let seed = 1_000_000u128 + history as u128 * 10;
                reset_gate_a_costs();
                reset_allocations();
                let started = Instant::now();
                black_box(store.commit_runtime(minimal_commit(seed)).unwrap());
                let elapsed = started.elapsed();
                let allocations = allocation_snapshot();
                let probes = gate_a_cost_snapshot();
                report("direct_store", history, probes, allocations);
                scale_single_operation(elapsed, iterations)
            });
        });
    }

    group.bench_function("mixed_all_families", |b| {
        let (mut store, commit) = mixed_store_and_commit(8_000_000);
        b.iter_batched(
            || (store.clone(), commit.clone()),
            |(mut iteration_store, iteration_commit)| {
                black_box(iteration_store.commit_runtime(iteration_commit).unwrap())
            },
            criterion::BatchSize::SmallInput,
        );
        black_box(&mut store);
    });
    group.finish();
}

fn explicit_savepoint(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a/explicit_savepoint");
    for operations in [0usize, 100, 1_000] {
        group.bench_function(BenchmarkId::from_parameter(operations), |b| {
            b.iter_custom(|iterations| {
                let mut fixture = HistoryTurnFixture::new();
                fixture.begin_and_stage_objects(operations);
                reset_gate_a_costs();
                reset_allocations();
                let started = Instant::now();
                fixture
                    .runtime
                    .gate_a_capture_runtime_operation_savepoint(&mut fixture.context)
                    .unwrap();
                let elapsed = started.elapsed();
                let allocations = allocation_snapshot();
                let (visible, physical) = fixture.context_event_lengths();
                let probes = with_context_event_lengths(gate_a_cost_snapshot(), visible, physical);
                report("explicit_savepoint", operations, probes, allocations);
                scale_single_operation(elapsed, iterations)
            });
        });
    }
    group.finish();
}

fn context_event_retention_steady(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a/context_event_retention_steady");
    for limit in [32usize, 1_024, 16_384, 100_000] {
        group.bench_function(BenchmarkId::from_parameter(limit), |b| {
            b.iter_custom(|iterations| {
                let mut fixture = HistoryTurnFixture::with_event_retention(limit);
                fixture.populate_context_events(limit);
                fixture.warm_context_event_retention(limit);
                assert_eq!(fixture.context_event_lengths(), (limit, limit));
                let measured_operations = iterations.max(limit as u64);

                reset_gate_a_costs();
                reset_allocations();
                let started = Instant::now();
                for _ in 0..measured_operations {
                    black_box(
                        fixture
                            .runtime
                            .gate_a_emit_representative_event(&mut fixture.context)
                            .unwrap(),
                    );
                }
                let elapsed = started.elapsed();
                let allocations = allocation_snapshot();
                let (visible, physical) = fixture.context_event_lengths();
                let probes = with_context_event_lengths(gate_a_cost_snapshot(), visible, physical);

                assert_eq!(probes.context_event_snapshot_count, 0);
                assert_eq!(probes.context_event_snapshot_items, 0);
                assert_eq!(probes.events_appended, measured_operations);
                assert_eq!(visible, limit);
                assert!(physical < 2 * limit);
                assert!(probes.context_event_compaction_count >= 1);
                assert!(probes.context_event_compaction_moved_items <= probes.events_appended);
                assert!(probes.context_event_compaction_count < probes.events_appended);

                report_latest("context_event_retention_steady", limit, probes, allocations);
                scale_measured_operations(elapsed, iterations, measured_operations)
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    full_turn_history,
    direct_event_history,
    direct_store_history,
    explicit_savepoint,
    context_event_retention_steady,
);
criterion_main!(benches);
