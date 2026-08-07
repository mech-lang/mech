use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeSet;
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mech_runtime::__gate_a_recording::{
    AccountedRecord, OutboxDeliveryPolicy, OutboxEffectId, OwnedEffectIntent, OwnedTurnRecordQueue,
    RecordBufferPool, RecordEstimate, RetainedEffectOutbox, RetainedTurnLedger, TurnId,
    prepare_outbox, prepare_queue, prepare_retained, reserve_outbox, reserve_queue,
    reserve_retained,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static REPORTED: OnceLock<Mutex<BTreeSet<(String, usize)>>> = OnceLock::new();

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, old: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(size as u64, Ordering::Relaxed);
        unsafe { System.realloc(pointer, old, size) }
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

#[derive(Clone, Copy)]
struct PhaseSample {
    reserve: Duration,
    prepare: Duration,
    append: Duration,
    bytes: usize,
    pool_reuse: bool,
    allocations: AllocationSnapshot,
}

fn report(operation: &str, history: usize, sample: PhaseSample) {
    let mut reported = REPORTED
        .get_or_init(|| Mutex::new(BTreeSet::new()))
        .lock()
        .unwrap();
    if !reported.insert((operation.to_string(), history)) {
        return;
    }
    eprintln!(
        "GATE_A_SAMPLE {{\"operation\":\"{operation}\",\"history\":{history},\"reserve_time_ns\":{},\"prepare_time_ns\":{},\"append_time_ns\":{},\"accounted_record_bytes\":{},\"pool_reuse\":{},\"allocation_count\":{},\"deallocation_count\":{},\"allocated_bytes\":{},\"post_publication_failure_branch\":false}}",
        sample.reserve.as_nanos(),
        sample.prepare.as_nanos(),
        sample.append.as_nanos(),
        sample.bytes,
        sample.pool_reuse,
        sample.allocations.allocations,
        sample.allocations.deallocations,
        sample.allocations.allocated_bytes,
    );
}

fn scaled_append(sample: PhaseSample, iterations: u64) -> Duration {
    sample
        .append
        .max(Duration::from_nanos(1))
        .mul_f64(iterations as f64)
}

fn boxed_record() -> Box<[u8]> {
    vec![0_u8; 32].into_boxed_slice()
}

fn append_retained(ledger: &mut RetainedTurnLedger<Box<[u8]>>) {
    let record = boxed_record();
    let permit = reserve_retained(
        ledger,
        RecordEstimate {
            records: 1,
            bytes: record.retained_bytes(),
        },
    )
    .unwrap();
    prepare_retained(ledger, permit, record).unwrap().append();
}

fn retained_fixture(history: usize) -> RetainedTurnLedger<Box<[u8]>> {
    let capacity = history + 1;
    let mut ledger = RetainedTurnLedger::new(capacity, capacity * 32).unwrap();
    for _ in 0..history {
        append_retained(&mut ledger);
    }
    ledger
}

fn retained_ledger_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a_recording/retained_ledger");
    for history in [0_usize, 1_000, 100_000] {
        group.bench_function(BenchmarkId::from_parameter(history), |b| {
            b.iter_custom(|iterations| {
                let mut ledger = retained_fixture(history);
                let record = boxed_record();
                let bytes = record.retained_bytes();
                let started = Instant::now();
                let permit =
                    reserve_retained(&ledger, RecordEstimate { records: 1, bytes }).unwrap();
                let reserve = started.elapsed();
                let started = Instant::now();
                let prepared = prepare_retained(&mut ledger, permit, record).unwrap();
                let prepare = started.elapsed();
                reset_allocations();
                let started = Instant::now();
                black_box(prepared.append());
                let append = started.elapsed();
                let sample = PhaseSample {
                    reserve,
                    prepare,
                    append,
                    bytes,
                    pool_reuse: false,
                    allocations: allocation_snapshot(),
                };
                report("recording_retained_ledger", history, sample);
                scaled_append(sample, iterations)
            });
        });
    }
    group.finish();
}

fn append_queued(queue: &OwnedTurnRecordQueue<Box<[u8]>>) {
    let record = boxed_record();
    let permit = reserve_queue(
        queue,
        RecordEstimate {
            records: 1,
            bytes: record.retained_bytes(),
        },
    )
    .unwrap();
    prepare_queue(queue, permit, record).unwrap().append();
}

fn queue_fixture(history: usize) -> OwnedTurnRecordQueue<Box<[u8]>> {
    let capacity = history + 1;
    let queue = OwnedTurnRecordQueue::new(capacity, capacity * 32).unwrap();
    for _ in 0..history {
        append_queued(&queue);
    }
    queue
}

