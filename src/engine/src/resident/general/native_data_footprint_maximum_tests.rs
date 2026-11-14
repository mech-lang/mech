//! Maximal call limits remain finite limits; cache selection uses actual demand.
//! These are the maintained Matrix[String] selector, binder, planner and kernel.
use super::*;
use crate::resident::general::native_collection_calls::{
    NativePreparedLibraryCall, NativePreparedLibraryCallCache, prepare_native_program_library_call,
};

pub(in crate::resident::general::native_collection_calls) fn maximum_limits(
    temporary: u64,
) -> mech_core::MemoryBudgetLimits {
    mech_core::MemoryBudgetLimits {
        max_output_elements: Some(u64::MAX),
        max_output_bytes: Some(u64::MAX),
        max_temporary_bytes: Some(temporary),
        max_cloned_bytes: Some(u64::MAX),
        max_retained_nodes: Some(u64::MAX),
        max_comparison_work: Some(u64::MAX),
        max_compute_work: Some(u64::MAX),
        max_scalar_instructions: Some(u64::MAX),
        max_transfer_bytes: Some(u64::MAX),
        max_storage_buffer_bytes: Some(u64::MAX),
        max_storage_bindings: Some(u32::MAX),
    }
}

pub(in crate::resident::general::native_collection_calls) fn setup(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    limits: mech_core::MemoryBudgetLimits,
    length: u64,
) -> (
    NativeCollectionCall,
    NativePreparedLibraryCall,
    [Option<Value>; 2],
) {
    let (finite, inputs) = fixture_with_len(artifact, memory, length);
    let parameters = inputs
        .iter()
        .map(|value| value.as_ref().unwrap().shape().parameter_values())
        .collect::<Vec<_>>();
    let call = bind_native_collection_operation(
        artifact,
        finite.template().origin().node(),
        &catalog(),
        &parameters,
        &[],
        limits,
    )
    .unwrap();
    let prepared =
        prepare_native_program_library_call(artifact, call.template(), &catalog()).unwrap();
    (call, prepared, inputs)
}

fn drafts(inputs: &[Option<Value>]) -> Vec<Option<ValueDataDraft>> {
    inputs
        .iter()
        .map(|value| {
            value
                .as_ref()
                .map(|value| value.canonical_data_draft().unwrap())
        })
        .collect()
}

fn string(output: &Option<Value>) -> &str {
    let ValueData::String(value) = output.as_ref().unwrap().data() else {
        panic!("selected string result")
    };
    value
}

fn assert_prior(output: &Option<Value>, prior: &Option<Value>) {
    assert_eq!(drafts(&[output.clone()]), drafts(&[prior.clone()]));
    if let Some(prior) = prior {
        assert!(output.as_ref().unwrap().shares_frozen_storage(prior));
    }
}

