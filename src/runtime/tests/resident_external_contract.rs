#![cfg(feature = "resident_external_test_support")]

use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, ParsedProgram, ReactiveInstanceId};
use mech_engine::__resident::{ActivationFacts, ResidentIntegrityMode, activate_external};
use mech_engine::{ProgramArtifact, decode_program_artifact_sections};
use mech_runtime::runtime::program::external::test_provider::{
    ExternalTestInputProvider, ExternalTestProviderTrace, ExternalTestSceneProvider,
    ExternalTestTransactionalProvider, SharedExternalTestProviderTrace,
};
use mech_runtime::{
    CapturedInputBatch, ExactRequirementAuthority, ResidentDurabilityPolicy,
    ResidentExternalCoordinator, ResidentExternalLimits, ResidentExternalTurnOutcome,
    ResidentTurnRecord, RuntimeBuilder, RuntimeResourceRegistry, resident_effect_ids_hash,
    resident_idempotency_keys_hash,
};
use sha2::{Digest, Sha256};

const TURNS: usize = 64;
const EFFECT_SOURCE: &str =
    include_str!("../../../tests/fixtures/resident-external/effect-source.mec");
const TRANSACTIONAL_SOURCE: &str =
    include_str!("../../../tests/fixtures/resident-external/transactional-source.mec");

#[derive(Clone, Copy, Debug)]
enum FixtureKind {
    Effect,
    Transactional,
}

