use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeSet;
use std::hint::black_box;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
#[cfg(feature = "runtime_bench_probes")]
use mech_runtime::{gate_a_cost_snapshot, reset_gate_a_costs};

mod support;
use support::gate_b::contract::{
    EPISODE_LENGTH, EkfState, REFERENCE_TRAJECTORY_SHA256, SCALED_INSTANCES, TRACE_SHA256,
    assert_state_close, reference_trajectory, trace_sha256,
};
use support::gate_b::full_write::{FullWriteEpochFixture, FullWriteProbe, buffer_hash};
use support::gate_b::legacy_atomic::{LegacyEkfFixture, LegacyFullWriteFixture};
use support::gate_b::raw_epoch::{EpochFixture, EpochProbe};
use support::gate_b::raw_kernel::KernelFixture;
use support::gate_b::resident_artifact::{
    ArtifactRoute, ResidentArtifactFixture, ResidentArtifactKernelFixture, ResidentArtifactProbe,
};
use support::gate_b::resident_kernel::{
    ResidentFullWriteFixture, ResidentKernelFixture, ResidentKernelProbe,
};
use support::gate_b::resident_turn::{
    ResidentCompleteProbe, ResidentFullWriteTurnFixture, ResidentScheduledFixture,
    ResidentTurnFixture,
};

struct CountingAllocator;

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static REPORTED: OnceLock<Mutex<BTreeSet<(String, usize, usize, u64)>>> = OnceLock::new();

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

#[derive(Clone, Copy, Debug, Default)]
struct StructuralProbe {
    candidate_seed_bytes: usize,
    candidate_written_bytes: usize,
    published_buffer_copy_bytes: usize,
    publication_store_count: usize,
    receipt_bytes: usize,
    commit_runtime_call_count: u64,
    legacy_journal_capture_count: u64,
    dirty_node_count: usize,
    record_preparation_count: usize,
    record_append_count: usize,
    records_retained_before_timing: usize,
    records_appended: usize,
    ledger_records_inspected: usize,
    post_publication_append_infallible: bool,
}

impl From<EpochProbe> for StructuralProbe {
    fn from(probe: EpochProbe) -> Self {
        Self {
            candidate_seed_bytes: probe.candidate_seed_bytes,
            candidate_written_bytes: probe.candidate_written_bytes,
            published_buffer_copy_bytes: probe.published_buffer_copy_bytes,
            publication_store_count: probe.publication_store_count,
            receipt_bytes: probe.receipt_bytes,
            record_preparation_count: probe.record_preparation_count,
            record_append_count: probe.record_append_count,
            records_appended: probe.records_appended,
            ledger_records_inspected: probe.ledger_records_inspected,
            post_publication_append_infallible: probe.post_publication_append_infallible,
            ..Self::default()
        }
    }
}

impl From<FullWriteProbe> for StructuralProbe {
    fn from(probe: FullWriteProbe) -> Self {
        Self {
            candidate_seed_bytes: probe.candidate_seed_bytes,
            candidate_written_bytes: probe.candidate_written_bytes,
            published_buffer_copy_bytes: probe.published_buffer_copy_bytes,
            publication_store_count: probe.publication_store_count,
            receipt_bytes: probe.receipt_bytes,
            record_preparation_count: probe.record_preparation_count,
            record_append_count: probe.record_append_count,
            records_appended: probe.records_appended,
            ledger_records_inspected: probe.ledger_records_inspected,
            post_publication_append_infallible: probe.post_publication_append_infallible,
            ..Self::default()
        }
    }
}

impl From<ResidentKernelProbe> for StructuralProbe {
    fn from(probe: ResidentKernelProbe) -> Self {
        Self {
            candidate_seed_bytes: probe.candidate_seed_bytes,
            candidate_written_bytes: probe.candidate_written_bytes,
            published_buffer_copy_bytes: probe.published_buffer_copy_bytes,
            publication_store_count: probe.publication_store_count,
            ..Self::default()
        }
    }
}

