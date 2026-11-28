//! Real selected original/decoded leaves: history reuse, finite eviction and identity.
use super::*;
use crate::{CanonicalSourceFrontend, ProgramArtifact};
use crate::resident::general::native_collection_calls::invocation::data_footprint_tests::maximum_tests::maximum_limits;
use mech_core::{ManagedMemoryBudget, ValueDraft, snapshot::{F64Bits, SnapshotValidationContext}};
use mech_syntax::document::{AstNode,DocumentId,DocumentSyntax,ParseConfig,Revision,TextSnapshot,parse_canonical_document};
use std::collections::BTreeMap;

fn artifacts() -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5348415045), Revision(0), "input<[f64]>'\n").unwrap(),
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
fn prepared(artifact: &ProgramArtifact) -> NativePreparedLibraryCall {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    let catalog = builder.build().unwrap();
    let template = artifact
        .nodes()
        .iter()
        .find_map(|node| {
            super::super::native_collection_operation_template(artifact, node.node, &catalog)
                .ok()
                .filter(|t| {
                    t.primitive()
                        == crate::native_artifact::NativeCollectionPrimitive::MatrixTranspose
                })
        })
        .unwrap();
    let input_parameters = template
        .inputs()
        .iter()
        .map(|schema| vec![1; schema.dimension_parameters().len()])
        .collect::<Vec<_>>();
    let references = input_parameters
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let output_parameters = vec![0; template.output().dimension_parameters().len()];
    let call = super::super::bind_native_collection_operation(
        artifact,
        template.node(),
        &catalog,
        &references,
        &output_parameters,
        maximum_limits(u64::MAX),
    )
    .unwrap();
    prepare_native_program_library_call(artifact, call.template(), &catalog).unwrap()
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    rows: u64,
    columns: u64,
    offset: u64,
) -> [Option<Value>; 1] {
    let input = &artifact.inputs()[0];
    let schema = artifact.schemas().get(input.schema).unwrap();
    let shape = mech_core::shape_for_resolved_extents(schema, &[rows, columns]).unwrap();
    let value = ValueDraft {
        schema: input.schema,
        shape_values: shape.parameter_values().into(),
        data: mech_core::ValueDataDraft::Matrix(
            (0..rows * columns)
                .map(|i| mech_core::ValueDataDraft::F64(F64Bits::from_f64((i + offset) as f64)))
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut capacity = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    [Some(value.into_memory_budget(&mut capacity).unwrap())]
}
fn draft(value: &Option<Value>) -> mech_core::ValueDataDraft {
    value.as_ref().unwrap().canonical_data_draft().unwrap()
}
fn slots(cache: &NativePreparedLibraryCallCache) -> usize {
    let mut next = cache.binding.as_deref();
    let mut count = 0;
    while let Some(binding) = next {
        count += 1;
        assert!(count <= 64);
        next = binding.previous.as_deref();
    }
    assert_eq!(
        count,
        cache.binding.as_ref().map_or(0, |b| b.history_len + 1)
    );
    count
}

#[test]
fn numeric_shape_history_reuses_checked_bindings_and_plans_with_changed_payloads() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(8 * 1024 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let limits = maximum_limits(memory.limit_bytes());
        let prepared = prepared(&artifact);
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut seen = BTreeMap::new();
        for pass in 0..3 {
            for columns in 1..=24 {
                let inputs = arguments(&artifact, &memory, 2, columns, pass * 100);
                let before = draft(&inputs[0]);
                let mut output = None;
                prepared
                    .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                    .unwrap();
                let binding = cache.binding.as_ref().unwrap();
                let NativePlanCacheState::Cached(plans) = &binding.plans else {
                    panic!("actual maintained turn-plan cache");
                };
                if pass == 0 {
                    seen.insert(
                        columns,
                        (
                            (binding.call.as_ref().unwrap() as *const NativeCollectionCall)
                                as usize,
                            std::rc::Rc::downgrade(plans),
                        ),
                    );
                } else {
                    let (pointer, weak) = &seen[&columns];
                    assert_eq!(
                        (binding.call.as_ref().unwrap() as *const NativeCollectionCall) as usize,
                        *pointer
                    );
                    assert!(std::rc::Rc::ptr_eq(
                        plans,
                        &weak
                            .upgrade()
                            .expect("the exact shape plan remained retained")
                    ));
                }
                let expected = (0..columns)
                    .flat_map(|column| {
                        (0..2).map(move |row| {
                            mech_core::ValueDataDraft::F64(F64Bits::from_f64(
                                (row * columns + column + pass * 100) as f64,
                            ))
                        })
                    })
                    .collect();
                assert_eq!(draft(&output), mech_core::ValueDataDraft::Matrix(expected));
                assert_eq!(draft(&inputs[0]), before);
                let demand = cache.retained_metadata_demand(&work).unwrap();
                assert!(
                    demand.turn_peak_bytes > 0 && demand.retained_nodes >= slots(&cache) as u64
                );
            }
        }
        assert_eq!(slots(&cache), 24);
        drop((seen, cache, prepared, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn numeric_shape_history_is_bounded_and_rejects_foreign_identity_ledgers_and_policy() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(8 * 1024 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let limits = maximum_limits(memory.limit_bytes());
        let prepared = prepared(&artifact);
        let mut cache = NativePreparedLibraryCallCache::default();
        for columns in 1..=70 {
            let inputs = arguments(&artifact, &memory, 1, columns, 0);
            let mut output = None;
            prepared
                .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                .unwrap();
            assert_eq!(slots(&cache), usize::try_from(columns.min(64)).unwrap());
        }
        let inputs = arguments(&artifact, &memory, 1, 70, 100);
        let mut output = None;
        prepared
            .invoke_current(&inputs, &mut output, limits, &work)
            .unwrap();
        let before = draft(&output);
        let foreign = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        assert!(
            prepared
                .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &foreign)
                .is_err()
        );
        assert_eq!(draft(&output), before);
        let mut builder = mech_core::FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut builder).unwrap();
        let catalog = builder.build().unwrap();
        let alternate =
            prepare_native_program_library_call(&artifact, prepared.template(), &catalog).unwrap();
        assert!(
            alternate
                .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                .is_err()
        );
        let mut changed = limits;
        changed.max_output_elements = Some(u64::MAX - 1);
        assert!(
            prepared
                .invoke_current_cached(&mut cache, &inputs, &mut output, changed, &work)
                .is_err()
        );
        assert_eq!(draft(&output), before);
        prepared
            .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
            .unwrap();
        assert_eq!(slots(&cache), 64);
        // Only private test mutation: fail closed without usize wrap/panic.
        let count = cache.binding.as_ref().unwrap().history_len;
        cache.binding.as_mut().unwrap().history_len = usize::MAX;
        assert!(cache.retained_metadata_demand(&work).is_err());
        cache.binding.as_mut().unwrap().history_len = count;
        drop((output, inputs, alternate, foreign, cache, prepared, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn numeric_shape_history_retires_under_current_pressure_without_starving_the_ordinary_call() {
    for artifact in artifacts() {
        let memory = ManagedMemoryBudget::new(512 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let limits = maximum_limits(memory.limit_bytes());
        let prepared = prepared(&artifact);
        let mut cache = NativePreparedLibraryCallCache::default();
        for columns in 1..=8 {
            let inputs = arguments(&artifact, &memory, 2, columns, 0);
            let mut output = None;
            prepared
                .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                .unwrap();
        }
        assert_eq!(slots(&cache), 8);
        let inputs = arguments(&artifact, &memory, 256, 256, 0);
        let available = memory.limit_bytes() - memory.used_bytes();
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        let trial = |allowance| {
            let padding = memory.reserve_capacity(available - allowance).unwrap();
            let mut output = None;
            let ok = prepared
                .invoke_current(&inputs, &mut output, limits, &work)
                .is_ok();
            drop((output, padding));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            ok
        };
        let (mut low, mut high) = (0, available);
        assert!(trial(high));
        while low < high {
            let middle = low + (high - low) / 2;
            if trial(middle) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        assert!(low > 0 && trial(low) && !trial(low - 1));
        let padding = memory.reserve_capacity(available - low).unwrap();
        let before = draft(&inputs[0]);
        let mut output = None;
        prepared
            .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
            .unwrap();
        assert_eq!(
            slots(&cache),
            0,
            "all optional bindings retired before ordinary replay"
        );
        assert_eq!(draft(&inputs[0]), before);
        drop((output, padding));
        prepared
            .invoke_current_cached(&mut cache, &inputs, &mut None, limits, &work)
            .unwrap();
        assert_eq!(slots(&cache), 1);
        drop((inputs, cache, prepared, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