fn owned_queue_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a_recording/owned_queue");
    for history in [0_usize, 1_000, 100_000] {
        group.bench_function(BenchmarkId::from_parameter(history), |b| {
            b.iter_custom(|iterations| {
                let queue = queue_fixture(history);
                let record = boxed_record();
                let bytes = record.retained_bytes();
                let started = Instant::now();
                let permit = reserve_queue(&queue, RecordEstimate { records: 1, bytes }).unwrap();
                let reserve = started.elapsed();
                let started = Instant::now();
                let prepared = prepare_queue(&queue, permit, record).unwrap();
                let prepare = started.elapsed();
                reset_allocations();
                let started = Instant::now();
                black_box(prepared.append());
                let append = started.elapsed();
                let sample = PhaseSample {
                    reserve,
                    prepare,
                    append,
                    bytes,
                    pool_reuse: false,
                    allocations: allocation_snapshot(),
                };
                report("recording_owned_queue", history, sample);
                scaled_append(sample, iterations)
            });
        });
    }
    group.finish();
}

fn pool_fixture(history: usize) -> RecordBufferPool {
    let pool = RecordBufferPool::new(1, 64, 64).unwrap();
    for _ in 0..history {
        let mut segment = pool.acquire(64).unwrap();
        segment.try_extend_from_slice(&[0_u8; 32]).unwrap();
        drop(segment);
    }
    pool
}

fn record_pool_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a_recording/record_pool");
    for history in [0_usize, 1_000, 100_000] {
        group.bench_function(BenchmarkId::from_parameter(history), |b| {
            b.iter_custom(|iterations| {
                let pool = pool_fixture(history);
                let allocations_before = pool.stats().allocations;
                let started = Instant::now();
                let mut segment = pool.acquire(64).unwrap();
                let reserve = started.elapsed();
                let started = Instant::now();
                segment.try_extend_from_slice(&[0_u8; 32]).unwrap();
                let prepare = started.elapsed();
                let bytes = segment.retained_bytes();
                reset_allocations();
                let started = Instant::now();
                drop(segment);
                let append = started.elapsed();
                let sample = PhaseSample {
                    reserve,
                    prepare,
                    append,
                    bytes,
                    pool_reuse: pool.stats().allocations == allocations_before,
                    allocations: allocation_snapshot(),
                };
                report("recording_record_pool", history, sample);
                scaled_append(sample, iterations)
            });
        });
    }
    group.finish();
}

fn effect(turn: u64) -> OwnedEffectIntent<Box<[u8]>> {
    OwnedEffectIntent {
        id: OutboxEffectId {
            turn_id: TurnId::new(turn).unwrap(),
            ordinal: 0,
        },
        operation: String::from("write"),
        target: String::from("bench"),
        payload: vec![0_u8; 16].into_boxed_slice(),
        idempotency_key: turn.to_string(),
        delivery: OutboxDeliveryPolicy::AtLeastOnce,
    }
}

fn append_effect(outbox: &mut RetainedEffectOutbox<Box<[u8]>>, turn: u64) {
    let effect = effect(turn);
    let bytes = effect.retained_bytes();
    let permit = reserve_outbox(outbox, RecordEstimate { records: 1, bytes }).unwrap();
    prepare_outbox(outbox, permit, vec![effect])
        .unwrap()
        .append();
}

fn outbox_fixture(history: usize) -> RetainedEffectOutbox<Box<[u8]>> {
    let capacity = history + 1;
    let mut outbox = RetainedEffectOutbox::new(capacity, capacity * 64).unwrap();
    for turn in 1..=history {
        append_effect(&mut outbox, turn as u64);
    }
    outbox
}

fn effect_outbox_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_a_recording/effect_outbox");
    for history in [0_usize, 1_000, 100_000] {
        group.bench_function(BenchmarkId::from_parameter(history), |b| {
            b.iter_custom(|iterations| {
                let mut outbox = outbox_fixture(history);
                let effect = effect(history as u64 + 1);
                let bytes = effect.retained_bytes();
                let started = Instant::now();
                let permit = reserve_outbox(&outbox, RecordEstimate { records: 1, bytes }).unwrap();
                let reserve = started.elapsed();
                let started = Instant::now();
                let prepared = prepare_outbox(&mut outbox, permit, vec![effect]).unwrap();
                let prepare = started.elapsed();
                reset_allocations();
                let started = Instant::now();
                prepared.append();
                let append = started.elapsed();
                let sample = PhaseSample {
                    reserve,
                    prepare,
                    append,
                    bytes,
                    pool_reuse: false,
                    allocations: allocation_snapshot(),
                };
                report("recording_effect_outbox", history, sample);
                scaled_append(sample, iterations)
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    retained_ledger_benchmark,
    owned_queue_benchmark,
    record_pool_benchmark,
    effect_outbox_benchmark,
);
criterion_main!(benches);