impl From<ResidentCompleteProbe> for StructuralProbe {
    fn from(probe: ResidentCompleteProbe) -> Self {
        Self {
            candidate_seed_bytes: probe.candidate_seed_bytes,
            candidate_written_bytes: probe.candidate_written_bytes,
            published_buffer_copy_bytes: probe.published_buffer_copy_bytes,
            publication_store_count: probe.publication_store_count,
            receipt_bytes: probe.receipt_bytes,
            dirty_node_count: probe.dirty_nodes,
            record_preparation_count: probe.record_preparation_count,
            record_append_count: probe.record_append_count,
            records_retained_before_timing: probe.records_retained_before_timing,
            records_appended: probe.records_appended,
            ledger_records_inspected: probe.ledger_records_inspected,
            post_publication_append_infallible: true,
            ..Self::default()
        }
    }
}

impl From<ResidentArtifactProbe> for StructuralProbe {
    fn from(probe: ResidentArtifactProbe) -> Self {
        Self {
            candidate_seed_bytes: probe.candidate_seed_bytes,
            candidate_written_bytes: probe.candidate_written_bytes,
            published_buffer_copy_bytes: probe.published_buffer_copy_bytes,
            publication_store_count: probe.publication_store_count,
            receipt_bytes: probe.receipt_bytes,
            commit_runtime_call_count: probe.commit_runtime_call_count as u64,
            legacy_journal_capture_count: probe.legacy_journal_capture_count as u64,
            dirty_node_count: probe.dirty_nodes,
            record_preparation_count: probe.record_preparation_count,
            record_append_count: probe.record_append_count,
            records_retained_before_timing: probe.records_retained_before_timing,
            records_appended: probe.records_appended,
            ledger_records_inspected: probe.ledger_records_inspected,
            post_publication_append_infallible: probe.post_publication_append_infallible,
        }
    }
}

fn report(
    lane: &str,
    instances: usize,
    allocations: AllocationSnapshot,
    probe: StructuralProbe,
    output_hash: &str,
    abort_output_hash: Option<&str>,
) {
    report_dimensions(
        lane,
        instances,
        0,
        1,
        allocations,
        probe,
        output_hash,
        abort_output_hash,
    );
}

#[allow(clippy::too_many_arguments)]
fn report_dimensions(
    lane: &str,
    instances: usize,
    retained_history: usize,
    next_epoch: u64,
    allocations: AllocationSnapshot,
    probe: StructuralProbe,
    output_hash: &str,
    abort_output_hash: Option<&str>,
) {
    let mut reported = REPORTED
        .get_or_init(|| Mutex::new(BTreeSet::new()))
        .lock()
        .expect("Gate B report lock");
    if !reported.insert((lane.to_string(), instances, retained_history, next_epoch)) {
        return;
    }
    drop(reported);
    let abort = abort_output_hash
        .map(|hash| format!("\"{hash}\""))
        .unwrap_or_else(|| "null".to_string());
    let line = format!(
        "GATE_B_SAMPLE {{\"lane\":\"{lane}\",\"instances\":{instances},\"turns\":{EPISODE_LENGTH},\"retained_history\":{retained_history},\"next_epoch\":{next_epoch},\"allocation_count\":{},\"deallocation_count\":{},\"allocated_bytes\":{},\"correctness\":true,\"quantized_state_hash\":\"{output_hash}\",\"candidate_seed_bytes\":{},\"candidate_written_bytes\":{},\"published_buffer_copy_bytes\":{},\"publication_store_count\":{},\"receipt_bytes\":{},\"commit_runtime_call_count\":{},\"legacy_journal_capture_count\":{},\"dirty_node_count\":{},\"record_preparation_count\":{},\"record_append_count\":{},\"records_retained_before_timing\":{},\"records_appended\":{},\"ledger_records_inspected\":{},\"post_publication_append_infallible\":{},\"abort_output_hash\":{abort}}}",
        allocations.allocations,
        allocations.deallocations,
        allocations.allocated_bytes,
        probe.candidate_seed_bytes,
        probe.candidate_written_bytes,
        probe.published_buffer_copy_bytes,
        probe.publication_store_count,
        probe.receipt_bytes,
        probe.commit_runtime_call_count,
        probe.legacy_journal_capture_count,
        probe.dirty_node_count,
        probe.record_preparation_count,
        probe.record_append_count,
        probe.records_retained_before_timing,
        probe.records_appended,
        probe.ledger_records_inspected,
        probe.post_publication_append_infallible,
    );
    let mut stderr = std::io::stderr().lock();
    stderr
        .write_all(line.as_bytes())
        .and_then(|()| stderr.write_all(b"\n"))
        .expect("write Gate B structural sample");
}