#[derive(Debug)]
struct LaneResult {
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
fn resident_external_publication_and_replay_are_deterministic() -> MResult<()> {
    let catalog = mech_stdlib::source_catalog();
    let (probe_artifact, _) = compile_fixture(FixtureKind::Effect)?;
    let structural = external_structural_probe(&probe_artifact, &catalog)?;
    assert_eq!(structural.commit_runtime_calls, 0);
    assert_eq!(structural.legacy_journal_captures, 0);
    assert_eq!(structural.runtime_execution_transaction_constructions, 0);
    assert_eq!(structural.publication_stores_per_accepted_turn, 1);
    assert_eq!(structural.publication_stores_per_rejected_turn, 0);
    assert_eq!(structural.post_candidate_rejections, 1);
    assert_eq!(structural.rejected_receipt_appends, 1);
    assert_eq!(structural.rejected_outbox_batch_appends, 0);
    assert_eq!(structural.rejected_provider_preparation_attempts, 1);
    assert_eq!(structural.rejected_delivery_count, 0);
    assert_eq!(structural.effects_delivered_before_publication, 0);
    assert_eq!(structural.effects_delivered_for_rejected_turns, 0);
    for kind in [FixtureKind::Effect, FixtureKind::Transactional] {
        let (source, bytecode) = compile_fixture(kind)?;
        assert_eq!(source.revision(), bytecode.revision());
        assert_eq!(source.requirements(), bytecode.requirements());

        let source_result = run_lane(&source, &catalog, kind, 0, 1, true)?;
        let bytecode_result = run_lane(&bytecode, &catalog, kind, 0, 1, false)?;
        assert_equivalent(&source_result, &bytecode_result);

        let replay = run_replay(
            &source,
            &catalog,
            kind,
            &source_result.batches,
            &source_result.records,
        )?;
        assert_eq!(source_result.state_hash, replay.state_hash);
        assert_eq!(source_result.receipt_hash, replay.receipt_hash);
        assert_eq!(source_result.effect_batch_hash, replay.effect_batch_hash);
        assert_eq!(replay.reads, 0);
    }

    let (effect, _) = compile_fixture(FixtureKind::Effect)?;
    for (history, next_epoch) in [(0, 1), (1_000, 1), (0, u64::MAX - TURNS as u64 - 1)] {
        let result = run_lane(
            &effect,
            &catalog,
            FixtureKind::Effect,
            history,
            next_epoch,
            false,
        )?;
        assert_eq!(result.receipts, TURNS);
        assert_eq!(result.publication_stores, TURNS);
    }
    Ok(())
}

fn compile_fixture(kind: FixtureKind) -> MResult<(ProgramArtifact, ProgramArtifact)> {
    let trace = Arc::new(Mutex::new(ExternalTestProviderTrace::default()));
    let builder = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .resource_provider(Box::new(ExternalTestInputProvider::new(
            0.25,
            trace.clone(),
        )));
    let builder = match kind {
        FixtureKind::Effect => {
            builder.resource_provider(Box::new(ExternalTestSceneProvider::new(trace)))
        }
        FixtureKind::Transactional => {
            builder.resource_provider(Box::new(ExternalTestTransactionalProvider::new(trace)))
        }
    };
    let mut compiler = builder.build_compiler()?;
    let product = compiler.compile_source(match kind {
        FixtureKind::Effect => EFFECT_SOURCE,
        FixtureKind::Transactional => TRANSACTIONAL_SOURCE,
    })?;
    let parsed = ParsedProgram::from_bytes(product.bytecode())?;
    let decoded = decode_program_artifact_sections(&parsed.artifact).map_err(|error| {
        contract_error(&format!("decode resident external artifact: {error:?}"))
    })?;
    Ok((product.artifact().clone(), decoded))
}

fn providers(
    kind: FixtureKind,
    trace: SharedExternalTestProviderTrace,
) -> MResult<RuntimeResourceRegistry> {
    let mut providers = RuntimeResourceRegistry::new();
    providers.register_provider(Box::new(ExternalTestInputProvider::new(
        0.25,
        trace.clone(),
    )))?;
    match kind {
        FixtureKind::Effect => {
            providers.register_provider(Box::new(ExternalTestSceneProvider::new(trace)))?;
        }
        FixtureKind::Transactional => {
            providers.register_provider(Box::new(ExternalTestTransactionalProvider::new(trace)))?;
        }
    }
    Ok(providers)
}

fn external_structural_probe(
    artifact: &ProgramArtifact,
    catalog: &Arc<mech_core::FunctionCatalog>,
) -> MResult<StructuralResult> {
    let accepted_trace = Arc::new(Mutex::new(ExternalTestProviderTrace::default()));
    let accepted_providers = providers(FixtureKind::Effect, accepted_trace)?;
    let accepted_instance = activate_external(
        ReactiveInstanceId::new(910, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| {
        contract_error(&format!(
            "activate resident external structural turn: {error:?}"
        ))
    })?;
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

    let rejected_trace = Arc::new(Mutex::new(ExternalTestProviderTrace::default()));
    let mut rejected_providers = RuntimeResourceRegistry::new();
    rejected_providers.register_provider(Box::new(ExternalTestInputProvider::new(
        0.25,
        rejected_trace.clone(),
    )))?;
    rejected_providers.register_provider(Box::new(
        ExternalTestSceneProvider::with_preparation_failures(rejected_trace.clone(), 1),
    ))?;
    let rejected_instance = activate_external(
        ReactiveInstanceId::new(911, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| {
        contract_error(&format!(
            "activate resident external rejected probe: {error:?}"
        ))
    })?;
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
    let rejected_trace = rejected_trace
        .lock()
        .expect("resident external rejected trace")
        .clone();
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
    let trace = Arc::new(Mutex::new(ExternalTestProviderTrace::default()));
    let providers = providers(kind, trace.clone())?;
    let instance = activate_external(
        ReactiveInstanceId::new(900, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| {
        contract_error(&format!(
            "activate resident external artifact: {error:?}; nodes={:?}",
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
        let mut trace = trace.lock().expect("external test provider trace");
        trace.read_calls = 0;
        trace.prepared.clear();
        trace.delivered = 0;
        trace.applied = 0;
    }
    let receipt_start = coordinator.receipts().count();
    let input_start = coordinator.input_facts().count();
    let structural_start = coordinator.structural_probe();
    for _ in 0..TURNS {
        require_accepted(coordinator.execute_turn()?)?;
    }
    let trace_snapshot = trace.lock().expect("external test provider trace").clone();
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
    let state_hash = receipts
        .last()
        .expect("resident external receipt")
        .state_hash;
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
    let trace = Arc::new(Mutex::new(ExternalTestProviderTrace::default()));
    let instance = activate_external(
        ReactiveInstanceId::new(900, 0),
        artifact,
        catalog,
        &ActivationFacts::default(),
        ResidentIntegrityMode::Checked,
    )
    .map_err(|error| {
        contract_error(&format!(
            "activate resident external replay artifact: {error:?}"
        ))
    })?;
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
    assert_eq!(batches.len(), records.len());
    for (batch, record) in batches.iter().zip(records) {
        require_accepted(coordinator.execute_replay_batch(Some(batch), record)?)?;
    }
    let receipts = coordinator
        .receipts()
        .map(|(_, record)| record.body.clone())
        .collect::<Vec<_>>();
    let trace_snapshot = trace.lock().expect("external test provider trace").clone();
    let probe = coordinator.structural_probe();
    Ok(LaneResult {
        state_hash: receipts
            .last()
            .expect("resident external replay receipt")
            .state_hash,
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

fn assert_equivalent(left: &LaneResult, right: &LaneResult) {
    assert_eq!(left.state_hash, right.state_hash);
    assert_eq!(left.receipt_hash, right.receipt_hash);
    assert_eq!(left.effect_batch_hash, right.effect_batch_hash);
    assert_eq!(left.effect_id_hash, right.effect_id_hash);
    assert_eq!(left.idempotency_key_hash, right.idempotency_key_hash);
    assert_eq!(left.receipts, right.receipts);
    assert_eq!(left.publication_stores, right.publication_stores);
    assert_eq!(left.outbox_batch_appends, right.outbox_batch_appends);
}

fn require_accepted(outcome: ResidentExternalTurnOutcome) -> MResult<()> {
    if matches!(outcome, ResidentExternalTurnOutcome::Accepted { .. }) {
        Ok(())
    } else {
        Err(contract_error(&format!(
            "resident external controlled turn was not accepted: {outcome:?}"
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

fn contract_error(message: &str) -> MechError {
    MechError::new(
        mech_core::GenericError {
            msg: message.to_owned(),
        },
        None,
    )
}
