//! Differential publication, current shape, source billing and fail-closed admission.
use super::call_cache_tests::artifacts;
use super::*;
use crate::native_artifact::program::prepare::call_routes::LibraryCallRoute as Route;
use crate::native_artifact::{JitProgramArtifact, NativeProgramLimitMode};
use mech_core::{SchemaBody, snapshot::F64Bits};

const ALIAS: &str = "xs := input<[f64]>\n(xs, xs)\n";
const MIXED: &str =
    "(input<[f64]>, peer<[u64]>, flag<bool>, code<u64>, signed<i64>, scalar<f64>)\n";
const RECORD: &str =
    "{values: input<[f64]>, other: peer<[u64]>, enabled: flag<bool>, code: code<u64>}\n";

fn arguments(
    artifact: &mech_engine::ProgramArtifact,
    memory: &ManagedMemoryBudget,
    dimensions: [u64; 2],
) -> Vec<Value> {
    let bits = [
        0,
        1 << 63,
        0x7ff8_0000_0000_0017,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff0_0000_0000_0001,
        1,
        0x7fef_ffff_ffff_ffff,
    ];
    artifact
        .inputs()
        .iter()
        .map(|input| {
            let schema = artifact.schemas().get(input.schema).unwrap();
            let name = mech_engine::decode_source_input_name(&input.name).unwrap();
            let (data, shape_values) = if matches!(schema.body(), SchemaBody::Matrix { .. }) {
                let d = if name == "peer" {
                    [dimensions[1], dimensions[0]]
                } else {
                    dimensions
                };
                let shape = mech_core::shape_for_resolved_extents(schema, &d).unwrap();
                let data = (0..d[0] * d[1])
                    .map(|i| {
                        if name == "peer" {
                            ValueDataDraft::U64(u64::MAX - i)
                        } else {
                            ValueDataDraft::F64(F64Bits::from_f64(f64::from_bits(
                                bits[i as usize % bits.len()],
                            )))
                        }
                    })
                    .collect();
                (
                    ValueDataDraft::Matrix(data),
                    shape.parameter_values().into(),
                )
            } else {
                let data = match name.as_str() {
                    "flag" => ValueDataDraft::Bool(true),
                    "code" => ValueDataDraft::U64(u64::MAX),
                    "signed" => ValueDataDraft::I64(i64::MIN),
                    "scalar" => ValueDataDraft::F64(F64Bits::from_f64(f64::from_bits(bits[2]))),
                    _ => panic!("unexpected input {name}"),
                };
                (data, Box::new([]) as Box<[u64]>)
            };
            let value = ValueDraft {
                schema: input.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}
fn contents(args: &[Value]) -> Vec<ValueDataDraft> {
    args.iter()
        .map(|v| v.canonical_data_draft().unwrap())
        .collect()
}
fn retained(memory: &ManagedMemoryBudget) -> (u64, u64, u64) {
    (
        memory.used_bytes(),
        memory.retained_nodes(),
        memory.retained_snapshot_nodes(),
    )
}
fn restored(jit: &JitProgramArtifact) -> NativeProgramDescriptor {
    super::super::tests::restore_admitted_helper_descriptor(jit)
}
fn canonical(jit: &JitProgramArtifact) -> NativeProgramDescriptor {
    let mut descriptor = restored(jit);
    assert!(
        descriptor
            .metadata
            .call_routes
            .contains(&Route::NumericComposite)
    );
    for route in &mut descriptor.metadata.call_routes {
        if *route == Route::NumericComposite {
            *route = Route::Canonical;
        }
    }
    // No serialized route can authorize the specialized construction strategy.
    assert_eq!(
        descriptor.encode_build_owned_image().unwrap(),
        jit.descriptor().encode_build_owned_image().unwrap()
    );
    descriptor
}

#[test]
fn numeric_composite_preserves_current_shapes_aliases_ieee_bits_and_source_accounting() {
    for source in [ALIAS, MIXED, RECORD] {
        for artifact in artifacts(source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let image = restored(&jit);
            let canonical = canonical(&jit);
            for dimensions in [
                [0, 0],
                [0, 7],
                [7, 0],
                [1, 1],
                [2, 3],
                [3, 2],
                [1, 1024],
                [4, 4],
            ] {
                let memory = mode.memory_budget();
                let args = arguments(&artifact, &memory, dimensions);
                let aliases = args.clone();
                let before = contents(&args);
                let baseline = retained(&memory);
                let mut results = Vec::new();
                let mut reports = Vec::new();
                for descriptor in [jit.descriptor(), &image, &canonical] {
                    let mut outputs = vec![None; artifact.outputs().len()];
                    reports.push(
                        crate::native_artifact::program::runtime::execute(
                            descriptor,
                            jit.test_generated_entry(),
                            &args,
                            &mut outputs,
                            mode.execution(),
                            &memory,
                        )
                        .unwrap(),
                    );
                    results.push(
                        outputs
                            .iter()
                            .map(|v| {
                                let value = v.as_ref().unwrap();
                                (
                                    value.schema_key(),
                                    value.shape().clone(),
                                    value.canonical_data_draft().unwrap(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    );
                    drop(outputs);
                    assert_eq!(retained(&memory), baseline);
                }
                assert_eq!(results[0], results[1]);
                assert_eq!(results[0], results[2], "{source} {dimensions:?}");
                assert_eq!(reports[0], reports[1]);
                assert_eq!(reports[0].visits, reports[2].visits);
                assert!(
                    reports[0].work > reports[2].work,
                    "selected source envelope must be paid: {reports:?}"
                );
                assert!(reports[0].comparison_work >= reports[2].comparison_work);
                assert_eq!(contents(&args), before);
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                drop((args, aliases));
                assert_eq!(retained(&memory), (0, 0, 0));
            }
        }
    }
}

#[test]
fn numeric_composite_missing_caps_declines_to_original_and_finite_denials_are_atomic() {
    for artifact in artifacts(MIXED) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let canonical = canonical(&jit);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = arguments(&artifact, &memory, [2, 3]);
        let before = contents(&args);
        let baseline = retained(&memory);
        for which in 0..18 {
            let mut limits = mode.execution();
            match which {
                0 => limits.library.max_output_elements = None,
                1 => limits.library.max_output_bytes = None,
                2 => limits.library.max_temporary_bytes = None,
                3 => limits.library.max_cloned_bytes = None,
                4 => limits.library.max_retained_nodes = None,
                5 => limits.library.max_compute_work = None,
                6 => limits.library.max_comparison_work = None,
                7 => limits.library.max_scalar_instructions = None,
                8 => limits.library.max_transfer_bytes = None,
                9 => limits.library.max_storage_buffer_bytes = None,
                10 => limits.library.max_storage_bindings = None,
                11 => limits.library.max_output_elements = Some(0),
                12 => limits.library.max_output_bytes = Some(0),
                13 => limits.library.max_temporary_bytes = Some(0),
                14 => limits.library.max_retained_nodes = Some(0),
                15 => limits.library.max_compute_work = Some(0),
                16 => limits.library.max_comparison_work = Some(0),
                _ => limits.execution.max_work = 1,
            }
            let mut results = Vec::new();
            for descriptor in [jit.descriptor(), &canonical] {
                let mut outputs = vec![None; artifact.outputs().len()];
                let result = crate::native_artifact::program::runtime::execute(
                    descriptor,
                    jit.test_generated_entry(),
                    &args,
                    &mut outputs,
                    limits,
                    &memory,
                );
                if which >= 11 && which != 16 {
                    assert!(result.is_err(), "finite denial {which}: {result:?}");
                }
                if which == 16 {
                    assert!(result.is_ok(), "ordinary constructor has no comparisons");
                }
                results.push((
                    format!("{result:?}"),
                    outputs
                        .iter()
                        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                        .collect::<Vec<_>>(),
                ));
                drop(outputs);
                assert_eq!(retained(&memory), baseline, "limit {which}");
            }
            assert_eq!(results[0], results[1], "original fallback on limit {which}");
            assert_eq!(contents(&args), before);
        }
        for nodes in [false, true] {
            let denied = memory.with_limits(
                if nodes {
                    memory.limit_bytes()
                } else {
                    baseline.0
                },
                if nodes { baseline.1 } else { u64::MAX },
            );
            let mut outputs = vec![None; artifact.outputs().len()];
            assert!(
                crate::native_artifact::program::runtime::execute(
                    jit.descriptor(),
                    jit.test_generated_entry(),
                    &args,
                    &mut outputs,
                    mode.execution(),
                    &denied
                )
                .is_err()
            );
            assert!(outputs.iter().all(Option::is_none));
            assert_eq!(retained(&memory), baseline);
        }
        let mut outputs = vec![None; artifact.outputs().len()];
        crate::native_artifact::program::runtime::execute(
            jit.descriptor(),
            jit.test_generated_entry(),
            &args,
            &mut outputs,
            mode.execution(),
            &memory,
        )
        .unwrap();
        // Keep the exact same prior publication for the source quote and
        // both finite runs. Resource decline may select a cheaper source bill;
        // that must never renew the ledger or discard a denied publication.
        let full = crate::native_artifact::program::runtime::execute(
            jit.descriptor(),
            jit.test_generated_entry(),
            &args,
            &mut outputs,
            mode.execution(),
            &memory,
        )
        .unwrap();
        let mut reference = outputs.clone();
        let source = crate::native_artifact::program::runtime::execute(
            &canonical,
            jit.test_generated_entry(),
            &args,
            &mut reference,
            mode.execution(),
            &memory,
        )
        .unwrap();
        drop(reference);
        assert!(full.work > source.work);
        let prior = outputs.clone();
        let baseline = retained(&memory);
        let mut finite = mode.execution();
        finite.execution.max_work = source.work - 1;
        let denied = crate::native_artifact::program::runtime::execute(
            jit.descriptor(),
            jit.test_generated_entry(),
            &args,
            &mut outputs,
            finite,
            &memory,
        )
        .unwrap_err();
        assert!(format!("{denied:?}").contains("WorkLimit"));
        assert!(outputs.iter().zip(&prior).all(|(a, b)| {
            a.as_ref()
                .unwrap()
                .shares_frozen_storage(b.as_ref().unwrap())
        }));
        assert_eq!(retained(&memory), baseline);
        finite.execution.max_work = full.work - 1;
        let fallback = crate::native_artifact::program::runtime::execute(
            jit.descriptor(),
            jit.test_generated_entry(),
            &args,
            &mut outputs,
            finite,
            &memory,
        )
        .unwrap();
        assert_eq!(
            fallback, source,
            "unfunded native envelope must use the complete original bill"
        );
        assert_eq!(contents(&args), before);
        drop((prior, outputs, args));
        assert_eq!(retained(&memory), (0, 0, 0));
    }
}

#[test]
fn numeric_composite_does_not_select_scalar_only_or_non_numeric_aggregate() {
    for source in [
        "(code<u64>, flag<bool>)\n",
        "(input<[string]>, code<u64>)\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                !jit.descriptor()
                    .metadata
                    .call_routes
                    .contains(&Route::NumericComposite)
            );
            assert!(
                jit.descriptor()
                    .metadata
                    .calls
                    .iter()
                    .flatten()
                    .all(|c| !c.supports_canonical_numeric_pack())
            );
        }
    }
}