fn validate_final(states: &[EkfState]) {
    for state in states {
        assert_state_close(*state, EkfState::REFERENCE_FINAL, EPISODE_LENGTH);
    }
}

fn validate_controls() {
    assert_eq!(trace_sha256(), TRACE_SHA256);
    assert_eq!(reference_trajectory().len(), EPISODE_LENGTH);

    let mut kernel = KernelFixture::new(1);
    assert_eq!(
        kernel.run_and_validate_every_turn(),
        REFERENCE_TRAJECTORY_SHA256
    );
    validate_final(kernel.states());

    let mut epoch = EpochFixture::new(1);
    assert_eq!(
        epoch.run_and_validate_every_turn(),
        REFERENCE_TRAJECTORY_SHA256
    );
    validate_final(epoch.published_states());
    assert_eq!(epoch.published_epoch(), EPISODE_LENGTH as u64);
    assert_eq!(epoch.retained_receipts(), EPISODE_LENGTH);
    assert_eq!(epoch.probe().candidate_seed_bytes, 0);
    assert_eq!(epoch.probe().published_buffer_copy_bytes, 0);
    assert_eq!(epoch.probe().publication_store_count, 1);

    let mut rejected = EpochFixture::new(1);
    rejected.force_rejected_turn_preserves_publication();

    let mut resident = ResidentKernelFixture::new(1);
    assert_eq!(
        resident.run_and_validate_every_turn(),
        REFERENCE_TRAJECTORY_SHA256
    );
    resident.validate_final();
    let mut resident_rejected = ResidentKernelFixture::new(1);
    resident_rejected.force_rejected_turn_preserves_publication();

    let mut scheduled = ResidentScheduledFixture::new(1);
    assert_eq!(
        scheduled.run_and_validate_every_turn(),
        REFERENCE_TRAJECTORY_SHA256
    );

    let mut complete = ResidentTurnFixture::new(1, 0, 1);
    assert_eq!(
        complete.run_and_validate_every_turn(),
        REFERENCE_TRAJECTORY_SHA256
    );
    complete.validate_final();

    let mut legacy = LegacyEkfFixture::new(1);
    assert_eq!(
        legacy.run_and_validate_every_turn(),
        REFERENCE_TRAJECTORY_SHA256
    );
    validate_final(&legacy.states());

    let mut full_epoch = FullWriteEpochFixture::new();
    full_epoch.run_episode();
    let full_hash = buffer_hash(full_epoch.published());
    let mut full_legacy = LegacyFullWriteFixture::new();
    full_legacy.run_episode();
    assert_eq!(buffer_hash(&full_legacy.published()), full_hash);
    let mut full_resident = ResidentFullWriteFixture::new();
    full_resident.run_episode();
    assert_eq!(buffer_hash(full_resident.published()), full_hash);
    let mut full_resident_turn = ResidentFullWriteTurnFixture::new();
    full_resident_turn.run_episode();
    assert_eq!(buffer_hash(full_resident_turn.published()), full_hash);
    let mut full_rejected = FullWriteEpochFixture::new();
    assert_eq!(
        full_rejected.abort_output_hash(),
        buffer_hash(full_rejected.published())
    );
}

fn rust_kernel(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_b/rust-kernel");
    for instances in SCALED_INSTANCES {
        let mut correctness = KernelFixture::new(instances);
        let trajectory_hash = correctness.run_and_validate_every_turn();
        assert_eq!(trajectory_hash, REFERENCE_TRAJECTORY_SHA256);
        group.bench_function(BenchmarkId::from_parameter(instances), |benchmark| {
            benchmark.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let mut fixture = KernelFixture::new(instances);
                    reset_allocations();
                    let started = Instant::now();
                    fixture.run_episode();
                    elapsed += started.elapsed();
                    let allocations = allocation_snapshot();
                    validate_final(fixture.states());
                    black_box(fixture.states());
                    report(
                        "rust-kernel",
                        instances,
                        allocations,
                        StructuralProbe::default(),
                        &trajectory_hash,
                        None,
                    );
                }
                elapsed
            });
        });
    }
    group.finish();
}

