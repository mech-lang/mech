//! Genuine selected Matrix[String] calls; no detached plan or cached permit.
use super::super::{bind_native_collection_operation, native_collection_operation_template};
use super::*;
use crate::{CanonicalSourceFrontend, ProgramArtifact};
use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft, ValueDraft};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn variants() -> [ProgramArtifact; 2] {
    let source = "words := input<[string]>\nwords[position<u64>]\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x44415441), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let artifact = CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    [artifact, decoded]
}

fn catalog() -> mech_core::FunctionCatalog {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    builder.build().unwrap()
}

fn admitted(value: Value, memory: &ManagedMemoryBudget) -> Value {
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    value.into_memory_budget(&mut reservation).unwrap()
}

fn fixture(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
) -> (NativeCollectionCall, [Option<Value>; 2]) {
    let catalog = catalog();
    let template = artifact.nodes().iter().find_map(|node| {
        native_collection_operation_template(artifact, node.node, &catalog).ok()
            .filter(|t| t.primitive() == crate::resident::native_collection_semantics::NativeCollectionPrimitive::MatrixScalarAccess)
    }).unwrap();
    let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
        panic!("declared access")
    };
    let source_id = contract.inputs[0].schema;
    let selector_id = contract.inputs[1].schema;
    let source_shape =
        mech_core::shape_for_resolved_extents(template.inputs().first().unwrap(), &[1, 64])
            .unwrap();
    let source = ValueDraft {
        schema: source_id,
        shape_values: source_shape.parameter_values().to_vec().into_boxed_slice(),
        data: ValueDataDraft::Matrix(
            (0..64)
                .map(|i| {
                    ValueDataDraft::String(
                        match i % 3 {
                            0 => "é",
                            1 => "a\0b",
                            _ => "🧑‍🚀",
                        }
                        .into(),
                    )
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let selector = ValueDraft {
        schema: selector_id,
        shape_values: Box::new([]),
        data: ValueDataDraft::U64(2),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let inputs = [
        Some(admitted(source, memory)),
        Some(admitted(selector, memory)),
    ];
    let parameters = inputs
        .iter()
        .map(|v| v.as_ref().unwrap().shape().parameter_values())
        .collect::<Vec<_>>();
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_compute_work = Some(128 * 1024 * 1024);
    limits.max_temporary_bytes = Some(128 * 1024 * 1024);
    let call = bind_native_collection_operation(
        artifact,
        template.node(),
        &catalog,
        &parameters,
        &[],
        limits,
    )
    .unwrap();
    (call, inputs)
}

fn cached(state: &NativePlanCacheState) -> Rc<NativeTurnPlanCache> {
    let NativePlanCacheState::Cached(cache) = state else {
        panic!("actual selected call must retain bounded metadata")
    };
    cache.clone()
}

#[test]
fn immutable_data_memo_reuses_actual_source_not_current_values_or_publication() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let (call, mut inputs) = fixture(&artifact, &memory);
        let baseline = memory.used_bytes();
        let mut state = NativePlanCacheState::Cold;
        let mut output = None;
        call.invoke_cached(&mut state, &inputs, &mut output, &work)
            .unwrap();
        assert!(
            matches!(output.as_ref().unwrap().data(), ValueData::String(s) if s.as_ref() == "a\0b")
        );
        let cache = cached(&state);
        let first = cache.data_observations();
        assert_eq!(first.1, 2);
        assert_eq!(first.2, 0);
        assert!(first.0.1 > 64, "real source and selector visitors ran");
        assert!(
            first.0.2 > 0,
            "actual maintained numeric source used the SAME ready memo"
        );
        drop(output.take());
        call.invoke_cached(&mut state, &inputs, &mut output, &work)
            .unwrap();
        let second = cache.data_observations();
        assert_eq!(
            second.0.1, first.0.1,
            "no immutable data chunk revisited on clone hit"
        );
        assert_eq!(second.0.0, first.0.0 + 2);
        assert!(second.0.2 > first.0.2);
        // A different owner claim of the SAME data is accepted only after
        // fresh full wrapper/account checks, not by the weak token alone.
        let raw = inputs[0].as_ref().unwrap().clone();
        let mut reservation = memory
            .reserve_capacity(
                raw.memory_budget_required_bytes().unwrap()
                    + Value::memory_budget_claim_metadata_bytes(),
            )
            .unwrap();
        inputs[0] = Some(
            raw.clone()
                .into_independent_memory_budget(&mut reservation)
                .unwrap(),
        );
        drop(reservation);
        drop(output.take());
        call.invoke_cached(&mut state, &inputs, &mut output, &work)
            .unwrap();
        assert_eq!(cache.data_observations().0.1, first.0.1);
        // Equal bytes from a different root are a genuine cold measurement.
        let (_, replacement) = fixture(&artifact, &memory);
        assert!(
            !replacement[0]
                .as_ref()
                .unwrap()
                .shares_frozen_storage(inputs[0].as_ref().unwrap())
        );
        drop(output.take());
        call.invoke_cached(&mut state, &replacement, &mut output, &work)
            .unwrap();
        assert!(cache.data_observations().0.1 > first.0.1);
        let prior = output.as_ref().unwrap().clone();
        let held = memory.used_bytes();
        let blocker = memory
            .reserve_capacity(memory.limit_bytes() - held)
            .unwrap();
        assert!(
            call.invoke_cached(&mut state, &inputs, &mut output, &work)
                .is_err()
        );
        assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
        drop(blocker);
        assert_eq!(memory.used_bytes(), held);
        drop((prior, output, replacement, inputs, cache, state, call, raw));
        // All weak shell capacity and real claims retire with the cache.
        assert!(baseline > memory.used_bytes());
        drop(work);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn data_memo_rejects_unpaid_partial_and_later_denied_measurement_transactions() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(1_000_000_000, 1_000_000, &memory).unwrap();
        let (call, inputs) = fixture(&artifact, &memory);
        let mut state = NativePlanCacheState::Cold;
        let mut output = None;
        call.invoke_cached(&mut state, &inputs, &mut output, &work)
            .unwrap();
        let cache = cached(&state);
        assert!(
            cache.begin_data_measurements().is_err(),
            "absent scope is not paid authority"
        );
        let (_, fresh) = fixture(&artifact, &memory);
        let source = fresh[0].as_ref().unwrap();
        let owner = source.schemas().unwrap();
        work.with_scope(|| {
            let transaction = cache.begin_data_measurements().unwrap();
            assert!(
                transaction
                    .measure(0, source, source.schema_key(), |_, _| Ok(()))
                    .is_err(),
                "even a completed visitor cannot install unpaid reusable metadata"
            );
        })
        .unwrap();
        assert_eq!(cache.data_observations().2, 0);
        work.with_scope(|| {
            let transaction = cache.begin_data_measurements().unwrap();
            let mut demand = ResourceDemand::default();
            let mut chunks = 0;
            assert!(
                transaction
                    .measure(0, source, source.schema_key(), |_, n| {
                        chunks += 1;
                        demand.work.compute += n;
                        budget::observe_control_work(demand).map_err(measurement_error)?;
                        if chunks == 3 {
                            Err(MemoryPlanError::DescriptorMismatch)
                        } else {
                            Ok(())
                        }
                    })
                    .is_err()
            );
        })
        .unwrap();
        assert_eq!(
            cache.data_observations().2,
            0,
            "partial walk must never persist"
        );
        let ready = cache.data_observations().1;
        work.with_scope(|| {
            let transaction = cache.begin_data_measurements().unwrap();
            let mut demand = ResourceDemand::default();
            transaction
                .measure(0, source, source.schema_key(), |_, n| {
                    demand.work.compute += n;
                    budget::observe_control_work(demand).map_err(measurement_error)
                })
                .unwrap();
            assert_eq!(cache.data_observations().2, 1);
            // Simulate a later input/publication/region/current-budget failure
            // at the transaction boundary: do NOT commit completed data.
            drop(transaction);
        })
        .unwrap();
        assert_eq!(cache.data_observations().1, ready);
        assert_eq!(cache.data_observations().2, 0);
        drop((output, state, cache, inputs, fresh, owner, call, work));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[path = "native_data_footprint_boundary_tests.rs"]
mod boundary_tests;
