#![cfg(feature = "runtime_bench_gate_d3")]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use mech_core::{
    ExecutionHostFunctionRequest, ExecutionResourceRequest, LegacyValue, MResult, MechError,
    MechExecutionServices, ParsedProgram, ReactiveInstanceId, ValRef,
};
use mech_engine::__resident::{
    ActivationFacts, CapturedValueInput, ResidentActivationOptions, ResidentExternalAdmission,
    ResidentIntegrityMode, activate_external, activate_with_options,
};
use mech_engine::{
    MechProgram, MechProgramConfig, ProgramArtifact, decode_program_artifact_sections,
};
use mech_runtime::runtime::program::external::test_provider::{
    D3InputProvider, D3ProviderTrace, D3SceneProvider, D3TransactionalProvider,
    SharedD3ProviderTrace,
};
use mech_runtime::{
    CapturedInputBatch, ExactRequirementAuthority, ResidentDurabilityPolicy,
    ResidentExternalContractResolver, ResidentExternalCoordinator, ResidentExternalLimits,
    ResidentExternalTurnOutcome, ResidentTurnRecord, RuntimeResourceRegistry,
    captured_value_from_legacy, resident_effect_ids_hash, resident_idempotency_keys_hash,
};
use serde_json::json;
use sha2::{Digest, Sha256};

const TURNS: usize = 4_096;
const SAMPLES: usize = 10;
const HISTORY_SAMPLES: usize = 3;
const EFFECT_SOURCE: &str =
    include_str!("../../../tests/fixtures/resident-external/effect-source.mec");
const TRANSACTIONAL_SOURCE: &str =
    include_str!("../../../tests/fixtures/resident-external/transactional-source.mec");

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[derive(Clone, Copy, Debug)]
enum FixtureKind {
    Effect,
    Transactional,
}

impl FixtureKind {
    fn name(self) -> &'static str {
        match self {
            Self::Effect => "effect",
            Self::Transactional => "transactional",
        }
    }
}

#[derive(Debug)]
struct LaneResult {
    elapsed_ns: u128,
    state_hash: u64,
    receipt_hash: String,
    effect_batch_hash: String,
    effect_id_hash: String,
    idempotency_key_hash: String,
    reads: u64,
    receipts: usize,
    publication_stores: usize,
    outbox_batch_appends: usize,
    batches: Vec<CapturedInputBatch>,
    records: Vec<ResidentTurnRecord>,
}

#[derive(Clone, Copy, Debug)]
struct StructuralResult {
    commit_runtime_calls: usize,
    legacy_journal_captures: usize,
    runtime_execution_transaction_constructions: usize,
    publication_stores_per_accepted_turn: usize,
    publication_stores_per_rejected_turn: usize,
    post_candidate_rejections: usize,
    rejected_receipt_appends: usize,
    rejected_outbox_batch_appends: usize,
    rejected_provider_preparation_attempts: usize,
    rejected_delivery_count: usize,
    effects_delivered_before_publication: usize,
    effects_delivered_for_rejected_turns: usize,
}

