//! Actual selected original/decoded Matrix[String] gather, no detached plan.
use super::*;
use crate::native_artifact::{
    NativeCollectionInvocationError, NativeCollectionPrimitive, NativePreparedLibraryCall,
    NativePreparedLibraryCallCache, NativeProgramLibraryTemplate,
    native_collection_operation_template, prepare_native_program_library_call,
};
use crate::{CanonicalSourceFrontend, ProgramArtifact};
use mech_core::{
    FunctionCatalog, FunctionCatalogBuilder, ManagedMemoryBudget, MemoryBudgetLimits,
    ValueDataDraft, ValueDraft,
    snapshot::{SequenceView, SnapshotValidationContext},
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

const COUNT: usize = 65_537;

fn word(index: usize) -> &'static str {
    match index % 4 {
        0 => "",
        1 => "é",
        2 => "a\0b",
        _ => "🧑‍🚀",
    }
}

fn variants() -> [ProgramArtifact; 2] {
    let source = "words := input<[string]>\nwords[positions<[u64]>]\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x454c454d), Revision(0), source).unwrap(),
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

fn catalog() -> FunctionCatalog {
    let mut builder = FunctionCatalogBuilder::new();
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
) -> (
    NativePreparedLibraryCall,
    [Option<Value>; 2],
    [Option<Value>; 2],
) {
    let catalog = catalog();
    let template = NativeProgramLibraryTemplate::from_root(
        artifact
            .nodes()
            .iter()
            .find_map(|node| {
                native_collection_operation_template(artifact, node.node, &catalog)
                    .ok()
                    .filter(|template| {
                        template.primitive() == NativeCollectionPrimitive::MatrixRangeAccess
                    })
            })
            .unwrap(),
    );
    let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
        panic!("actual declared gather")
    };
    let source_shape =
        mech_core::shape_for_resolved_extents(&template.inputs()[0], &[1, COUNT as u64]).unwrap();
    let source = ValueDraft {
        schema: contract.inputs[0].schema,
        shape_values: source_shape.parameter_values().to_vec().into_boxed_slice(),
        data: ValueDataDraft::Matrix(
            (0..COUNT)
                .map(|index| ValueDataDraft::String(word(index).into()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let selector = |positions: Vec<u64>| {
        let shape = mech_core::shape_for_resolved_extents(
            &template.inputs()[1],
            &[1, positions.len() as u64],
        )
        .unwrap();
        admitted(
            ValueDraft {
                schema: contract.inputs[1].schema,
                shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                data: ValueDataDraft::Matrix(
                    positions
                        .into_iter()
                        .map(ValueDataDraft::U64)
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap(),
            memory,
        )
    };
    let source = admitted(source, memory);
    let small = [Some(source.clone()), Some(selector(vec![1, COUNT as u64]))];
    let large = [
        Some(source),
        Some(selector((1..=COUNT as u64).rev().collect())),
    ];
    let prepared = prepare_native_program_library_call(artifact, &template, &catalog).unwrap();
    (prepared, small, large)
}

fn limits(elements: u64) -> MemoryBudgetLimits {
    let mut limits = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    limits.max_output_elements = Some(elements);
    limits.max_output_bytes = Some(128 * 1024 * 1024);
    limits.max_temporary_bytes = Some(128 * 1024 * 1024);
    limits.max_cloned_bytes = Some(128 * 1024 * 1024);
    limits.max_retained_nodes = Some(1_000_000);
    limits.max_compute_work = Some(512 * 1024 * 1024);
    limits.max_comparison_work = Some(128 * 1024 * 1024);
    limits
}

fn assert_output(value: &Value, large: bool) {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("genuine Matrix result")
    };
    let SequenceView::String(strings) = matrix.elements() else {
        panic!("packed String result")
    };
    if large {
        assert_eq!(strings.len(), COUNT);
        for (index, text) in strings.iter().enumerate() {
            assert_eq!(text.as_ref(), word(COUNT - index - 1));
        }
    } else {
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].as_ref(), word(0));
        assert_eq!(strings[1].as_ref(), word(COUNT - 1));
    }
}

#[test]
fn selected_matrix_string_gather_obeys_current_policy_above_resident_default() {
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let (prepared, small, large) = fixture(&artifact, &memory);
        let fixture_bytes = memory.used_bytes();
        for cap in [COUNT as u64 + 1, COUNT as u64, COUNT as u64 - 1] {
            let work =
                NativeLibraryWorkBudget::new(4_000_000_000, 128 * 1024 * 1024, &memory).unwrap();
            let mut cache = NativePreparedLibraryCallCache::default();
            let mut output = None;
            prepared
                .invoke_current_cached(&mut cache, &small, &mut output, limits(cap), &work)
                .unwrap();
            assert_output(output.as_ref().unwrap(), false);
            drop(cache);
            let prior = output.as_ref().unwrap().clone();
            let baseline = memory.used_bytes();
            let mut cache = NativePreparedLibraryCallCache::default();
            OUTPUT_ELEMENT_PREFLIGHT.with(|observed| observed.set((0, 0, 0)));
            let before = work.used();
            let result =
                prepared.invoke_current_cached(&mut cache, &large, &mut output, limits(cap), &work);
            let observed = OUTPUT_ELEMENT_PREFLIGHT.with(std::cell::Cell::get);
            assert_eq!(
                observed,
                (1, COUNT as u64, cap),
                "must reach the maintained count preflight, not an earlier unrelated denial"
            );
            if cap >= COUNT as u64 {
                result.unwrap();
                assert_output(output.as_ref().unwrap(), true);
            } else {
                assert!(
                    matches!(result, Err(NativeCollectionInvocationError::Kernel(
                    ResidentKernelError::ResourceLimit {
                        phase: mech_core::ResidentResourcePhase::Admission,
                        dimension: MemoryBudgetDimension::OutputElements,
                        required,
                        limit,
                    }
                )) if required == COUNT as u64 && limit == cap)
                );
                assert!(output.as_ref().unwrap().shares_frozen_storage(&prior));
                assert_output(output.as_ref().unwrap(), false);
            }
            assert!(
                work.used().0 > before.0,
                "failed attempt never refunds accepted setup/measurement work"
            );
            drop(cache);
            if cap < COUNT as u64 {
                assert_eq!(
                    memory.used_bytes(),
                    baseline,
                    "denied candidate releases all new capacity"
                );
            }
            let mut recovery = NativePreparedLibraryCallCache::default();
            prepared
                .invoke_current_cached(&mut recovery, &small, &mut output, limits(cap), &work)
                .unwrap();
            assert_output(output.as_ref().unwrap(), false);
            drop((recovery, prior, output, work));
            assert_eq!(memory.used_bytes(), fixture_bytes);
        }
        drop((prepared, small, large));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn absent_current_turn_keeps_historical_default_without_a_permit() {
    assert!(with_active_turn_plan(|active| active.is_none()));
    assert_eq!(check_output_element_count(0), Ok(()));
    assert_eq!(
        check_output_element_count(MAX_RESIDENT_OUTPUT_ELEMENTS),
        Ok(())
    );
    assert_eq!(
        check_output_element_count(MAX_RESIDENT_OUTPUT_ELEMENTS + 1),
        Err(ResidentKernelError::InvalidShape)
    );
    assert!(with_active_turn_plan(|active| active.is_none()));
}