fn rust_epoch(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_b/rust-epoch");
    for instances in SCALED_INSTANCES {
        let mut correctness = EpochFixture::new(instances);
        let trajectory_hash = correctness.run_and_validate_every_turn();
        assert_eq!(trajectory_hash, REFERENCE_TRAJECTORY_SHA256);
        group.bench_function(BenchmarkId::from_parameter(instances), |benchmark| {
            benchmark.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let mut fixture = EpochFixture::new(instances);
                    reset_allocations();
                    let started = Instant::now();
                    fixture.run_episode();
                    elapsed += started.elapsed();
                    let allocations = allocation_snapshot();
                    validate_final(fixture.published_states());
                    black_box(fixture.published_states());
                    report(
                        "rust-epoch",
                        instances,
                        allocations,
                        fixture.probe().into(),
                        &trajectory_hash,
                        None,
                    );
                }
                elapsed
            });
        });
    }
    group.finish();
}

fn resident_kernel(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_b/mech-resident-kernel");
    for instances in SCALED_INSTANCES {
        let mut correctness = ResidentKernelFixture::new(instances);
        let trajectory_hash = correctness.run_and_validate_every_turn();
        assert_eq!(trajectory_hash, REFERENCE_TRAJECTORY_SHA256);
        group.bench_function(BenchmarkId::from_parameter(instances), |benchmark| {
            benchmark.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let mut fixture = ResidentKernelFixture::new(instances);
                    reset_allocations();
                    let started = Instant::now();
                    fixture.run_episode();
                    elapsed += started.elapsed();
                    let allocations = allocation_snapshot();
                    fixture.validate_final();
                    black_box(fixture.state(0));
                    report(
                        "mech-resident-kernel",
                        instances,
                        allocations,
                        StructuralProbe::default(),
                        &trajectory_hash,
                        None,
                    );
                }
                elapsed
            });
        });
    }
    group.finish();
}

fn resident_scheduled(c: &mut Criterion) {
    let mut correctness = ResidentScheduledFixture::new(1);
    let trajectory_hash = correctness.run_and_validate_every_turn();
    assert_eq!(trajectory_hash, REFERENCE_TRAJECTORY_SHA256);
    let mut group = c.benchmark_group("gate_b/mech-resident-scheduled");
    group.bench_function("1", |benchmark| {
        benchmark.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let mut fixture = ResidentScheduledFixture::new(1);
                reset_allocations();
                let started = Instant::now();
                fixture.run_episode();
                elapsed += started.elapsed();
                let allocations = allocation_snapshot();
                fixture.validate_final();
                black_box(fixture.state(0));
                report(
                    "mech-resident-scheduled",
                    1,
                    allocations,
                    StructuralProbe {
                        candidate_written_bytes: 96,
                        publication_store_count: 1,
                        dirty_node_count: 15,
                        ..StructuralProbe::default()
                    },
                    &trajectory_hash,
                    None,
                );
            }
            elapsed
        });
    });
    group.finish();
}

fn resident_turn(c: &mut Criterion) {
    let mut correctness = ResidentTurnFixture::new(1, 0, 1);
    let trajectory_hash = correctness.run_and_validate_every_turn();
    assert_eq!(trajectory_hash, REFERENCE_TRAJECTORY_SHA256);
    correctness.validate_final();

    let mut group = c.benchmark_group("gate_b/mech-resident-turn");
    for (name, history, next_epoch) in [
        ("history-0-low-epoch", 0, 1),
        ("history-1000-low-epoch", 1_000, 1),
        ("history-100000-low-epoch", 100_000, 1),
        ("history-0-high-epoch", 0, 1_000_000_001),
    ] {
        group.bench_function(name, |benchmark| {
            benchmark.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let mut fixture = ResidentTurnFixture::new(1, history, next_epoch);
                    reset_allocations();
                    let started = Instant::now();
                    fixture.run_episode();
                    elapsed += started.elapsed();
                    let allocations = allocation_snapshot();
                    fixture.validate_final();
                    black_box(fixture.state(0));
                    report_dimensions(
                        "mech-resident-turn",
                        1,
                        history,
                        next_epoch,
                        allocations,
                        fixture.probe().into(),
                        &trajectory_hash,
                        None,
                    );
                }
                elapsed
            });
        });
    }
    group.finish();
}