#[test]
fn controlled_d3_resident_external_evidence() -> MResult<()> {
    let catalog = mech_stdlib::source_catalog();
    let (probe_artifact, _) = compile_fixture(FixtureKind::Effect)?;
    let candidate_allocations = candidate_allocation_probe(&probe_artifact, &catalog)?;
    let structural = external_structural_probe(&probe_artifact, &catalog)?;
    println!(
        "GATE_D3_STRUCTURAL {}",
        serde_json::to_string(&json!({
            "candidate_allocations": candidate_allocations,
            "commit_runtime_calls": structural.commit_runtime_calls,
            "legacy_journal_captures": structural.legacy_journal_captures,
            "runtime_execution_transaction_constructions": structural.runtime_execution_transaction_constructions,
            "publication_stores_per_accepted_turn": structural.publication_stores_per_accepted_turn,
            "publication_stores_per_rejected_turn": structural.publication_stores_per_rejected_turn,
            "post_candidate_rejections": structural.post_candidate_rejections,
            "rejected_receipt_appends": structural.rejected_receipt_appends,
            "rejected_outbox_batch_appends": structural.rejected_outbox_batch_appends,
            "rejected_provider_preparation_attempts": structural.rejected_provider_preparation_attempts,
            "rejected_delivery_count": structural.rejected_delivery_count,
            "effects_delivered_before_publication": structural.effects_delivered_before_publication,
            "effects_delivered_for_rejected_turns": structural.effects_delivered_for_rejected_turns,
        }))
        .expect("serialize D3 structural evidence")
    );
    for kind in [FixtureKind::Effect, FixtureKind::Transactional] {
        let (source, bytecode) = compile_fixture(kind)?;
        assert_eq!(source.revision(), bytecode.revision());
        assert_eq!(source.requirements(), bytecode.requirements());

        let mut replay_batches = None;
        let mut replay_records = None;
        let mut source_first = None;
        for sample in 0..SAMPLES {
            let source_result = run_lane(&source, &catalog, kind, 0, 1, sample == 0)?;
            let bytecode_result = run_lane(&bytecode, &catalog, kind, 0, 1, false)?;
            assert_equivalent(&source_result, &bytecode_result);
            emit_sample(
                kind,
                "source",
                sample,
                &source_result,
                candidate_allocations,
            );
            emit_sample(
                kind,
                "bytecode",
                sample,
                &bytecode_result,
                candidate_allocations,
            );
            if sample == 0 {
                replay_batches = Some(source_result.batches.clone());
                replay_records = Some(source_result.records.clone());
                source_first = Some(source_result);
            }
        }

        let replay = run_replay(
            &source,
            &catalog,
            kind,
            replay_batches.as_deref().expect("first live sample"),
            replay_records.as_deref().expect("first live sample"),
        )?;
        let source_first = source_first.as_ref().expect("first live sample result");
        assert_eq!(source_first.state_hash, replay.state_hash);
        assert_eq!(source_first.receipt_hash, replay.receipt_hash);
        assert_eq!(source_first.effect_batch_hash, replay.effect_batch_hash);
        assert_eq!(replay.reads, 0);
        println!(
            "GATE_D3_REPLAY {}",
            serde_json::to_string(&json!({
                "fixture": kind.name(),
                "turns": TURNS,
                "elapsed_ns": replay.elapsed_ns,
                "provider_reads": replay.reads,
                "state_hash": replay.state_hash,
                "receipt_hash": replay.receipt_hash,
                "effect_batch_hash": replay.effect_batch_hash,
                "effect_id_hash": replay.effect_id_hash,
                "idempotency_key_hash": replay.idempotency_key_hash,
                "publication_stores": replay.publication_stores,
            }))
            .expect("serialize D3 replay sample")
        );
    }

    let (effect, _) = compile_fixture(FixtureKind::Effect)?;
    for (lane, history, next_epoch) in [
        ("history-0", 0, 1),
        ("history-1k", 1_000, 1),
        ("history-100k", 100_000, 1),
        ("high-epoch", 0, u64::MAX - TURNS as u64 - 1),
    ] {
        for sample in 0..HISTORY_SAMPLES {
            let result = run_lane(
                &effect,
                &catalog,
                FixtureKind::Effect,
                history,
                next_epoch,
                false,
            )?;
            println!(
                "GATE_D3_CONTROL {}",
                serde_json::to_string(&json!({
                    "lane": lane,
                    "sample": sample,
                    "turns": TURNS,
                    "elapsed_ns": result.elapsed_ns,
                    "retained_history": history,
                    "next_epoch": next_epoch,
                }))
                .expect("serialize D3 control sample")
            );
        }
    }
    Ok(())
}