#[test]
fn maximum_and_near_maximum_temporary_limits_select_both_cache_levels() {
    for artifact in variants() {
        for temporary in [u64::MAX, u64::MAX - 1] {
            let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let limits = maximum_limits(temporary);
            let (call, prepared, inputs) = setup(&artifact, &memory, limits, 64);
            let aliases = inputs.clone();
            let before = drafts(&inputs);
            let mut plan = NativePlanCacheState::Cold;
            let mut binding = NativePreparedLibraryCallCache::default();
            let mut output = None;
            for _ in 0..3 {
                call.invoke_cached(&mut plan, &inputs, &mut output, &work)
                    .unwrap();
                assert!(
                    matches!(plan, NativePlanCacheState::Cached(_)),
                    "MAX is a policy bound, not an allocation request or reason to disable the plan cache"
                );
                assert_eq!(string(&output), "a\0b");
                prepared
                    .invoke_current_cached(&mut binding, &inputs, &mut output, limits, &work)
                    .unwrap();
                let retained = binding.retained_metadata_demand(&work).unwrap();
                assert!(
                    retained.turn_peak_bytes > 0 && retained.retained_nodes > 1,
                    "the exact current binding must be retained under maximal limits"
                );
                assert_eq!(string(&output), "a\0b");
                assert_eq!(drafts(&inputs), before);
                assert_eq!(drafts(&aliases), before);
            }
            drop((output, binding, plan, inputs, aliases, call, prepared, work));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn cache_capacity_overflow_declines_before_allocation_or_publication_and_keeps_debit() {
    use crate::resident::general::native_collection_calls::cache_capacity;
    for artifact in variants() {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let limits = maximum_limits(u64::MAX);
        let (call, prepared, inputs) = setup(&artifact, &memory, limits, 8);
        let mut output = None;
        prepared
            .invoke_current(&inputs, &mut output, limits, &work)
            .unwrap();
        let prior = output.clone();
        let before = drafts(&inputs);
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        for (bytes, nodes, additional_bytes, additional_nodes) in [
            (u64::MAX, 0, 0, 0),
            (0, u64::MAX, 0, 0),
            (0, 0, u64::MAX, 0),
            (0, 0, 0, u64::MAX),
        ] {
            let spent = work.used();
            assert!(
                !cache_capacity::admits(
                    call.template(),
                    artifact.schemas(),
                    &inputs,
                    &output,
                    limits,
                    &work,
                    bytes,
                    nodes,
                    additional_bytes,
                    additional_nodes
                )
                .unwrap()
            );
            assert!(work.used().0 > spent.0);
            assert_prior(&output, &prior);
            assert_eq!(drafts(&inputs), before);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        }
        drop((output, prior, inputs, prepared, call, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn maximum_string_matrix_length_selects_both_caches_for_empty_and_changing_shapes() {
    let parsed = parse_canonical_document(
        TextSnapshot::new(
            DocumentId(0x4d41584c),
            Revision(0),
            "words := input<[string]>\nsequence/length(words)\n",
        )
        .unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let original = CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let limits = maximum_limits(u64::MAX);
        let catalog = catalog();
        let template = artifact
            .nodes()
            .iter()
            .find_map(|node| {
                native_collection_operation_template(&artifact, node.node, &catalog)
                    .ok()
                    .filter(|template| {
                        template.primitive()
                            == crate::native_artifact::NativeCollectionPrimitive::SequenceLength
                    })
            })
            .unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!("declared length")
        };
        let program =
            crate::native_artifact::NativeProgramLibraryTemplate::from_root(template.clone());
        let prepared = prepare_native_program_library_call(&artifact, &program, &catalog).unwrap();
        let mut cache = NativePreparedLibraryCallCache::default();
        let mut output = None;
        for length in [0, 3, 64, 0] {
            let shape =
                mech_core::shape_for_resolved_extents(&template.inputs()[0], &[1, length]).unwrap();
            let value = ValueDraft {
                schema: contract.inputs[0].schema,
                shape_values: shape.parameter_values().into(),
                data: ValueDataDraft::Matrix(
                    (0..length)
                        .map(|_| ValueDataDraft::String("é\0🧑‍🚀".into()))
                        .collect(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let inputs = [Some(admitted(value, &memory))];
            let before = drafts(&inputs);
            let parameters = [inputs[0].as_ref().unwrap().shape().parameter_values()];
            let call = bind_native_collection_operation(
                &artifact,
                template.node(),
                &catalog,
                &parameters,
                &[],
                limits,
            )
            .unwrap();
            let mut plan = NativePlanCacheState::Cold;
            call.invoke_cached(&mut plan, &inputs, &mut output, &work)
                .unwrap();
            assert!(matches!(plan, NativePlanCacheState::Cached(_)));
            drop(plan);
            for _ in 0..2 {
                prepared
                    .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                    .unwrap();
                assert!(
                    matches!(output.as_ref().unwrap().data(), ValueData::U64(actual) if *actual == length)
                );
                assert!(
                    cache
                        .retained_metadata_demand(&work)
                        .unwrap()
                        .retained_nodes
                        > 1
                );
                assert_eq!(drafts(&inputs), before);
            }
        }
        drop((output, cache, prepared, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn unsupported_unsigned_length_keeps_the_complete_ceiling_cache_route_at_maximum() {
    use crate::resident::general::native_collection_calls::headroom;
    let parsed = parse_canonical_document(
        TextSnapshot::new(
            DocumentId(0x4d415855),
            Revision(0),
            "items := input<[u8]>\nsequence/length(items)\n",
        )
        .unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean());
    let original = CanonicalSourceFrontend
        .compile_document(&DocumentSyntax::cast(parsed.syntax()).unwrap())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let limits = maximum_limits(u64::MAX);
        let catalog = catalog();
        let template = artifact
            .nodes()
            .iter()
            .find_map(|node| {
                native_collection_operation_template(&artifact, node.node, &catalog)
                    .ok()
                    .filter(|template| {
                        template.primitive()
                            == crate::native_artifact::NativeCollectionPrimitive::SequenceLength
                    })
            })
            .unwrap();
        let mech_core::ResolvedOperationContract::Declared(contract) = template.contract() else {
            panic!("declared length")
        };
        let program =
            crate::native_artifact::NativeProgramLibraryTemplate::from_root(template.clone());
        assert!(!headroom::supports_template(&program));
        let prepared = prepare_native_program_library_call(&artifact, &program, &catalog).unwrap();
        let shape = mech_core::shape_for_resolved_extents(&template.inputs()[0], &[1, 3]).unwrap();
        let value = ValueDraft {
            schema: contract.inputs[0].schema,
            shape_values: shape.parameter_values().into(),
            data: ValueDataDraft::Matrix(
                vec![
                    ValueDataDraft::U8(1),
                    ValueDataDraft::U8(2),
                    ValueDataDraft::U8(3),
                ]
                .into(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let inputs = [Some(admitted(value, &memory))];
        let aliases = inputs.clone();
        let before = drafts(&inputs);
        let parameters = [inputs[0].as_ref().unwrap().shape().parameter_values()];
        let call = bind_native_collection_operation(
            &artifact,
            template.node(),
            &catalog,
            &parameters,
            &[],
            limits,
        )
        .unwrap();
        let mut plan = NativePlanCacheState::Cold;
        let mut binding = NativePreparedLibraryCallCache::default();
        let mut output = None;
        for _ in 0..2 {
            call.invoke_cached(&mut plan, &inputs, &mut output, &work)
                .unwrap();
            assert!(matches!(plan, NativePlanCacheState::Uncached));
            prepared
                .invoke_current_cached(&mut binding, &inputs, &mut output, limits, &work)
                .unwrap();
            assert_eq!(
                binding
                    .retained_metadata_demand(&work)
                    .unwrap()
                    .retained_nodes,
                0
            );
            assert!(matches!(output.as_ref().unwrap().data(), ValueData::U64(3)));
            assert_eq!(drafts(&inputs), before);
            assert_eq!(drafts(&aliases), before);
        }
        drop((output, binding, plan, inputs, aliases, call, prepared, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn maximum_cached_binding_reads_fresh_owners_and_keeps_error_publications() {
    for artifact in variants() {
        let limits = maximum_limits(u64::MAX);
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let (call, prepared, mut inputs) = setup(&artifact, &memory, limits, 64);
        let original = inputs.clone();
        let original_drafts = drafts(&original);
        let mut binding = NativePreparedLibraryCallCache::default();
        let mut output = None;
        prepared
            .invoke_current_cached(&mut binding, &inputs, &mut output, limits, &work)
            .unwrap();
        assert!(
            binding
                .retained_metadata_demand(&work)
                .unwrap()
                .turn_peak_bytes
                > 0
        );
        let published = output.clone();
        let source = inputs[0].as_ref().unwrap();
        let changed = ValueDraft {
            schema: source.schema(),
            shape_values: source.shape().parameter_values().into(),
            data: ValueDataDraft::Matrix(
                (0..64)
                    .map(|index| ValueDataDraft::String(format!("new-{index}🧑‍🚀")))
                    .collect(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        inputs[0] = Some(admitted(changed, &memory));
        let changed_alias = inputs[0].clone();
        let changed_before = drafts(&inputs);
        prepared
            .invoke_current_cached(&mut binding, &inputs, &mut output, limits, &work)
            .unwrap();
        assert_eq!(string(&output), "new-1🧑‍🚀");
        assert_eq!(string(&published), "a\0b");
        assert_eq!(drafts(&inputs), changed_before);
        assert!(
            inputs[0]
                .as_ref()
                .unwrap()
                .shares_frozen_storage(changed_alias.as_ref().unwrap())
        );
        let selector = inputs[1].as_ref().unwrap();
        let zero = ValueDraft {
            schema: selector.schema(),
            shape_values: Box::new([]),
            data: ValueDataDraft::U64(0),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        inputs[1] = Some(admitted(zero, &memory));
        let prior = output.clone();
        let before_error = work.used();
        let cached_error = prepared
            .invoke_current_cached(&mut binding, &inputs, &mut output, limits, &work)
            .unwrap_err();
        assert_prior(&output, &prior);
        assert!(
            work.used().0 > before_error.0,
            "a failed selected call must retain spent work"
        );
        let plain_error = prepared
            .invoke_current(&inputs, &mut output, limits, &work)
            .unwrap_err();
        assert_eq!(format!("{cached_error:?}"), format!("{plain_error:?}"));
        assert_prior(&output, &prior);
        inputs[1] = original[1].clone();
        let before_retry = work.used();
        prepared
            .invoke_current_cached(&mut binding, &inputs, &mut output, limits, &work)
            .unwrap();
        assert!(work.used().0 > before_retry.0);
        assert_eq!(string(&output), "new-1🧑‍🚀");
        assert_eq!(drafts(&original), original_drafts);
        drop((
            output,
            prior,
            published,
            changed_alias,
            binding,
            inputs,
            original,
            call,
            prepared,
            work,
        ));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn maximum_binding_keeps_policy_work_and_current_memory_owner_checks() {
    for artifact in variants() {
        let limits = maximum_limits(u64::MAX);
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let (call, prepared, inputs) = setup(&artifact, &memory, limits, 64);
        let mut binding = NativePreparedLibraryCallCache::default();
        let mut output = None;
        prepared
            .invoke_current_cached(&mut binding, &inputs, &mut output, limits, &work)
            .unwrap();
        assert!(
            binding
                .retained_metadata_demand(&work)
                .unwrap()
                .turn_peak_bytes
                > 0
        );
        let prior = output.clone();
        let independent = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let foreign_prepared =
            prepare_native_program_library_call(&artifact, call.template(), &catalog()).unwrap();
        let mut changed = limits;
        changed.max_temporary_bytes = Some(u64::MAX - 1);
        for (selected, policy, account) in [
            (&prepared, changed, &work),
            (&prepared, limits, &independent),
            (&foreign_prepared, limits, &work),
        ] {
            assert_eq!(
                selected.invoke_current_cached(&mut binding, &inputs, &mut output, policy, account),
                Err(NativeCollectionInvocationError::InvalidInput)
            );
            assert_prior(&output, &prior);
        }
        let foreign = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let source = inputs[0].as_ref().unwrap();
        let mut reserve = foreign
            .reserve_capacity(
                source.memory_budget_required_bytes().unwrap()
                    + Value::memory_budget_claim_metadata_bytes(),
            )
            .unwrap();
        let imported = source
            .clone()
            .into_independent_memory_budget(&mut reserve)
            .unwrap();
        drop(reserve);
        assert_eq!(
            prepared.invoke_current_cached(
                &mut binding,
                &[Some(imported.clone()), inputs[1].clone()],
                &mut output,
                limits,
                &work
            ),
            Err(NativeCollectionInvocationError::UnadmittedValue)
        );
        assert_prior(&output, &prior);
        let arena = artifact.schemas().clone();
        let rebound = source
            .rebind_with_context(
                source.schema(),
                source.shape(),
                &SnapshotValidationContext::new(&arena),
            )
            .unwrap();
        assert!(rebound.memory_budget_retained_bytes(&memory).is_none());
        assert_eq!(
            prepared.invoke_current_cached(
                &mut binding,
                &[Some(rebound.clone()), inputs[1].clone()],
                &mut output,
                limits,
                &work
            ),
            Err(NativeCollectionInvocationError::UnadmittedValue)
        );
        assert_prior(&output, &prior);
        prepared
            .invoke_current_cached(&mut binding, &inputs, &mut output, limits, &work)
            .unwrap();
        assert_eq!(string(&output), "a\0b");
        drop((
            output,
            prior,
            binding,
            imported,
            rebound,
            arena,
            inputs,
            call,
            prepared,
            foreign_prepared,
            work,
            independent,
        ));
        assert_eq!(
            (
                memory.used_bytes(),
                memory.retained_nodes(),
                foreign.used_bytes(),
                foreign.retained_nodes()
            ),
            (0, 0, 0, 0)
        );
    }
}

#[test]
fn maximum_cache_finite_work_boundary_is_atomic_and_spent_work_survives_retry() {
    for artifact in variants() {
        let limits = maximum_limits(u64::MAX);
        let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
        let (call, prepared, inputs) = setup(&artifact, &memory, limits, 64);
        let before_inputs = drafts(&inputs);
        let reference_work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
        let mut reference = None;
        prepared
            .invoke_current(&inputs, &mut reference, limits, &reference_work)
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        let trial = |allowance| {
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let mut cache = NativePreparedLibraryCallCache::default();
            let mut output = reference.clone();
            let result = work
                .with_call_limits(allowance, u64::MAX, || {
                    prepared.invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                })
                .unwrap();
            let spent = work.used();
            assert!(spent.0 <= allowance);
            if result.is_err() {
                assert_prior(&output, &reference);
            } else {
                assert_eq!(string(&output), "a\0b");
            }
            assert_eq!(drafts(&inputs), before_inputs);
            let pass = result.is_ok();
            // A fresh source visit can retry under the original same ledger;
            // ending a tighter call scope must not refund its spent prefix.
            if !pass {
                prepared
                    .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                    .unwrap();
                assert!(work.used().0 > spent.0);
                assert_eq!(string(&output), "a\0b");
            }
            drop((output, cache, work));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            (pass, spent)
        };
        let (mut low, mut high) = (0, 1_000_000_000_000u64);
        assert!(trial(high).0);
        while low < high {
            let middle = low + (high - low) / 2;
            if trial(middle).0 {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        assert!(trial(low).0);
        assert!(low > 0);
        let (pass, spent) = trial(low - 1);
        assert!(
            !pass && spent.0 > 0,
            "one-short compute admission must fail without refunding its reached prefix"
        );
        assert!(trial(1_000_000_000_000).0);
        drop((reference, reference_work, inputs, prepared, call));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn warm_maximum_cache_cannot_starve_a_viable_grown_call_at_byte_or_node_boundary() {
    for artifact in variants() {
        for dimension in ["bytes", "nodes"] {
            let limits = maximum_limits(u64::MAX);
            let memory =
                ManagedMemoryBudget::new(16 * 1024 * 1024).with_limits(16 * 1024 * 1024, 4096);
            let work = NativeLibraryWorkBudget::new(u64::MAX, u64::MAX, &memory).unwrap();
            let (call, prepared, mut inputs) = setup(&artifact, &memory, limits, 64);
            let mut cache = NativePreparedLibraryCallCache::default();
            let mut output = None;
            prepared
                .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                .unwrap();
            let retained = cache.retained_metadata_demand(&work).unwrap();
            assert!(retained.turn_peak_bytes > 0 && retained.retained_nodes > 0);
            let old = inputs.clone();
            let old_output = output.clone();
            let source = inputs[0].as_ref().unwrap();
            let grown = ValueDraft {
                schema: source.schema(),
                shape_values: source.shape().parameter_values().into(),
                data: ValueDataDraft::Matrix(
                    (0..64)
                        .map(|index| {
                            ValueDataDraft::String(format!("{index}:{}", "é🧑‍🚀".repeat(64 + index)))
                        })
                        .collect(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            inputs[0] = Some(admitted(grown, &memory));
            let aliases = inputs.clone();
            let before = drafts(&inputs);
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            let available = if dimension == "bytes" {
                memory.limit_bytes() - memory.used_bytes()
            } else {
                memory.limit_nodes() - memory.retained_nodes()
            };
            let trial = |allowance| {
                let bytes = (dimension == "bytes")
                    .then(|| memory.reserve_capacity(available - allowance).unwrap());
                let nodes = (dimension == "nodes")
                    .then(|| memory.reserve_nodes(available - allowance).unwrap());
                let mut candidate = old_output.clone();
                let result = prepared.invoke_current(&inputs, &mut candidate, limits, &work);
                if result.is_err() {
                    assert_prior(&candidate, &old_output);
                }
                let pass = result.is_ok();
                drop((candidate, bytes, nodes));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                assert_eq!(drafts(&inputs), before);
                pass
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
            assert!(low > 0 && trial(low));
            assert!(!trial(low - 1));
            // The retained optional metadata accounts for the missing one
            // unit of headroom. A cache must not make an otherwise viable
            // ordinary invocation fail after actual payload geometry grows.
            let bytes = (dimension == "bytes")
                .then(|| memory.reserve_capacity(available - (low - 1)).unwrap());
            let nodes = (dimension == "nodes")
                .then(|| memory.reserve_nodes(available - (low - 1)).unwrap());
            let before_call = work.used();
            prepared
                .invoke_current_cached(&mut cache, &inputs, &mut output, limits, &work)
                .unwrap();
            assert!(work.used().0 > before_call.0);
            assert_eq!(string(&output), format!("1:{}", "é🧑‍🚀".repeat(65)));
            assert_eq!(string(&old_output), "a\0b");
            assert_eq!(drafts(&inputs), before);
            assert_eq!(drafts(&aliases), before);
            drop((
                bytes, nodes, output, old_output, aliases, old, cache, inputs, call, prepared, work,
            ));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}