fn resident_artifact(c: &mut Criterion) {
    let mut complete_group = c.benchmark_group("gate_b/mech-resident-artifact");
    for (route, lane, benchmark_name, history, next_epoch) in [
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-source",
            "source-history-0-low-epoch",
            0,
            1,
        ),
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-source",
            "source-history-1000-low-epoch",
            1_000,
            1,
        ),
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-source",
            "source-history-100000-low-epoch",
            100_000,
            1,
        ),
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-source",
            "source-history-0-high-epoch",
            0,
            1_000_000_001,
        ),
        (
            ArtifactRoute::Bytecode,
            "mech-resident-artifact-bytecode",
            "bytecode-history-0-low-epoch",
            0,
            1,
        ),
    ] {
        let mut correctness = ResidentArtifactFixture::with_controls(route, history, next_epoch);
        let trajectory_hash = correctness.run_and_validate_every_turn();
        assert_eq!(trajectory_hash, REFERENCE_TRAJECTORY_SHA256);
        correctness.validate_final();
        complete_group.bench_function(benchmark_name, |benchmark| {
            benchmark.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let mut fixture =
                        ResidentArtifactFixture::with_controls(route, history, next_epoch);
                    reset_allocations();
                    let started = Instant::now();
                    fixture.run_episode();
                    elapsed += started.elapsed();
                    let allocations = allocation_snapshot();
                    fixture.validate_final();
                    black_box(fixture.state());
                    report_dimensions(
                        lane,
                        1,
                        history,
                        next_epoch,
                        allocations,
                        fixture.probe().into(),
                        &trajectory_hash,
                        None,
                    );
                }
                elapsed
            });
        });
    }
    complete_group.finish();

    let mut kernel_group = c.benchmark_group("gate_b/mech-resident-artifact-kernel");
    for (route, lane, benchmark_name) in [
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-kernel-source",
            "source",
        ),
        (
            ArtifactRoute::Bytecode,
            "mech-resident-artifact-kernel-bytecode",
            "bytecode",
        ),
    ] {
        let mut correctness = ResidentArtifactKernelFixture::new(route);
        let trajectory_hash = correctness.run_and_validate_every_turn();
        assert_eq!(trajectory_hash, REFERENCE_TRAJECTORY_SHA256);
        correctness.validate_final();
        kernel_group.bench_function(benchmark_name, |benchmark| {
            benchmark.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let mut fixture = ResidentArtifactKernelFixture::new(route);
                    reset_allocations();
                    let started = Instant::now();
                    fixture.run_episode();
                    elapsed += started.elapsed();
                    let allocations = allocation_snapshot();
                    fixture.validate_final();
                    black_box(fixture.state());
                    report(
                        lane,
                        1,
                        allocations,
                        fixture.probe().into(),
                        &trajectory_hash,
                        None,
                    );
                }
                elapsed
            });
        });
    }
    kernel_group.finish();
}

fn legacy_atomic(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_b/mech-legacy-atomic");
    for instances in SCALED_INSTANCES {
        let mut correctness = LegacyEkfFixture::new(instances);
        let trajectory_hash = correctness.run_and_validate_every_turn();
        assert_eq!(trajectory_hash, REFERENCE_TRAJECTORY_SHA256);
        group.bench_function(BenchmarkId::from_parameter(instances), |benchmark| {
            benchmark.iter_custom(|iterations| {
                let mut elapsed = Duration::ZERO;
                for _ in 0..iterations {
                    let mut fixture = LegacyEkfFixture::new(instances);
                    reset_allocations();
                    let started = Instant::now();
                    fixture.run_episode();
                    elapsed += started.elapsed();
                    let allocations = allocation_snapshot();
                    let states = fixture.states();
                    validate_final(&states);
                    black_box(states);
                    report(
                        "mech-legacy-atomic",
                        instances,
                        allocations,
                        StructuralProbe::default(),
                        &trajectory_hash,
                        None,
                    );
                }
                elapsed
            });
        });
    }
    group.finish();
}

