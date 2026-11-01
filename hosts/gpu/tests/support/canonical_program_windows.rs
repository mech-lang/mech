//! General range, String-selection and matrix-conversion machine-code tests.
use super::{artifacts, check, equal, inputs, limits, reference, reference_result};
use mech_core::{ManagedMemoryBudget, ResidentKernelError, Value, ValueDataDraft};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeProgramExecutionError, emit_program_object,
};

pub(super) const WINDOWS_FUNCTION: &str = include_str!("../fixtures/native/windows.mec");

fn indices(values: &[u64]) -> ValueDataDraft {
    ValueDataDraft::Matrix(values.iter().copied().map(ValueDataDraft::U64).collect())
}

#[test]
fn encoded_one_input_string_all_uses_the_actual_maintained_factory() {
    use mech_core::{BindingId, OperationContractTableBuilder};
    use mech_engine::{BindingDeclaration as Binding, ProgramArtifactDraft};
    let base = artifacts("s := input<string>\ns[positions<[u64]>]\n")
        .0
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(base.nodes().len(), 1);
    let mut nodes = base.nodes().to_vec();
    let mech_engine::ExecutableNodeBody::Operation(operation) = &mut nodes[0].body else {
        panic!("one actual access/range call")
    };
    assert_eq!(operation.operation.operation_name, "range");
    let mut contract = base.contracts().get(operation.contract).unwrap().clone();
    let mech_core::ResolvedOperationContract::Declared(call) = &mut contract else {
        panic!("maintained declared contract")
    };
    call.inputs = vec![call.inputs[0].clone()].into_boxed_slice();
    let mut contracts = OperationContractTableBuilder::new();
    let pending = contracts.insert(contract).unwrap();
    let build = contracts.finish().unwrap();
    operation.contract = build.resolve(pending).unwrap();
    let contracts = build.into_parts().0;
    nodes[0].input_bindings = 0..1;
    nodes[0].output_bindings = 1..2;
    let source = match base.bindings()[0] {
        Binding::Input { source, .. } => source,
        _ => panic!("source binding"),
    };
    let target = match base.bindings().last().unwrap() {
        Binding::Output { target, .. } => *target,
        _ => panic!("output binding"),
    };
    let node = nodes[0].node;
    let original = ProgramArtifactDraft {
        schemas: base.schemas().clone(),
        constants: base.constants().clone(),
        contracts,
        requirements: base.requirements().clone(),
        inputs: base.inputs().into(),
        slots: base.slots().into(),
        nodes: nodes.into_boxed_slice(),
        bindings: Box::new([
            Binding::Input {
                id: BindingId::new(0),
                node,
                port_ordinal: 0,
                source,
            },
            Binding::Output {
                id: BindingId::new(1),
                node,
                port_ordinal: 0,
                target,
            },
        ]),
        outputs: base.outputs().into(),
        constraints: base.constraints().into(),
        compute_regions: base.compute_regions().into(),
    }
    .finalize()
    .unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let template = mech_engine::native_artifact::native_collection_operation_template(
            &artifact,
            node,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        assert_eq!(
            template.primitive(),
            mech_engine::native_artifact::NativeCollectionPrimitive::StringAllAccess
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        for text in ["é🧑‍🚀é", "", "é"] {
            let args = inputs(
                &artifact,
                &[
                    ("input", ValueDataDraft::String(text.into())),
                    ("positions", indices(&[])),
                ],
                &memory,
            );
            let expected = reference(&artifact, &args);
            assert_eq!(
                expected.last().unwrap().canonical_data_draft().unwrap(),
                ValueDataDraft::String(text.into())
            );
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn three_operand_ranges_execute_the_actual_step_in_roots_and_functions() {
    for inclusive in [false, true] {
        let end = if inclusive { "..=" } else { ".." };
        for lexical in [false, true] {
            let expression = format!("start<u64>..increment<u64>{end}end<u64>");
            let source = if lexical {
                format!(
                    "stride := increment<u64>\nlimit := end<u64>\nchoose := flag<bool>\nrange(x<u64>) => (out<[u64]>) := out := (choose? | true => x..stride{end}limit | false => x..stride{end}limit).\nrange(start<u64>)\n"
                )
            } else {
                format!("{expression}\n")
            };
            for (start, increment, end, exclusive, including) in [
                (1, 1, 5, &[1, 2, 3, 4][..], &[1, 2, 3, 4, 5][..]),
                (2, 3, 11, &[2, 5, 8][..], &[2, 5, 8, 11][..]),
                (8, 1, 9, &[8][..], &[8, 9][..]),
            ] {
                let named = [
                    ("start", ValueDataDraft::U64(start)),
                    ("increment", ValueDataDraft::U64(increment)),
                    ("end", ValueDataDraft::U64(end)),
                    ("flag", ValueDataDraft::Bool(true)),
                ];
                if lexical {
                    check(&source, &named);
                } else {
                    // Live roots must work too, not become a permanently
                    // unsupported target exception. Check independent literal
                    // results alongside the ordinary resident implementation.
                    for artifact in artifacts(&source).0 {
                        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                        let args = inputs(&artifact, &named, &memory);
                        let ordinary = reference(&artifact, &args);
                        let jit =
                            JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog())
                                .unwrap();
                        let mut output = vec![None; artifact.outputs().len()];
                        jit.evaluate_into(&args, &mut output, limits(), &memory)
                            .unwrap();
                        equal(&artifact, &output, &ordinary);
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            indices(if inclusive { including } else { exclusive })
                        );
                        let object =
                            emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
                        assert_eq!(
                            object.descriptor().typed_ir_sha256(),
                            jit.descriptor().typed_ir_sha256()
                        );
                        drop((args, output, ordinary));
                        assert_eq!(memory.used_bytes(), 0);
                    }
                }
            }
        }
    }
}

#[test]
fn canonical_string_at_gather_mask_and_all_keep_raw_graphemes_and_order() {
    for (expression, selector, expected) in [
        (
            "s[position<u64>]",
            Some(("position", ValueDataDraft::U64(2))),
            "🧑‍🚀",
        ),
        (
            "s[positions<[u64]>]",
            Some(("positions", indices(&[3, 1, 3, 2]))),
            "ééé🧑‍🚀",
        ),
        ("s[positions<[u64]>]", Some(("positions", indices(&[]))), ""),
        (
            "s[mask<[bool]>]",
            Some((
                "mask",
                ValueDataDraft::Matrix(
                    vec![
                        ValueDataDraft::Bool(true),
                        ValueDataDraft::Bool(false),
                        ValueDataDraft::Bool(true),
                    ]
                    .into_boxed_slice(),
                ),
            )),
            "éé",
        ),
        ("s[:]", None, "é🧑‍🚀é"),
    ] {
        let source = format!("s := input<string>\n{expression}\n");
        let mut named = vec![("input", ValueDataDraft::String("é🧑‍🚀é".into()))];
        if let Some(selector) = selector {
            named.push(selector);
        }
        for artifact in artifacts(&source).0 {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(&artifact, &named, &memory);
            let ordinary = reference(&artifact, &args);
            assert_eq!(
                ordinary.last().unwrap().canonical_data_draft().unwrap(),
                ValueDataDraft::String(expected.into())
            );
        }
        check(&source, &named);
    }
}

#[test]
fn native_string_selection_errors_preserve_prior_publication_and_recover() {
    let source = "s := input<string>\ns[position<u64>]\n";
    for artifact in artifacts(source).0 {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output: Vec<Option<Value>> = vec![None; artifact.outputs().len()];
        for position in [1, 0, 4, 2] {
            let args = inputs(
                &artifact,
                &[
                    ("input", ValueDataDraft::String("é🧑‍🚀é".into())),
                    ("position", ValueDataDraft::U64(position)),
                ],
                &memory,
            );
            if position == 0 || position == 4 {
                let ordinary = reference_result(&artifact, &args).unwrap_err();
                let expected = if position == 0 {
                    ResidentKernelError::InvalidInput
                } else {
                    ResidentKernelError::IndexOutOfRange {
                        index: 4,
                        upper_bound: 3,
                    }
                };
                assert!(ordinary.contains(&format!("{expected:?}")), "{ordinary}");
                let prior = output
                    .iter()
                    .map(|value| value.as_ref().unwrap().clone())
                    .collect::<Vec<_>>();
                let before = memory.used_bytes();
                let Err(NativeProgramExecutionError::Library {
                    error:
                        mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(actual),
                    ..
                }) = jit.evaluate_into(&args, &mut output, limits(), &memory)
                else {
                    panic!("selection must fail at the maintained kernel")
                };
                assert_eq!(actual, expected);
                equal(&artifact, &output, &prior);
                assert_eq!(memory.used_bytes(), before);
            } else {
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                equal(&artifact, &output, &reference(&artifact, &args));
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn shallow_matrix_conversion_keeps_live_shapes_and_numeric_cast_semantics() {
    for source in [
        "converted<[f64]> := values<[u64]>\nconverted\n",
        "cast(x<[u64]>) => (out<[f64]>) := out := x.\ncast(values<[u64]>)\n",
        "identity(x<[string]>) => (out<[string]>) := out := x.\nidentity(values<[string]>)\n",
    ] {
        for count in [3, 0, 1, 5] {
            let values = if source.contains("string") {
                ValueDataDraft::Matrix(
                    (0..count)
                        .map(|i| ValueDataDraft::String(if i % 2 == 0 { "é" } else { "🧑‍🚀" }.into()))
                        .collect(),
                )
            } else {
                indices(&(1..=count).collect::<Vec<_>>())
            };
            check(source, &[("values", values)]);
        }
    }
}
