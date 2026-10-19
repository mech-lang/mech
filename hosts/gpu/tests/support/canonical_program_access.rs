//! Maintained matrix and map selection, not alternate native lookup kernels.
use super::{artifacts, check, equal, inputs, limits, reference, reference_result};
use mech_core::snapshot::{F32Bits, F64Bits, MapEntryDraft};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

fn matrix(values: &[Data]) -> Data {
    Data::Matrix(values.to_vec().into_boxed_slice())
}

#[test]
fn native_synchronous_tail_machine_uses_generated_control_and_structural_states() {
    let source = "#Countdown(n<u64>) => <u64>\n  | :Loop(value<u64>)\n  | :Done(value<u64>).\n#Countdown(n) -> :Loop(n)\n  :Loop(value)\n    | value > 0u64 -> :Loop(value - 1u64)\n    | * -> :Done(value)\n  :Done(value) => value.\n#Countdown(input<u64>)\n";
    for count in [0, 1, 7] {
        exact(source, &[("input", Data::U64(count))], &Data::U64(0));
    }
}

#[test]
fn ordinary_knucleotide_algorithm_compiles_to_native_without_a_fsm_interpreter() {
    // This is algorithm-only qualification, NOT hosted completion. Preserve
    // every ordinary fixture expression; only expose the provider input and
    // delivery as library test ports. A separate hosted route must compile
    // and execute the unmodified fixture with its original requirements.
    let fixture = include_str!("../fixtures/native/knucleotide-idiomatic.mec");
    let mut source = fixture
        .lines()
        .filter(|line| !line.starts_with("+> @"))
        .map(|line| {
            line.replace("@in/text", "input<string>")
                .replace("@out/text <- ", "delivery := ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    source.push_str("\ndelivery\n");
    exact(
        &source,
        &[(
            "input",
            Data::String(">ONE\nAAAA\n>THREE description\ngG\nta\n>FOUR\nCC\n".into()),
        )],
        &Data::String(
            concat!(
                "G 50.000\nA 25.000\nT 25.000\n\n",
                "GG 33.333\nGT 33.333\nTA 33.333\n\n",
                "1\tGGT\n1\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n"
            )
            .into(),
        ),
    );
}

#[test]
fn generated_sort_and_reductions_use_the_maintained_canonical_kernels() {
    let root =
        "+> sequence\nsequence/sort-indices(counts<[u64]>, \"desc\", keys<[string]>, \"asc\")\n";
    let lexical = "+> sequence\nrank(xs<[u64]>, ks<[string]>) => (out<u64>) :=\n  order := sequence/sort-indices(xs, \"desc\", ks, \"asc\")\n  out := sequence/length(order).\nrank(counts<[u64]>, keys<[string]>)\n";
    for count in [5, 0] {
        let counts = [
            9_007_199_254_740_993,
            9_007_199_254_740_992,
            9_007_199_254_740_993,
            9_007_199_254_740_993,
            9_007_199_254_740_992,
        ];
        let keys = ["β", "b", "a", "a", "a"];
        let named = [
            (
                "counts",
                matrix(
                    &counts[..count]
                        .iter()
                        .copied()
                        .map(Data::U64)
                        .collect::<Vec<_>>(),
                ),
            ),
            (
                "keys",
                matrix(
                    &keys[..count]
                        .iter()
                        .map(|key| Data::String((*key).into()))
                        .collect::<Vec<_>>(),
                ),
            ),
        ];
        exact(
            root,
            &named,
            &matrix(
                &[3, 4, 1, 5, 2][..count]
                    .iter()
                    .copied()
                    .map(Data::Index)
                    .collect::<Vec<_>>(),
            ),
        );
        exact(lexical, &named, &Data::U64(count as u64));
        let order_source = root.replacen("\nsequence/", "\norder := sequence/", 1);
        exact(
            &format!("{order_source}[i | i <- order]\n"),
            &named,
            &matrix(
                &[3, 4, 1, 5, 2][..count]
                    .iter()
                    .copied()
                    .map(Data::Index)
                    .collect::<Vec<_>>(),
            ),
        );
        exact(
            &format!("{order_source}[keys[i] | i <- order]\n"),
            &named,
            &matrix(
                &["a", "a", "β", "a", "b"][..count]
                    .iter()
                    .map(|key| Data::String((*key).into()))
                    .collect::<Vec<_>>(),
            ),
        );
    }
    for (axis, expected) in [("column", &[6, 15][..]), ("row", &[5, 7, 9][..])] {
        let source =
            format!("+> stats\nxs := [1u64 2u64 3u64; 4u64 5u64 6u64]\nstats/sum/{axis}(xs)\n");
        exact(
            &source,
            &[],
            &matrix(&expected.iter().copied().map(Data::U64).collect::<Vec<_>>()),
        );
    }
}

#[test]
fn managed_string_predicates_and_concat_keep_raw_identity_in_generated_jit_and_objects() {
    for (left, right, same) in [
        ("", "", true),
        ("é", "e\u{301}", false),
        ("🧑‍🚀", "🧑‍🚀", true),
        ("", "x", false),
        (">THREE", ">THREE", true),
    ] {
        let named = [
            ("lhs", Data::String(left.into())),
            ("rhs", Data::String(right.into())),
        ];
        for (operator, expected) in [("==", same), ("!=", !same)] {
            exact(
                &format!("left := lhs<string>\nright := rhs<string>\nleft {operator} right\n"),
                &named,
                &Data::Bool(expected),
            );
            exact(
                &format!(
                    "compare(a<string>, b<string>) => (out<bool>) := out := a {operator} b.\ncompare(lhs<string>, rhs<string>)\n"
                ),
                &named,
                &Data::Bool(expected),
            );
        }
        exact(
            "left := lhs<string>\nright := rhs<string>\nleft + right\n",
            &named,
            &Data::String(format!("{left}{right}")),
        );
        exact(
            "combine(a<string>, b<string>) => (out<string>) :=\n  out := ((a == b)?\n    | true => a + b\n    | false => b + a).\ncombine(lhs<string>, rhs<string>)\n",
            &named,
            &Data::String(if same {
                format!("{left}{right}")
            } else {
                format!("{right}{left}")
            }),
        );
    }
    exact(
        "tail := suffix<string>\nstep(s<string>) => (out<string>) := out := s + tail.\nseed<string> |2> step\n",
        &[
            ("suffix", Data::String("🧑‍🚀".into())),
            ("seed", Data::String("e\u{301}".into())),
        ],
        &Data::String("e\u{301}🧑‍🚀🧑‍🚀".into()),
    );
    exact(
        "[char + char | char <- input<string>]\n",
        &[("input", Data::String("e\u{301}🧑‍🚀".into()))],
        &matrix(&[
            Data::String("e\u{301}e\u{301}".into()),
            Data::String("🧑‍🚀🧑‍🚀".into()),
        ]),
    );
}

#[test]
fn matrix_range_access_keeps_shallow_elements_empty_selectors_and_lexical_order() {
    let values = matrix(&[
        Data::String("é".into()),
        Data::String("e\u{301}".into()),
        Data::String("🧑‍🚀".into()),
        Data::String("tail".into()),
    ]);
    for (selectors, expected) in [
        (
            vec![3, 1, 3],
            vec![
                Data::String("🧑‍🚀".into()),
                Data::String("é".into()),
                Data::String("🧑‍🚀".into()),
            ],
        ),
        (vec![], vec![]),
        (vec![2], vec![Data::String("e\u{301}".into())]),
    ] {
        let named = [
            ("values", values.clone()),
            (
                "selectors",
                matrix(&selectors.into_iter().map(Data::U64).collect::<Vec<_>>()),
            ),
        ];
        exact(
            "xs := values<[string]>\nxs[selectors<[u64]>]\n",
            &named,
            &matrix(&expected),
        );
        exact(
            "gather(xs<[string]>, indices<[u64]>) => (out<[string]>) := out := xs[indices].\ngather(values<[string]>, selectors<[u64]>)\n",
            &named,
            &matrix(&expected),
        );
    }
    exact(
        "xs := values<[string]>\nxs[first<u64>..end<u64>]\n",
        &[
            ("values", values),
            ("first", Data::U64(2)),
            ("end", Data::U64(4)),
        ],
        &matrix(&[Data::String("e\u{301}".into()), Data::String("🧑‍🚀".into())]),
    );
    exact(
        "xs := values<[u128]>\nxs[selectors<[u64]>]\n",
        &[
            (
                "values",
                matrix(&[Data::U128(u128::MAX), Data::U128(1 << 100)]),
            ),
            ("selectors", matrix(&[Data::U64(2), Data::U64(1)])),
        ],
        &matrix(&[Data::U128(1 << 100), Data::U128(u128::MAX)]),
    );
}

#[test]
fn generated_matrix_range_failures_preserve_publication_account_and_recover() {
    let source = "xs := values<[string]>\nxs[selectors<[u64]>]\n";
    let source_values = matrix(&[
        Data::String("é".into()),
        Data::String("e\u{301}".into()),
        Data::String("🧑‍🚀".into()),
    ]);
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let mut output = vec![None; artifact.outputs().len()];
        for selectors in [vec![3, 1], vec![0, 1], vec![1, 4], vec![2, 1]] {
            let named = [
                ("values", source_values.clone()),
                (
                    "selectors",
                    matrix(&selectors.into_iter().map(Data::U64).collect::<Vec<_>>()),
                ),
            ];
            let args = inputs(&artifact, &named, &memory);
            match reference_result(&artifact, &args) {
                Ok(expected) => {
                    jit.evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap();
                    equal(&artifact, &output, &expected);
                }
                Err(expected) => {
                    let prior = output
                        .iter()
                        .map(|v| v.as_ref().unwrap().clone())
                        .collect::<Vec<_>>();
                    let before = memory.used_bytes();
                    let error = jit
                        .evaluate_into(&args, &mut output, limits(), &memory)
                        .unwrap_err();
                    let mech_gpu::native_artifact::NativeProgramExecutionError::Library {
                        error:
                            mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                                actual,
                            ),
                        ..
                    } = error
                    else {
                        panic!("{error:?}")
                    };
                    assert!(
                        expected.contains(&format!("{actual:?}")),
                        "{expected}: {actual:?}"
                    );
                    equal(&artifact, &output, &prior);
                    assert_eq!(memory.used_bytes(), before);
                }
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn annotated_matrix_to_set_conversion_runs_without_a_comprehension() {
    // The reference's two-dimensional matrix example uses an annotation,
    // not an identity comprehension or a special native constructor.
    exact(
        "x := [1.1 2.2; 3.3 4.4]\ns<{f64}> := x\ns\n",
        &[],
        &Data::Set(
            [1.1, 2.2, 3.3, 4.4]
                .map(|v| Data::F64(F64Bits::from_f64(v)))
                .into(),
        ),
    );
    for (kind, values, expected) in [
        (
            "u64",
            vec![Data::U64(4), Data::U64(1), Data::U64(4), Data::U64(2)],
            vec![Data::U64(1), Data::U64(2), Data::U64(4)],
        ),
        (
            "u128",
            vec![
                Data::U128(u128::MAX),
                Data::U128(1 << 100),
                Data::U128(u128::MAX),
            ],
            vec![Data::U128(1 << 100), Data::U128(u128::MAX)],
        ),
        (
            "bool",
            vec![Data::Bool(true), Data::Bool(false), Data::Bool(true)],
            vec![Data::Bool(false), Data::Bool(true)],
        ),
        (
            "string",
            vec![
                Data::String("é".into()),
                Data::String("é".into()),
                Data::String("x".into()),
                Data::String("é".into()),
            ],
            vec![
                Data::String("é".into()),
                Data::String("x".into()),
                Data::String("é".into()),
            ],
        ),
    ] {
        let source = format!("unique<{{{kind}}}> := values<[{kind}]>\nunique\n");
        exact(
            &source,
            &[("values", matrix(&values))],
            &Data::Set(expected.into_boxed_slice()),
        );
        exact(
            &source,
            &[("values", matrix(&[]))],
            &Data::Set(Box::new([])),
        );
    }
    exact(
        "unique<{i64}> := values<[f64]>\nunique\n",
        &[(
            "values",
            matrix(&[
                Data::F64(F64Bits::from_f64(1.1)),
                Data::F64(F64Bits::from_f64(1.9)),
                Data::F64(F64Bits::from_f64(2.1)),
            ]),
        )],
        &Data::Set(vec![Data::I64(1), Data::I64(2)].into_boxed_slice()),
    );
}

#[test]
fn variable_arity_constructors_keep_live_scalar_and_managed_operands() {
    for count in [1, 2, 4, 16, 64] {
        let entries = (0..count)
            .map(|i| format!("{i}u64: base<u64> + {i}u64"))
            .collect::<Vec<_>>()
            .join(", ");
        let source = format!("mapping := {{{entries}}}\nmapping{{query<u64>}}\n");
        exact_with_resident_work_policy(
            &source,
            &[("base", Data::U64(1000)), ("query", Data::U64(count - 1))],
            &Data::U64(1000 + count - 1),
            count >= 16,
            count == 64,
        );
    }
    exact(
        "(number<u64> + 1u64, label<string>, flag<bool>, number * 2u64)\n",
        &[
            ("number", Data::U64(9)),
            ("label", Data::String("🧑‍🚀".into())),
            ("flag", Data::Bool(true)),
        ],
        &Data::Tuple(
            vec![
                Data::U64(10),
                Data::String("🧑‍🚀".into()),
                Data::Bool(true),
                Data::U64(18),
            ]
            .into(),
        ),
    );
}

#[test]
fn converted_key_sets_feed_maintained_membership_and_keep_live_extents() {
    for operation in ["∈", "∉"] {
        let source =
            format!("keys<{{string}}> := values<[string]>\nquery<string> {operation} keys\n");
        for (values, present) in [
            (vec!["a", "b", "a"], true),
            (vec![], false),
            (vec!["c"], false),
            (vec!["b"], true),
        ] {
            exact(
                &source,
                &[
                    (
                        "values",
                        matrix(
                            &values
                                .iter()
                                .map(|s| Data::String((*s).into()))
                                .collect::<Vec<_>>(),
                        ),
                    ),
                    ("query", Data::String("b".into())),
                ],
                &Data::Bool(if operation == "∈" {
                    present
                } else {
                    !present
                }),
            );
            let histogram_source = format!(
                "+> map\ncounts := map/histogram(values<[string]>)\nkeys<{{string}}> := map/keys(counts)\nquery<string> {operation} keys\n"
            );
            exact(
                &histogram_source,
                &[
                    (
                        "values",
                        matrix(
                            &values
                                .iter()
                                .map(|s| Data::String((*s).into()))
                                .collect::<Vec<_>>(),
                        ),
                    ),
                    ("query", Data::String("b".into())),
                ],
                &Data::Bool(if operation == "∈" {
                    present
                } else {
                    !present
                }),
            );
        }
    }
}

#[test]
fn set_conversion_live_geometry_checked_casts_and_finite_denials_are_atomic() {
    use mech_core::{MemoryBudgetDimension as Dimension, MemoryPlanError, ResidentKernelError};
    use mech_engine::native_artifact::NativeCollectionInvocationError as LeafError;
    use mech_gpu::native_artifact::NativeProgramExecutionError as Error;
    for artifact in artifacts("unique<{i64}> := values<[f64]>\nunique\n").0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let named = |values: &[f64]| {
            [(
                "values",
                matrix(
                    &values
                        .iter()
                        .map(|v| Data::F64(F64Bits::from_f64(*v)))
                        .collect::<Vec<_>>(),
                ),
            )]
        };
        let good = inputs(&artifact, &named(&[1.1, 1.9, 2.1]), &memory);
        let bad = inputs(&artifact, &named(&[3.0, f64::NAN]), &memory);
        let mut outputs = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&good, &mut outputs, limits(), &memory)
            .unwrap();
        let prior = outputs
            .iter()
            .map(|v| v.as_ref().unwrap().clone())
            .collect::<Vec<_>>();
        let before = memory.used_bytes();
        assert!(reference_result(&artifact, &bad).is_err());
        assert!(matches!(
            jit.evaluate_into(&bad, &mut outputs, limits(), &memory),
            Err(Error::Library {
                error: LeafError::Kernel(ResidentKernelError::Arithmetic),
                ..
            })
        ));
        equal(&artifact, &outputs, &prior);
        assert_eq!(memory.used_bytes(), before);
        for dimension in [
            Dimension::OutputElements,
            Dimension::OutputBytes,
            Dimension::TemporaryBytes,
            Dimension::ClonedBytes,
            Dimension::RetainedNodes,
            Dimension::ComputeWork,
            Dimension::ComparisonWork,
        ] {
            let mut denied = limits();
            match dimension {
                Dimension::OutputElements => denied.library.max_output_elements = Some(0),
                Dimension::OutputBytes => denied.library.max_output_bytes = Some(0),
                Dimension::TemporaryBytes => denied.library.max_temporary_bytes = Some(0),
                Dimension::ClonedBytes => denied.library.max_cloned_bytes = Some(0),
                Dimension::RetainedNodes => denied.library.max_retained_nodes = Some(0),
                Dimension::ComputeWork => denied.library.max_compute_work = Some(0),
                Dimension::ComparisonWork => denied.library.max_comparison_work = Some(0),
                _ => unreachable!(),
            }
            let before = memory.used_bytes();
            let error = jit
                .evaluate_into(&good, &mut outputs, denied, &memory)
                .unwrap_err();
            let actual = match &error {
                Error::Library {
                    error: LeafError::Kernel(ResidentKernelError::ResourceLimit { dimension, .. }),
                    ..
                } => *dimension,
                Error::Library {
                    error: LeafError::Plan(MemoryPlanError::TargetLimitExceeded { violation }),
                    ..
                } => violation.dimension,
                Error::Library {
                    error: LeafError::Binding(mech_engine::native_artifact::NativeArtifactLoweringError::ResidentBinding {
                        error: mech_engine::resident::ResidentActivationError::ResidentMemoryPlanRejected {
                            error: MemoryPlanError::TargetLimitExceeded { violation },
                        },
                    }),
                    ..
                } => violation.dimension,
                _ => panic!("expected typed {dimension:?} denial, got {error:?}"),
            };
            assert_eq!(actual, dimension, "{error:?}");
            equal(&artifact, &outputs, &prior);
            assert_eq!(memory.used_bytes(), before);
            jit.evaluate_into(&good, &mut outputs, limits(), &memory)
                .unwrap();
            equal(&artifact, &outputs, &prior);
        }
        for values in [&[][..], &[9.1, 9.9][..], &[4.1, 3.0, 4.9][..]] {
            let args = inputs(&artifact, &named(values), &memory);
            jit.evaluate_into(&args, &mut outputs, limits(), &memory)
                .unwrap();
            equal(&artifact, &outputs, &reference(&artifact, &args));
        }
        drop((good, bad, outputs, prior));
        assert_eq!(memory.used_bytes(), 0);
    }
}

fn map(entries: &[(&str, Data)]) -> Data {
    Data::Map(
        entries
            .iter()
            .map(|(key, value)| MapEntryDraft {
                items: vec![Data::String((*key).into()), value.clone()].into_boxed_slice(),
            })
            .collect(),
    )
}

fn exact(source: &str, named: &[(&str, Data)], expected: &Data) {
    exact_with_resident_work_policy(source, named, expected, false, false)
}

fn exact_with_resident_work_policy(
    source: &str,
    named: &[(&str, Data)],
    expected: &Data,
    allow_resident_comparison_denial: bool,
    expect_native_compute_denial: bool,
) {
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, named, &memory);
        let ordinary = match reference_result(&artifact, &args) {
            Ok(ordinary) => {
                assert_eq!(
                    &ordinary.last().unwrap().canonical_data_draft().unwrap(),
                    expected
                );
                Some(ordinary)
            }
            Err(error) => {
                // Large constructors retain the ordinary resident policy's
                // finite denial; do not raise product limits or drop the case.
                // Native execution uses the explicit existing qualification
                // policy and must still match the independent literal oracle.
                assert!(
                    allow_resident_comparison_denial
                        && error.contains(
                            "ResourceLimit { phase: Admission, dimension: ComparisonWork,"
                        )
                        && error.contains("limit: 65536"),
                    "{source}: {error}"
                );
                None
            }
        };
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        if expect_native_compute_denial {
            // All128 operands compile; this deliberately unfunded case must
            // preserve the existing finite policy, not gain a larger budget.
            // It is denial coverage, NOT successful large-map execution.
            let before = memory.used_bytes();
            for _ in 0..2 {
                let error = jit
                    .evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap_err();
                assert!(
                    matches!(error,
                    mech_gpu::native_artifact::NativeProgramExecutionError::Library {
                        error: mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::ResourceLimit {
                                dimension: mech_core::MemoryBudgetDimension::ComputeWork, required, limit, ..
                            }), ..
                    } if required > limit),
                    "{error:?}"
                );
                assert!(output.iter().all(Option::is_none));
                assert_eq!(memory.used_bytes(), before);
            }
            drop((args, ordinary, output));
            assert_eq!(memory.used_bytes(), 0);
            continue;
        }
        if let Err(error) = jit.evaluate_into(&args, &mut output, limits(), &memory) {
            if let mech_gpu::native_artifact::NativeProgramExecutionError::Library {
                origin, ..
            } = &error
            {
                for block in jit.descriptor().plan().blocks() {
                    if let mech_engine::native_artifact::NativeIterationAction::LibraryCall {
                        template,
                        ..
                    } = block.action()
                        && template.origin() == *origin
                    {
                        let prepared =
                            mech_engine::native_artifact::prepare_native_program_library_call(
                                &artifact,
                                template,
                                &mech_stdlib::source_catalog(),
                            )
                            .unwrap();
                        eprintln!(
                            "native leaf failure: {:?} {:?}; binding_work={}, binding_metadata_bytes={}",
                            template.origin(),
                            template.selected_key(),
                            prepared.binding_work(),
                            prepared.binding_metadata_bytes()
                        );
                    }
                }
            }
            panic!("{error:?}");
        }
        if let Some(ordinary) = &ordinary {
            equal(&artifact, &output, ordinary);
        }
        assert_eq!(
            &output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected
        );
        drop((args, ordinary, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn matrix_scalar_selection_preserves_every_atomic_kind_in_roots_and_functions() {
    let cases = [
        ("bool", Data::Bool(false), Data::Bool(true)),
        ("ix", Data::Index(1), Data::Index(9)),
        ("u8", Data::U8(0), Data::U8(u8::MAX)),
        ("u16", Data::U16(0), Data::U16(u16::MAX)),
        ("u32", Data::U32(0), Data::U32(u32::MAX)),
        ("u64", Data::U64(0), Data::U64(u64::MAX)),
        ("u128", Data::U128(0), Data::U128(u128::MAX)),
        ("i8", Data::I8(i8::MIN), Data::I8(i8::MAX)),
        ("i16", Data::I16(i16::MIN), Data::I16(i16::MAX)),
        ("i32", Data::I32(i32::MIN), Data::I32(i32::MAX)),
        ("i64", Data::I64(i64::MIN), Data::I64(i64::MAX)),
        ("i128", Data::I128(i128::MIN), Data::I128(i128::MAX)),
        (
            "f32",
            Data::F32(F32Bits::from_f32(1.0)),
            Data::F32(F32Bits::from_f32(-0.0)),
        ),
        (
            "f64",
            Data::F64(F64Bits::from_f64(1.0)),
            Data::F64(F64Bits::from_f64(-0.0)),
        ),
        (
            "string",
            Data::String("é".into()),
            Data::String("🧑‍🚀".into()),
        ),
    ];
    for (kind, first, second) in cases {
        let named = [
            ("values", matrix(&[first, second.clone()])),
            ("position", Data::U64(2)),
        ];
        for source in [
            format!("xs := values<[{kind}]>\nxs[position<u64>]\n"),
            format!(
                "pick(xs<[{kind}]>, i<u64>) => (out<{kind}>) := out := xs[i].\npick(values<[{kind}]>, position<u64>)\n"
            ),
        ] {
            exact(&source, &named, &second);
        }
    }
}

#[test]
fn map_lookup_uses_live_string_keys_and_atomic_values_without_narrowing() {
    for (kind, value) in [
        ("u64", Data::U64(u64::MAX)),
        ("u128", Data::U128(u128::MAX)),
        ("i128", Data::I128(i128::MIN)),
        ("f64", Data::F64(F64Bits::from_f64(-0.0))),
        ("bool", Data::Bool(true)),
        ("string", Data::String("🧑‍🚀é".into())),
    ] {
        exact(
            &format!("mapping := entries<{{string:{kind}}}>\nmapping{{query<string>}}\n"),
            &[
                (
                    "entries",
                    map(&[("a", value.clone()), ("é", value.clone())]),
                ),
                ("query", Data::String("é".into())),
            ],
            &value,
        );
    }
    // Normal language-level case translation, including iteration of graphemes.
    exact(
        "+> text\nuppercase := {\"a\": \"A\", \"c\": \"C\", \"g\": \"G\", \"t\": \"T\"}\ntext/join([uppercase{char} | char <- input<string>], \"\")\n",
        &[("input", Data::String("gattaca".into()))],
        &Data::String("GATTACA".into()),
    );
    exact(
        "mapping := {1u64: 11u64, 2u64: 22u64}\nmapping{query<u64>}\n",
        &[("query", Data::U64(2))],
        &Data::U64(22),
    );
}

#[test]
fn indexed_generated_comprehensions_and_lexical_calls_share_selection_semantics() {
    let named = [
        (
            "values",
            matrix(&[Data::U64(10), Data::U64(20), Data::U64(30)]),
        ),
        ("entries", map(&[("x", Data::U64(7))])),
        ("query", Data::String("x".into())),
        ("position", Data::U64(2)),
        ("seed", Data::U64(1)),
    ];
    check(
        "xs := values<[u64]>\nh := entries<{string:u64}>\nkey := query<string>\npos := position<u64>\nstep(x<u64>) => (out<u64>) := out := x + xs[pos] + h{key}.\nseed<u64> |2> step\n",
        &named,
    );
    exact(
        "xs := values<[u64]>\nh := entries<{string:u64}>\nkey := query<string>\n[xs[i] + h{key} | i <- [3u64 1u64 3u64]]\n",
        &named,
        &matrix(&[Data::U64(37), Data::U64(17), Data::U64(37)]),
    );
}

#[test]
fn matrix_and_map_selection_failures_preserve_publication_memory_and_recover() {
    for (source, calls) in [
        (
            "xs := values<[u64]>\nxs[position<u64>]\n",
            vec![
                vec![
                    ("values", matrix(&[Data::U64(10), Data::U64(20)])),
                    ("position", Data::U64(1)),
                ],
                vec![("values", matrix(&[])), ("position", Data::U64(1))],
                vec![
                    ("values", matrix(&[Data::U64(10)])),
                    ("position", Data::U64(0)),
                ],
                vec![
                    ("values", matrix(&[Data::U64(10)])),
                    ("position", Data::U64(2)),
                ],
                vec![
                    ("values", matrix(&[Data::U64(30)])),
                    ("position", Data::U64(1)),
                ],
            ],
        ),
        (
            "h := entries<{string:u64}>\nh{query<string>}\n",
            vec![
                vec![
                    ("entries", map(&[("a", Data::U64(1))])),
                    ("query", Data::String("a".into())),
                ],
                vec![("entries", map(&[])), ("query", Data::String("a".into()))],
                vec![
                    ("entries", map(&[("a", Data::U64(1))])),
                    ("query", Data::String("missing".into())),
                ],
                vec![
                    ("entries", map(&[("a", Data::U64(9))])),
                    ("query", Data::String("a".into())),
                ],
            ],
        ),
    ] {
        for artifact in artifacts(source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut output = vec![None; artifact.outputs().len()];
            for named in &calls {
                let args = inputs(&artifact, named, &memory);
                match reference_result(&artifact, &args) {
                    Ok(expected) => {
                        jit.evaluate_into(&args, &mut output, limits(), &memory)
                            .unwrap();
                        equal(&artifact, &output, &expected);
                    }
                    Err(expected) => {
                        let prior = output
                            .iter()
                            .map(|v| v.as_ref().unwrap().clone())
                            .collect::<Vec<_>>();
                        let before = memory.used_bytes();
                        let error = jit
                            .evaluate_into(&args, &mut output, limits(), &memory)
                            .unwrap_err();
                        // Exact maintained kernel error, not just any denial.
                        let mech_gpu::native_artifact::NativeProgramExecutionError::Library {
                            error:
                                mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                                    actual,
                                ),
                            ..
                        } = error
                        else {
                            panic!("{error:?}")
                        };
                        assert!(
                            expected.contains(&format!("{actual:?}")),
                            "{expected}: {actual:?}"
                        );
                        equal(&artifact, &output, &prior);
                        assert_eq!(memory.used_bytes(), before);
                    }
                }
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