fn full_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("gate_b/full-write");
    group.bench_function("rust-epoch", |benchmark| {
        benchmark.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let mut fixture = FullWriteEpochFixture::new();
                reset_allocations();
                let started = Instant::now();
                fixture.run_episode();
                elapsed += started.elapsed();
                let allocations = allocation_snapshot();
                let output_hash = buffer_hash(fixture.published());
                let mut rejected = FullWriteEpochFixture::new();
                let abort_hash = rejected.abort_output_hash();
                black_box(fixture.published());
                report(
                    "rust-epoch-full-write",
                    1,
                    allocations,
                    fixture.probe().into(),
                    &output_hash,
                    Some(&abort_hash),
                );
            }
            elapsed
        });
    });
    group.bench_function("mech-legacy-atomic", |benchmark| {
        benchmark.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let mut fixture = LegacyFullWriteFixture::new();
                reset_allocations();
                let started = Instant::now();
                fixture.run_episode();
                elapsed += started.elapsed();
                let allocations = allocation_snapshot();
                let published = fixture.published();
                let output_hash = buffer_hash(&published);
                black_box(published);
                report(
                    "mech-legacy-atomic-full-write",
                    1,
                    allocations,
                    StructuralProbe {
                        candidate_written_bytes: support::gate_b::full_write::WRITTEN_BYTES,
                        ..StructuralProbe::default()
                    },
                    &output_hash,
                    None,
                );
            }
            elapsed
        });
    });
    group.bench_function("mech-resident-kernel", |benchmark| {
        benchmark.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let mut fixture = ResidentFullWriteFixture::new();
                reset_allocations();
                let started = Instant::now();
                fixture.run_episode();
                elapsed += started.elapsed();
                let allocations = allocation_snapshot();
                let output_hash = buffer_hash(fixture.published());
                let mut rejected = ResidentFullWriteFixture::new();
                let abort_hash = rejected.abort_output_hash();
                black_box(fixture.published());
                report(
                    "mech-resident-kernel-full-write",
                    1,
                    allocations,
                    StructuralProbe::default(),
                    &output_hash,
                    Some(&abort_hash),
                );
            }
            elapsed
        });
    });
    group.bench_function("mech-resident-turn", |benchmark| {
        benchmark.iter_custom(|iterations| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iterations {
                let mut fixture = ResidentFullWriteTurnFixture::new();
                reset_allocations();
                let started = Instant::now();
                fixture.run_episode();
                elapsed += started.elapsed();
                let allocations = allocation_snapshot();
                let output_hash = buffer_hash(fixture.published());
                let mut rejected = ResidentFullWriteTurnFixture::new();
                let abort_hash = rejected.abort_output_hash_with_rejection();
                black_box(fixture.published());
                report(
                    "mech-resident-turn-full-write",
                    1,
                    allocations,
                    fixture.probe().into(),
                    &output_hash,
                    Some(&abort_hash),
                );
            }
            elapsed
        });
    });
    group.finish();
}