fn candidate_allocation_probe(
    artifact: &ProgramArtifact,
    catalog: &Arc<mech_core::FunctionCatalog>,
) -> MResult<usize> {
    let mut instance = activate_with_options(
        ReactiveInstanceId::new(899, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentActivationOptions {
            external: ResidentExternalAdmission::StructuralOnly,
            ..ResidentActivationOptions::default()
        },
    )
    .map_err(|error| benchmark_error(&format!("activate D3 allocation probe: {error:?}")))?;
    let input = instance
        .plan
        .inputs
        .first()
        .expect("D3 effect fixture input")
        .clone();
    let value = captured_value_from_legacy(
        &LegacyValue::F64(mech_core::Ref::new(0.25)),
        input.schema,
        &input.shape,
        artifact.schemas(),
    )?;
    let captured = [CapturedValueInput {
        slot: input.slot,
        value: &value,
    }];
    instance
        .prepare_turn_values(&captured)
        .map_err(|error| benchmark_error(&format!("warm D3 candidate: {error:?}")))?
        .abort();
    ALLOCATIONS.store(0, Ordering::SeqCst);
    let prepared = instance
        .prepare_turn_values(&captured)
        .map_err(|error| benchmark_error(&format!("measure D3 candidate: {error:?}")))?;
    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
    prepared.abort();
    Ok(allocations)
}

fn compile_fixture(kind: FixtureKind) -> MResult<(ProgramArtifact, ProgramArtifact)> {
    let trace = Arc::new(Mutex::new(D3ProviderTrace::default()));
    let providers = providers(kind, trace)?;
    let resolver = ResidentExternalContractResolver::new(&providers);
    let mut services = PlanningServices;
    let mut program = MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        mech_stdlib::source_catalog(),
    );
    program.run_string_with_services(
        match kind {
            FixtureKind::Effect => EFFECT_SOURCE,
            FixtureKind::Transactional => TRANSACTIONAL_SOURCE,
        },
        &mut services,
    )?;
    let product = program.compile_program_product_with_external_contracts(&resolver)?;
    let parsed = ParsedProgram::from_bytes(product.bytecode())?;
    let decoded = decode_program_artifact_sections(&parsed.artifact)
        .map_err(|error| benchmark_error(&format!("decode D3 artifact: {error:?}")))?;
    Ok((product.artifact().clone(), decoded))
}

fn providers(kind: FixtureKind, trace: SharedD3ProviderTrace) -> MResult<RuntimeResourceRegistry> {
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(D3InputProvider::new(0.25, trace.clone())))?;
    match kind {
        FixtureKind::Effect => {
            providers.register_provider(Box::new(D3SceneProvider::new(trace)))?;
        }
        FixtureKind::Transactional => {
            providers.register_provider(Box::new(D3TransactionalProvider::new(trace)))?;
        }
    }
    Ok(providers)
}