#[cfg(feature = "runtime_bench_probes")]
fn resident_structural_samples() {
    for (route, lane, history, next_epoch) in [
        (ArtifactRoute::Source, "mech-resident-artifact-source", 0, 1),
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-source",
            1_000,
            1,
        ),
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-source",
            100_000,
            1,
        ),
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-source",
            0,
            1_000_000_001,
        ),
        (
            ArtifactRoute::Bytecode,
            "mech-resident-artifact-bytecode",
            0,
            1,
        ),
    ] {
        let mut fixture = ResidentArtifactFixture::with_controls(route, history, next_epoch);
        let trajectory_hash = fixture.run_and_validate_every_turn();
        let probe = fixture.probe();
        let mut rejected = ResidentArtifactFixture::new(route);
        let abort_hash = rejected.abort_output_hash();
        report_dimensions(
            lane,
            1,
            history,
            next_epoch,
            AllocationSnapshot::default(),
            probe.into(),
            &trajectory_hash,
            Some(&abort_hash),
        );
    }

    for (route, lane) in [
        (
            ArtifactRoute::Source,
            "mech-resident-artifact-kernel-source",
        ),
        (
            ArtifactRoute::Bytecode,
            "mech-resident-artifact-kernel-bytecode",
        ),
    ] {
        let mut fixture = ResidentArtifactKernelFixture::new(route);
        let trajectory_hash = fixture.run_and_validate_every_turn();
        report(
            lane,
            1,
            AllocationSnapshot::default(),
            fixture.probe().into(),
            &trajectory_hash,
            None,
        );
    }

    for instances in SCALED_INSTANCES {
        let mut fixture = ResidentKernelFixture::new(instances);
        let trajectory_hash = fixture.run_and_validate_every_turn();
        report(
            "mech-resident-kernel",
            instances,
            AllocationSnapshot::default(),
            fixture.probe().into(),
            &trajectory_hash,
            None,
        );
    }

    let mut fixture = ResidentFullWriteFixture::new();
    fixture.run_episode();
    let output_hash = buffer_hash(fixture.published());
    let mut rejected = ResidentFullWriteFixture::new();
    let abort_hash = rejected.abort_output_hash();
    report(
        "mech-resident-kernel-full-write",
        1,
        AllocationSnapshot::default(),
        fixture.probe().into(),
        &output_hash,
        Some(&abort_hash),
    );

    let mut scheduled = ResidentScheduledFixture::new(1);
    let scheduled_hash = scheduled.run_and_validate_every_turn();
    report(
        "mech-resident-scheduled",
        1,
        AllocationSnapshot::default(),
        StructuralProbe {
            candidate_written_bytes: 96,
            publication_store_count: 1,
            dirty_node_count: 15,
            ..StructuralProbe::default()
        },
        &scheduled_hash,
        None,
    );

    for (history, next_epoch) in [(0, 1), (1_000, 1), (100_000, 1), (0, 1_000_000_001)] {
        let mut complete = ResidentTurnFixture::new(1, history, next_epoch);
        let complete_hash = complete.run_and_validate_every_turn();
        report_dimensions(
            "mech-resident-turn",
            1,
            history,
            next_epoch,
            AllocationSnapshot::default(),
            complete.probe().into(),
            &complete_hash,
            None,
        );
    }

    let mut complete_full = ResidentFullWriteTurnFixture::new();
    complete_full.run_episode();
    let complete_full_hash = buffer_hash(complete_full.published());
    let mut rejected_full = ResidentFullWriteTurnFixture::new();
    let complete_abort_hash = rejected_full.abort_output_hash_with_rejection();
    report(
        "mech-resident-turn-full-write",
        1,
        AllocationSnapshot::default(),
        complete_full.probe().into(),
        &complete_full_hash,
        Some(&complete_abort_hash),
    );
}

#[cfg(feature = "runtime_bench_probes")]
fn legacy_structural_samples() {
    for instances in SCALED_INSTANCES {
        let mut fixture = LegacyEkfFixture::new(instances);
        reset_gate_a_costs();
        reset_allocations();
        let trajectory_hash = fixture.run_and_validate_every_turn();
        let allocations = allocation_snapshot();
        let costs = gate_a_cost_snapshot();
        report(
            "mech-legacy-atomic",
            instances,
            allocations,
            StructuralProbe {
                commit_runtime_call_count: costs.commit_runtime_call_count,
                legacy_journal_capture_count: costs.reactive_journal_cell_count,
                ..StructuralProbe::default()
            },
            &trajectory_hash,
            None,
        );
    }

    let mut fixture = LegacyFullWriteFixture::new();
    reset_gate_a_costs();
    reset_allocations();
    fixture.run_episode();
    let allocations = allocation_snapshot();
    let costs = gate_a_cost_snapshot();
    let published = fixture.published();
    let output_hash = buffer_hash(&published);
    report(
        "mech-legacy-atomic-full-write",
        1,
        allocations,
        StructuralProbe {
            candidate_written_bytes: support::gate_b::full_write::WRITTEN_BYTES,
            commit_runtime_call_count: costs.commit_runtime_call_count,
            legacy_journal_capture_count: costs.reactive_journal_cell_count,
            ..StructuralProbe::default()
        },
        &output_hash,
        None,
    );
}

fn gate_b_controls(c: &mut Criterion) {
    validate_controls();
    #[cfg(feature = "runtime_bench_probes")]
    if std::env::var_os("MECH_GATE_B_STRUCTURAL_ONLY").is_some() {
        resident_structural_samples();
        legacy_structural_samples();
        return;
    }
    rust_kernel(c);
    rust_epoch(c);
    resident_kernel(c);
    resident_scheduled(c);
    resident_turn(c);
    resident_artifact(c);
    legacy_atomic(c);
    full_write(c);
}

criterion_group!(benches, gate_b_controls);
criterion_main!(benches);