fn external_structural_probe(
    artifact: &ProgramArtifact,
    catalog: &Arc<mech_core::FunctionCatalog>,
) -> MResult<StructuralResult> {
    let accepted_trace = Arc::new(Mutex::new(D3ProviderTrace::default()));
    let accepted_providers = providers(FixtureKind::Effect, accepted_trace)?;
    let accepted_instance = activate_external(
        ReactiveInstanceId::new(910, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| benchmark_error(&format!("activate D3 structural turn: {error:?}")))?;
    let authority = ExactRequirementAuthority::new(
        artifact
            .requirements()
            .iter()
            .map(|(_, requirement)| requirement.clone()),
    )?;
    let mut accepted = ResidentExternalCoordinator::new_live(
        accepted_instance,
        Arc::new(artifact.clone()),
        &accepted_providers,
        &authority,
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;
    let accepted_before = accepted.instance().published_epoch().get();
    require_accepted(accepted.execute_turn()?)?;
    let accepted_after = accepted.instance().published_epoch().get();
    let accepted_probe = accepted.structural_probe();

    let rejected_trace = Arc::new(Mutex::new(D3ProviderTrace::default()));
    let mut rejected_providers = RuntimeResourceRegistry::new();
    rejected_providers
        .register_provider(Box::new(D3InputProvider::new(0.25, rejected_trace.clone())))?;
    rejected_providers.register_provider(Box::new(D3SceneProvider::with_preparation_failures(
        rejected_trace.clone(),
        1,
    )))?;
    let rejected_instance = activate_external(
        ReactiveInstanceId::new(911, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| benchmark_error(&format!("activate D3 rejected probe: {error:?}")))?;
    let mut rejected = ResidentExternalCoordinator::new_live(
        rejected_instance,
        Arc::new(artifact.clone()),
        &rejected_providers,
        &authority,
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits::default(),
    )?;
    let rejected_before = rejected.instance().published_epoch().get();
    let rejected_receipts_before = rejected.receipts().count();
    let rejected_probe_before = rejected.structural_probe();
    assert!(matches!(
        rejected.execute_turn()?,
        ResidentExternalTurnOutcome::Rejected {
            phase: mech_runtime::TurnFailurePhase::ExternalPrepare,
            ..
        }
    ));
    let rejected_after = rejected.instance().published_epoch().get();
    let rejected_receipts_after = rejected.receipts().count();
    let rejected_probe = rejected.structural_probe();
    let rejected_trace = rejected_trace.lock().expect("D3 rejected trace").clone();
    assert_eq!(rejected_trace.read_calls, 1);
    assert_eq!(rejected_trace.prepared.len(), 1);
    let rejected_deliveries = rejected_trace.delivered;

    Ok(StructuralResult {
        commit_runtime_calls: accepted_probe
            .commit_runtime_call_count
            .saturating_add(rejected_probe.commit_runtime_call_count),
        legacy_journal_captures: accepted_probe
            .legacy_journal_capture_count
            .saturating_add(rejected_probe.legacy_journal_capture_count),
        runtime_execution_transaction_constructions: accepted_probe
            .runtime_execution_transaction_construction_count
            .saturating_add(rejected_probe.runtime_execution_transaction_construction_count),
        publication_stores_per_accepted_turn: (accepted_after - accepted_before) as usize,
        publication_stores_per_rejected_turn: (rejected_after - rejected_before) as usize,
        post_candidate_rejections: usize::from(rejected_trace.prepared.len() == 1),
        rejected_receipt_appends: rejected_receipts_after.saturating_sub(rejected_receipts_before),
        rejected_outbox_batch_appends: rejected_probe
            .outbox_batch_append_count
            .saturating_sub(rejected_probe_before.outbox_batch_append_count),
        rejected_provider_preparation_attempts: rejected_trace.prepared.len(),
        rejected_delivery_count: rejected_deliveries as usize,
        effects_delivered_before_publication: accepted_probe
            .effects_delivered_before_publication
            .saturating_add(rejected_probe.effects_delivered_before_publication),
        effects_delivered_for_rejected_turns: rejected_probe
            .effects_delivered_for_rejected_turns
            .saturating_add(rejected_deliveries as usize),
    })
}

fn run_lane(
    artifact: &ProgramArtifact,
    catalog: &Arc<mech_core::FunctionCatalog>,
    kind: FixtureKind,
    history: usize,
    next_epoch: u64,
    retain_batches: bool,
) -> MResult<LaneResult> {
    let trace = Arc::new(Mutex::new(D3ProviderTrace::default()));
    let providers = providers(kind, trace.clone())?;
    let instance = activate_external(
        ReactiveInstanceId::new(900, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| {
        benchmark_error(&format!(
            "activate D3 artifact: {error:?}; nodes={:?}",
            artifact.nodes()
        ))
    })?;
    let authority = ExactRequirementAuthority::new(
        artifact
            .requirements()
            .iter()
            .map(|(_, requirement)| requirement.clone()),
    )?;
    let total = history.saturating_add(TURNS).saturating_add(32);
    let mut coordinator = ResidentExternalCoordinator::new_live(
        instance,
        Arc::new(artifact.clone()),
        &providers,
        &authority,
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits {
            input_batches: total,
            input_bytes: total.saturating_mul(512),
            receipts: total,
            receipt_bytes: total.saturating_mul(512),
            outbox_effects: TURNS.saturating_add(32),
            outbox_bytes: TURNS.saturating_mul(512),
        },
    )?;
    if next_epoch != 1 {
        coordinator.set_next_epoch_for_benchmark(next_epoch);
    }
    for _ in 0..history {
        require_accepted(coordinator.execute_turn()?)?;
    }
    {
        let mut trace = trace.lock().expect("D3 provider trace");
        trace.read_calls = 0;
        trace.prepared.clear();
        trace.delivered = 0;
        trace.applied = 0;
    }
    let receipt_start = coordinator.receipts().count();
    let input_start = coordinator.input_facts().count();
    let structural_start = coordinator.structural_probe();
    let started = Instant::now();
    for _ in 0..TURNS {
        require_accepted(coordinator.execute_turn()?)?;
    }
    let elapsed_ns = started.elapsed().as_nanos();
    let trace_snapshot = trace.lock().expect("D3 provider trace").clone();
    let receipts = coordinator
        .receipts()
        .skip(receipt_start)
        .map(|(_, record)| record.body.clone())
        .collect::<Vec<_>>();
    let records = if retain_batches {
        coordinator
            .receipts()
            .skip(receipt_start)
            .map(|(_, record)| record.clone())
            .collect()
    } else {
        Vec::new()
    };
    let batches = if retain_batches {
        coordinator
            .input_facts()
            .skip(input_start)
            .map(|(_, batch)| batch.clone())
            .collect()
    } else {
        Vec::new()
    };
    let state_hash = receipts.last().expect("D3 receipt").state_hash;
    let receipt_hash = debug_hash(receipts.iter());
    let effect_batch_hash = debug_hash(receipts.iter().map(|receipt| receipt.effect_batch_hash));
    assert_eq!(trace_snapshot.prepared.len(), receipts.len());
    for ((effect_id, idempotency_key), receipt) in trace_snapshot.prepared.iter().zip(&receipts) {
        assert_eq!(
            receipt.effect_ids_hash,
            resident_effect_ids_hash([*effect_id])
        );
        assert_eq!(
            receipt.idempotency_keys_hash,
            resident_idempotency_keys_hash([idempotency_key.as_str()])
        );
    }
    let effect_id_hash = debug_hash(receipts.iter().map(|receipt| receipt.effect_ids_hash));
    let idempotency_key_hash =
        debug_hash(receipts.iter().map(|receipt| receipt.idempotency_keys_hash));
    let structural_end = coordinator.structural_probe();
    Ok(LaneResult {
        elapsed_ns,
        state_hash,
        receipt_hash,
        effect_batch_hash,
        effect_id_hash,
        idempotency_key_hash,
        reads: trace_snapshot.read_calls,
        receipts: receipts.len(),
        publication_stores: structural_end
            .publication_store_count
            .saturating_sub(structural_start.publication_store_count),
        outbox_batch_appends: structural_end
            .outbox_batch_append_count
            .saturating_sub(structural_start.outbox_batch_append_count),
        batches,
        records,
    })
}

fn run_replay(
    artifact: &ProgramArtifact,
    catalog: &Arc<mech_core::FunctionCatalog>,
    _kind: FixtureKind,
    batches: &[CapturedInputBatch],
    records: &[ResidentTurnRecord],
) -> MResult<LaneResult> {
    let trace = Arc::new(Mutex::new(D3ProviderTrace::default()));
    let instance = activate_external(
        ReactiveInstanceId::new(900, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| benchmark_error(&format!("activate D3 replay artifact: {error:?}")))?;
    let mut coordinator = ResidentExternalCoordinator::new_replay(
        instance,
        Arc::new(artifact.clone()),
        ResidentDurabilityPolicy::Retained,
        ResidentExternalLimits {
            input_batches: TURNS + 32,
            input_bytes: (TURNS + 32) * 512,
            receipts: TURNS + 32,
            receipt_bytes: (TURNS + 32) * 512,
            outbox_effects: TURNS + 32,
            outbox_bytes: (TURNS + 32) * 512,
        },
    )?;
    let started = Instant::now();
    assert_eq!(batches.len(), records.len());
    for (batch, record) in batches.iter().zip(records) {
        require_accepted(coordinator.execute_replay_batch(Some(batch), record)?)?;
    }
    let elapsed_ns = started.elapsed().as_nanos();
    let receipts = coordinator
        .receipts()
        .map(|(_, record)| record.body.clone())
        .collect::<Vec<_>>();
    let trace_snapshot = trace.lock().expect("D3 provider trace").clone();
    let probe = coordinator.structural_probe();
    Ok(LaneResult {
        elapsed_ns,
        state_hash: receipts.last().expect("D3 replay receipt").state_hash,
        receipt_hash: debug_hash(receipts.iter()),
        effect_batch_hash: debug_hash(receipts.iter().map(|receipt| receipt.effect_batch_hash)),
        effect_id_hash: debug_hash(receipts.iter().map(|receipt| receipt.effect_ids_hash)),
        idempotency_key_hash: debug_hash(
            receipts.iter().map(|receipt| receipt.idempotency_keys_hash),
        ),
        reads: trace_snapshot.read_calls,
        receipts: receipts.len(),
        publication_stores: probe.publication_store_count,
        outbox_batch_appends: probe.outbox_batch_append_count,
        batches: Vec::new(),
        records: Vec::new(),
    })
}

fn emit_sample(
    kind: FixtureKind,
    artifact: &str,
    sample: usize,
    result: &LaneResult,
    candidate_allocations: usize,
) {
    println!(
        "GATE_D3_SAMPLE {}",
        serde_json::to_string(&json!({
            "fixture": kind.name(),
            "artifact": artifact,
            "sample": sample,
            "turns": TURNS,
            "elapsed_ns": result.elapsed_ns,
            "state_hash": result.state_hash,
            "receipt_hash": result.receipt_hash,
            "effect_batch_hash": result.effect_batch_hash,
            "effect_id_hash": result.effect_id_hash,
            "idempotency_key_hash": result.idempotency_key_hash,
            "provider_reads": result.reads,
            "receipt_appends": result.receipts,
            "ordinary_outbox_appends": result.outbox_batch_appends,
            "publication_stores": result.publication_stores,
            "candidate_allocations": candidate_allocations,
        }))
        .expect("serialize D3 sample")
    );
}

fn assert_equivalent(left: &LaneResult, right: &LaneResult) {
    assert_eq!(left.state_hash, right.state_hash);
    assert_eq!(left.receipt_hash, right.receipt_hash);
    assert_eq!(left.effect_batch_hash, right.effect_batch_hash);
    assert_eq!(left.effect_id_hash, right.effect_id_hash);
    assert_eq!(left.idempotency_key_hash, right.idempotency_key_hash);
}

fn require_accepted(outcome: ResidentExternalTurnOutcome) -> MResult<()> {
    if matches!(outcome, ResidentExternalTurnOutcome::Accepted { .. }) {
        Ok(())
    } else {
        Err(benchmark_error(&format!(
            "D3 controlled turn was not accepted: {outcome:?}"
        )))
    }
}

fn debug_hash<I, T>(values: I) -> String
where
    I: IntoIterator<Item = T>,
    T: std::fmt::Debug,
{
    let mut hash = Sha256::new();
    for value in values {
        hash.update(format!("{value:?}\n"));
    }
    hash.finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug)]
struct PlanningServices;

impl MechExecutionServices for PlanningServices {
    fn invoke_host_function(
        &mut self,
        _request: &ExecutionHostFunctionRequest,
        _arguments: &[LegacyValue],
    ) -> MResult<LegacyValue> {
        Err(benchmark_error("D3 fixture has no host call"))
    }

    fn plan_resource_read_output(
        &mut self,
        _request: &ExecutionResourceRequest,
    ) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(mech_core::Ref::new(0.25)))
    }

    fn read_resource(&mut self, _request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        Ok(LegacyValue::F64(mech_core::Ref::new(0.25)))
    }

    fn write_resource(
        &mut self,
        _request: &ExecutionResourceRequest,
        _value: &LegacyValue,
    ) -> MResult<()> {
        Ok(())
    }

    fn bind_live_resource(
        &mut self,
        _interpreter_id: u64,
        _request: &ExecutionResourceRequest,
        _target: ValRef,
    ) -> MResult<()> {
        Ok(())
    }
}

fn benchmark_error(message: &str) -> MechError {
    MechError::new(
        mech_core::GenericError {
            msg: message.to_owned(),
        },
        None,
    )
}
