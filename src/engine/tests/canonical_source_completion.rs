//! Behavioral acceptance for the canonical frontend, independent of IR fingerprints.
//! These examples retain their specified results while lowering is completed.
#![cfg(all(feature = "source", feature = "resident-artifact"))]

use mech_core::snapshot::{MapEntryDraft, NamedValueDraft, TableColumnDraft};
use mech_core::{
    FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft as Data,
};
use mech_engine::__resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{
    ArtifactBuildError, CanonicalSourceFrontend, CanonicalSourceProgram, ControlCapture,
    ControlOperationBody, ControlParameterSource, ExecutableNodeBody, MatchPattern,
    ProgramArtifactDraft,
};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, SyntaxNode, TextSnapshot,
    VariableDefineSyntax, parse_canonical_document,
};

fn definition(source: &str) -> VariableDefineSyntax {
    fn find(node: SyntaxNode) -> Option<VariableDefineSyntax> {
        VariableDefineSyntax::cast(node.clone()).or_else(|| node.children().find_map(find))
    }
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x555), Revision(1), source).unwrap(),
        rules::VARIABLE_DEFINE,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    assert_eq!(parsed.consumed.end.0 as usize, source.len(), "{source}");
    find(parsed.syntax()).unwrap()
}

fn compile(source: &str) -> CanonicalSourceProgram {
    CanonicalSourceFrontend
        .compile_definition(&definition(source))
        .unwrap_or_else(|error| panic!("unfinished source lowering on {source:?}: {error}"))
}

fn execute<'a>(source: &str, turns: impl IntoIterator<Item = (Vec<ResidentValueRef<'a>>, Data)>) {
    let compiled = compile(source);
    let artifact = compiled
        .compile_artifact()
        .unwrap_or_else(|error| panic!("unfinished artifact lowering on {source:?}: {error:?}"));
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    assert_eq!(
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
        encoded
    );
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x555, 0),
        &decoded,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap_or_else(|error| panic!("unfinished advertised resident path on {source:?}: {error:?}"));
    for (inputs, expected) in turns {
        assert_eq!(
            inputs.len(),
            instance.plan.inputs.len(),
            "unexpected external input in {source:?}"
        );
        let inputs = inputs
            .into_iter()
            .zip(instance.plan.inputs.iter())
            .map(|(value, input)| CapturedSignalInput {
                slot: input.slot,
                value,
            })
            .collect::<Vec<_>>();
        instance
            .turn(&inputs)
            .unwrap_or_else(|error| panic!("unfinished execution on {source:?}: {error:?}"));
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected,
            "{source}"
        );
    }
}

fn compile_document(source: &str) -> CanonicalSourceProgram {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x556), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    CanonicalSourceFrontend
        .compile_document(&document)
        .unwrap_or_else(|error| panic!("unfinished document lowering on {source:?}: {error}"))
}

fn execute_document(source: &str, expected: Data) {
    let compiled = compile_document(source);
    let artifact = compiled.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    assert_eq!(
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
        encoded
    );
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x556, 0),
        &decoded,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    for _ in 0..2 {
        instance.turn(&[]).unwrap();
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            expected
        );
    }
}

fn f(value: f64) -> Data {
    Data::F64(mech_core::snapshot::F64Bits::from_f64(value))
}

fn matrix(values: &[f64]) -> Data {
    Data::Matrix(values.iter().copied().map(f).collect())
}

#[test]
fn recursive_pattern_functions_use_bounded_call_local_frames() {
    execute_document(
        "factorial(n<f64>) => <f64>\n  | 0 => 1\n  | n => n * factorial(n - 1).\nfactorial(5)\n",
        f(120.0),
    );
    execute_document(
        "fib(n<f64>) => <f64>\n  | 0 => 0\n  | 1 => 1\n  | n => fib(n - 1) + fib(n - 2).\nfib(10)\n",
        f(55.0),
    );
    execute_document(
        "countdown(n<f64>, answer<f64>) => <f64>\n  | (0, answer) => answer\n  | (n, answer) => countdown(n - 1, answer + 1).\ncountdown(20, 22)\n",
        f(42.0),
    );
    execute_document(
        "tuple-countdown(state<(f64,f64)>) => <(f64,f64)>\n  | (0, answer) => (0, answer)\n  | (n, answer) => tuple-countdown((n - 1, answer + 1)).\ntuple-countdown((5, 37))\n",
        Data::Tuple(vec![f(0.0), f(42.0)].into_boxed_slice()),
    );
    execute_document(
        "nested-countdown(n<f64>) => <f64>\n  | 0 => 0\n  | n => (n > 0 ? | true => nested-countdown(n - 1) | false => 0).\nnested-countdown(5)\n",
        f(0.0),
    );
}

#[test]
fn recursive_target_cannot_capture_its_own_scrutinee() {
    for source in [
        "zero(n<f64>) => <f64>\n  | 0 => n\n  | x => zero(x - 1).\nzero(3)\n",
        "zero(n<f64>) => <f64>\n  | 0 => 0\n  | * => zero(n - 1).\nzero(3)\n",
    ] {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x556), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
        let error = CanonicalSourceFrontend
            .compile_document(&document)
            .err()
            .unwrap();
        assert_eq!(error.code, "source-semantics/recursive-function-capture");
    }
}

#[test]
fn recursive_capture_check_respects_nested_bindings() {
    let source =
        "shadow(n<f64>) => <f64>\n  | 0 => (1 ? | n => n)\n  | n => shadow(n - 1).\nshadow(0)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x556), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let compiled = CanonicalSourceFrontend
        .compile_document(&document)
        .unwrap_or_else(|error| panic!("{source}: {error}"));
    compiled.compile_artifact().unwrap();
}

#[test]
fn nested_recursive_helper_calls_its_own_pattern_match() {
    execute_document(
        "outer(n<f64>) => <f64>\n  | 0 => 0\n  | n => helper(n).\nhelper(n<f64>) => <f64>\n  | 0 => 1\n  | n => helper(n - 1).\nouter(3)\n",
        f(1.0),
    );
}

#[test]
fn recursion_inside_a_nested_match_keeps_the_outer_function_target() {
    execute_document(
        "walk(n<f64>) => <f64>\n  | 0 => 0\n  | n => ((n > 0) ? | true => walk(n - 1) | * => 0).\nwalk(3)\n",
        f(0.0),
    );
}

#[test]
fn recursive_calls_inside_comprehensions_fail_at_the_source_boundary() {
    let source = "walk(n<f64>) => <f64>\n  | 0 => 0\n  | n => [((n > 0) ? | true => walk(n - 1) | * => 0) | x <- [1]].\nwalk(2)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x556), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    let error = CanonicalSourceFrontend
        .compile_document(&document)
        .err()
        .expect("recursive calls inside comprehensions must fail before artifact creation");
    assert_eq!(error.code, "source-semantics/recursive-comprehension");
}

#[test]
fn recursive_pattern_function_lifts_fail_at_the_source_boundary() {
    for call in ["walk([1 2])", "walk({1, 2})"] {
        let source = format!("walk(n<f64>) => <f64>\n  | 0 => 0\n  | n => walk(n - 1).\n{call}\n");
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x556), Revision(1), source.clone()).unwrap(),
            ParseConfig::default(),
        );
        let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
        let error = CanonicalSourceFrontend
            .compile_document(&document)
            .err()
            .unwrap_or_else(|| panic!("{call} must reject recursive implicit lifting"));
        assert_eq!(error.code, "source-semantics/recursive-comprehension");
    }
}

#[test]
fn artifact_rejects_direct_bind_as_a_recursive_target() {
    let artifact = compile_document(
        "countdown(n<f64>) => <f64>\n  | 0 => 0\n  | n => countdown(n - 1).\ncountdown(2)\n",
    )
    .compile_artifact()
    .unwrap();
    let mut draft = ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().into(),
        slots: artifact.slots().into(),
        nodes: artifact.nodes().into(),
        bindings: artifact.bindings().into(),
        outputs: artifact.outputs().into(),
        constraints: artifact.constraints().into(),
        compute_regions: artifact.compute_regions().into(),
    };
    let control = draft
        .nodes
        .iter_mut()
        .find_map(|node| match &mut node.body {
            ExecutableNodeBody::Match(control) => Some(control),
            _ => None,
        })
        .unwrap();
    let recursive_arm = control
        .arms
        .iter_mut()
        .find(|arm| {
            arm.body
                .operations
                .iter()
                .any(|operation| matches!(&operation.body, ControlOperationBody::Recur(_)))
        })
        .unwrap();
    assert!(matches!(
        &recursive_arm.pattern,
        MatchPattern::Structural(_)
    ));
    recursive_arm.pattern = MatchPattern::Bind;
    for parameter in recursive_arm.body.parameters.iter_mut() {
        if matches!(parameter.source, ControlParameterSource::PatternBinding(_)) {
            parameter.source = ControlParameterSource::Scrutinee;
        }
    }
    let error = draft.finalize().err().unwrap();
    assert!(matches!(
        error,
        ArtifactBuildError::InvalidControl {
            reason: "recursive target cannot use a direct bind pattern",
            ..
        }
    ));
}

#[test]
fn artifact_rejects_recursive_target_capture_of_its_scrutinee() {
    let artifact = compile_document(
        "countdown(n<f64>) => <f64>\n  | 0 => 0\n  | n => countdown(n - 1).\ncountdown(2)\n",
    )
    .compile_artifact()
    .unwrap();
    let mut draft = ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().into(),
        slots: artifact.slots().into(),
        nodes: artifact.nodes().into(),
        bindings: artifact.bindings().into(),
        outputs: artifact.outputs().into(),
        constraints: artifact.constraints().into(),
        compute_regions: artifact.compute_regions().into(),
    };
    let control = draft
        .nodes
        .iter_mut()
        .find_map(|node| match &mut node.body {
            ExecutableNodeBody::Match(control) => Some(control),
            _ => None,
        })
        .unwrap();
    let schema = control
        .arms
        .iter()
        .find_map(|arm| match &arm.pattern {
            MatchPattern::Literal(constant) => {
                draft.constants.get(*constant).map(|value| value.schema())
            }
            _ => None,
        })
        .unwrap();
    let mut captures = control.captures.to_vec();
    captures.push(ControlCapture {
        input: control.scrutinee,
        schema,
        freeze_on_suspend: false,
    });
    control.captures = captures.into_boxed_slice();
    assert!(matches!(
        draft.finalize(),
        Err(ArtifactBuildError::InvalidControl {
            reason: "recursive target cannot capture its own scrutinee",
            ..
        })
    ));
}

#[test]
fn direct_recursion_at_the_eighth_lexical_function_level_is_admitted() {
    execute_document(
        "f1(n<f64>) => <f64> | n => f2(n).\n\
         f2(n<f64>) => <f64> | n => f3(n).\n\
         f3(n<f64>) => <f64> | n => f4(n).\n\
         f4(n<f64>) => <f64> | n => f5(n).\n\
         f5(n<f64>) => <f64> | n => f6(n).\n\
         f6(n<f64>) => <f64> | n => f7(n).\n\
         f7(n<f64>) => <f64> | n => f8(n).\n\
         f8(n<f64>) => <f64> | 0 => 0 | n => f8(n - 1).\n\
         f1(2)\n",
        f(0.0),
    );
}

#[test]
fn structural_root_bind_remains_exhaustive() {
    execute(
        "result := signal<bool> ? | value => value",
        [(vec![ResidentValueRef::Bool(&[1])], Data::Bool(true))],
    );
}

#[test]
fn recursive_match_initialization_schedules_downstream_output() {
    let source = "countdown(n<f64>) => <f64>\n  | 0 => 0\n  | n => countdown(n - 1).\nresult := countdown(signal<f64>) + 1\nresult\n";
    let compiled = compile_document(source);
    let artifact = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x556, 2),
        &decoded,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let slot = instance.plan.inputs[0].slot;
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::F64(&[5.0]),
        }])
        .unwrap();
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        f(1.0)
    );
}

#[test]
fn recursive_identity_result_updates_downstream_on_the_next_turn() {
    let source = "countdown(n<f64>, answer<f64>) => <f64>\n  | (0, answer) => answer\n  | (n, answer) => countdown(n - 1, answer).\nresult := countdown(3, signal<f64>) + 1\nresult\n";
    let artifact = compile_document(source).compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x556, 3),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let slot = instance.plan.inputs[0].slot;
    for (answer, expected) in [(2.0, 3.0), (5.0, 6.0)] {
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&[answer]),
            }])
            .unwrap();
        assert_eq!(
            instance
                .copied_output(0)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            f(expected)
        );
    }
}

#[test]
fn recursive_frame_limit_rolls_back_and_allows_retry() {
    let source = "countdown(n<f64>) => <f64>\n  | 0 => 0\n  | n => countdown(n - 1).\ncountdown(signal<f64>)\n";
    let compiled = compile_document(source);
    let artifact = compiled.compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x556, 1),
        &decoded,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let slot = instance.plan.inputs[0].slot;
    assert!(
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::F64(&[300.0]),
            }])
            .is_err()
    );
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::F64(&[5.0]),
        }])
        .unwrap();
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        f(0.0)
    );
}

#[test]
fn promoted_arithmetic_executes_with_live_operands() {
    execute(
        "x := 250u8 + signal<f64>",
        [
            (vec![ResidentValueRef::F64(&[10.0])], f(260.0)),
            (vec![ResidentValueRef::F64(&[1.0])], f(251.0)),
        ],
    );
}

#[test]
fn expected_error_families_also_have_positive_call_and_literal_execution() {
    execute(
        "x := math/add(signal<f64>, 2)",
        [
            (vec![ResidentValueRef::F64(&[1.0])], f(3.0)),
            (vec![ResidentValueRef::F64(&[7.0])], f(9.0)),
        ],
    );
    execute("x := 1", [(Vec::new(), f(1.0)), (Vec::new(), f(1.0))]);
}

#[test]
fn table_join_has_real_resident_output_after_artifact_roundtrip() {
    let expected = |right| {
        Data::Table(
            vec![
                TableColumnDraft {
                    name: "a".to_owned(),
                    values: vec![Data::U8(1)].into_boxed_slice(),
                },
                TableColumnDraft {
                    name: "left".to_owned(),
                    values: vec![Data::U8(7)].into_boxed_slice(),
                },
                TableColumnDraft {
                    name: "right".to_owned(),
                    values: vec![f(right)].into_boxed_slice(),
                },
            ]
            .into_boxed_slice(),
        )
    };
    // Both operands have unmatched rows and distinct payload columns. The
    // right payload changes after activation, so neither passthrough, a cross
    // product, nor a cached result can satisfy both expected turns.
    execute(
        "x := (|a<u8> left<u8>|1u8 7u8|2u8 8u8|) ⋈ (|a<u8> right<f64>|1u8 signal<f64>|3u8 90|)",
        [
            (vec![ResidentValueRef::F64(&[3.0])], expected(3.0)),
            (vec![ResidentValueRef::F64(&[9.0])], expected(9.0)),
        ],
    );
}

#[test]
fn nested_structured_values_execute_and_preserve_field_column_and_key_order() {
    for value in [3.0, 7.0] {
        let input = [value];
        execute(
            "x := ({a: signal<f64>, b: true}, {2: signal<f64>, 1: 9}, (|left<f64> right<bool>|signal<f64> true|9 false|))",
            [(
                vec![ResidentValueRef::F64(&input)],
                Data::Tuple(
                    vec![
                        Data::Record(
                            vec![
                                NamedValueDraft {
                                    name: "a".to_owned(),
                                    value: f(value),
                                },
                                NamedValueDraft {
                                    name: "b".to_owned(),
                                    value: Data::Bool(true),
                                },
                            ]
                            .into_boxed_slice(),
                        ),
                        Data::Map(
                            vec![
                                MapEntryDraft {
                                    items: vec![f(1.0), f(9.0)].into_boxed_slice(),
                                },
                                MapEntryDraft {
                                    items: vec![f(2.0), f(value)].into_boxed_slice(),
                                },
                            ]
                            .into_boxed_slice(),
                        ),
                        Data::Table(
                            vec![
                                TableColumnDraft {
                                    name: "left".to_owned(),
                                    values: vec![f(value), f(9.0)].into_boxed_slice(),
                                },
                                TableColumnDraft {
                                    name: "right".to_owned(),
                                    values: vec![Data::Bool(true), Data::Bool(false)]
                                        .into_boxed_slice(),
                                },
                            ]
                            .into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                ),
            )],
        );
    }
}

#[test]
fn selection_reads_live_matrix_values_in_canonical_order() {
    execute(
        "x := (signal<[f64]:2,2>, signal[[false true],:])",
        [
            (
                vec![ResidentValueRef::F64(&[1.0, 3.0, 2.0, 4.0])],
                Data::Tuple(
                    vec![matrix(&[1.0, 2.0, 3.0, 4.0]), matrix(&[3.0, 4.0])].into_boxed_slice(),
                ),
            ),
            (
                vec![ResidentValueRef::F64(&[10.0, 30.0, 20.0, 40.0])],
                Data::Tuple(
                    vec![matrix(&[10.0, 20.0, 30.0, 40.0]), matrix(&[30.0, 40.0])]
                        .into_boxed_slice(),
                ),
            ),
        ],
    );
}

#[test]
fn matching_executes_patterns_guards_and_bound_arm_results() {
    execute(
        "x := signal<f64> ? | 0 => 10 | item, item > 0 => item + 1 | * => -1",
        [
            (vec![ResidentValueRef::F64(&[0.0])], f(10.0)),
            (vec![ResidentValueRef::F64(&[4.0])], f(5.0)),
            (vec![ResidentValueRef::F64(&[-2.0])], f(-1.0)),
        ],
    );
}

#[test]
fn match_publishes_the_selected_compound_result_across_turns() {
    execute(
        "x := signal<f64> ? | 0 => (1, true) | * => (2, false)",
        [
            (
                vec![ResidentValueRef::F64(&[0.0])],
                Data::Tuple(vec![f(1.0), Data::Bool(true)].into_boxed_slice()),
            ),
            (
                vec![ResidentValueRef::F64(&[4.0])],
                Data::Tuple(vec![f(2.0), Data::Bool(false)].into_boxed_slice()),
            ),
            (
                vec![ResidentValueRef::F64(&[0.0])],
                Data::Tuple(vec![f(1.0), Data::Bool(true)].into_boxed_slice()),
            ),
        ],
    );
}

#[test]
fn structural_match_patterns_bind_guard_and_fall_through_after_roundtrip() {
    execute(
        "x := (1, 2) ? | (left, right), left > 0 => left + right | * => 0",
        [(Vec::new(), f(3.0)), (Vec::new(), f(3.0))],
    );
    execute(
        "x := (1, 2) ? | (same, same) => 99 | * => 0",
        [(Vec::new(), f(0.0)), (Vec::new(), f(0.0))],
    );
    execute(
        "x := [1 2 3] ? | [head, ..., tail] => head + tail | * => 0",
        [(Vec::new(), f(4.0)), (Vec::new(), f(4.0))],
    );
    execute(
        "x := [1 2 3] ? | [head | [2, 3]] => head | * => 0",
        [(Vec::new(), f(1.0)), (Vec::new(), f(1.0))],
    );
    execute(
        "x := [1 2 3] ? | [head | rest] => head | * => 0",
        [(Vec::new(), f(1.0)), (Vec::new(), f(1.0))],
    );
    execute(
        "x := [1 2 3] ? | [head | rest] => rest[1] + rest[2] | * => 0",
        [(Vec::new(), f(5.0)), (Vec::new(), f(5.0))],
    );
    execute(
        "x := :Point((1, 2)) ? | :Point(left, right) => left + right | * => 0",
        [(Vec::new(), f(3.0)), (Vec::new(), f(3.0))],
    );
    execute(
        "x := ((1, 2), 3) ? | (pair, *) => pair | * => (0, 0)",
        [
            (
                Vec::new(),
                Data::Tuple(vec![f(1.0), f(2.0)].into_boxed_slice()),
            ),
            (
                Vec::new(),
                Data::Tuple(vec![f(1.0), f(2.0)].into_boxed_slice()),
            ),
        ],
    );
}

#[test]
fn sequential_structural_match_arms_admit_peak_clone_depth() {
    let source = "x := signal<(string,bool)> ? | (text, false) => 0 | (text, true) => 1 | * => 2";
    let artifact = compile(source).compile_artifact().unwrap();
    let input_schema = artifact.inputs()[0].schema;
    let payload = "x".repeat(10);
    let input = mech_core::ValueDraft {
        schema: input_schema,
        shape_values: Box::new([]),
        data: Data::Tuple(vec![Data::String(payload), Data::Bool(true)].into_boxed_slice()),
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        artifact.schemas(),
    ))
    .unwrap();
    execute(
        source,
        [(vec![ResidentValueRef::Snapshot(&[Some(input)])], f(1.0))],
    );
}

#[test]
fn structural_match_patterns_follow_live_scrutinee_values_without_binding_leaks() {
    let source = "x := signal<(f64,f64)> ? | (left, right), left > 0 => left + right | * => 0";
    let artifact = compile(source).compile_artifact().unwrap();
    let input_schema = artifact.inputs()[0].schema;
    let value = |left, right| {
        mech_core::ValueDraft {
            schema: input_schema,
            shape_values: Box::new([]),
            data: Data::Tuple(vec![f(left), f(right)].into_boxed_slice()),
        }
        .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
            artifact.schemas(),
        ))
        .unwrap()
    };
    let inputs = [[Some(value(1.0, 2.0))], [Some(value(-1.0, 4.0))]];
    execute(
        source,
        inputs
            .iter()
            .zip([f(3.0), f(0.0)])
            .map(|(input, expected)| (vec![ResidentValueRef::Snapshot(input)], expected)),
    );

    let source = "x := signal<(f64,f64)> ? | (same, same) => 99 | * => 0";
    let artifact = compile(source).compile_artifact().unwrap();
    let input_schema = artifact.inputs()[0].schema;
    let value = |left, right| {
        mech_core::ValueDraft {
            schema: input_schema,
            shape_values: Box::new([]),
            data: Data::Tuple(vec![f(left), f(right)].into_boxed_slice()),
        }
        .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
            artifact.schemas(),
        ))
        .unwrap()
    };
    let inputs = [[Some(value(2.0, 2.0))], [Some(value(2.0, 3.0))]];
    execute(
        source,
        inputs
            .iter()
            .zip([f(99.0), f(0.0)])
            .map(|(input, expected)| (vec![ResidentValueRef::Snapshot(input)], expected)),
    );
}

#[test]
fn identical_key_tables_join_after_artifact_roundtrip() {
    execute(
        "x := (|a<u8>|1u8|) ⋈ (|a<u8>|1u8|)",
        [(
            Vec::new(),
            Data::Table(
                vec![TableColumnDraft {
                    name: "a".to_owned(),
                    values: vec![Data::U8(1)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        )],
    );
}

#[test]
fn nested_matches_keep_outer_bindings_and_publish_selected_structures() {
    let result = |value| Data::Tuple(vec![f(value), Data::Bool(true)].into_boxed_slice());
    execute(
        "x := signal<f64> ? | item => ((item + 1) ? | inner => (inner + item, true))",
        [
            (vec![ResidentValueRef::F64(&[0.0])], result(1.0)),
            (vec![ResidentValueRef::F64(&[4.0])], result(9.0)),
            (vec![ResidentValueRef::F64(&[0.0])], result(1.0)),
        ],
    );
}

#[test]
fn comprehension_executes_generator_filter_and_yield() {
    execute(
        "x := [item + 1 | item <- [1 2 3], item > 1]",
        [(vec![], matrix(&[3.0, 4.0]))],
    );
    execute(
        "x := {item + 1 | item <- {1,2,3}, item > 1}",
        [(vec![], Data::Set(vec![f(3.0), f(4.0)].into_boxed_slice()))],
    );
}

#[test]
fn comprehensions_execute_nested_canonical_control() {
    execute(
        "x := [(item ? | 1 => 10 | * => 20) | item <- [1 2]]",
        [(vec![], matrix(&[10.0, 20.0]))],
    );
    execute(
        "x := [[item + z | z <- [1 2]] | item <- [1 2]]",
        [(
            vec![],
            Data::Matrix(vec![matrix(&[2.0, 3.0]), matrix(&[3.0, 4.0])].into_boxed_slice()),
        )],
    );
    execute(
        "x := [[item + z | z <- [2 3]] | item <- [2 3]]",
        [(
            vec![],
            Data::Matrix(vec![matrix(&[4.0, 5.0]), matrix(&[5.0, 6.0])].into_boxed_slice()),
        )],
    );
    execute(
        "x := [[item + z | z <- [1 2]] | item <- [1 2], false]",
        [(vec![], Data::Matrix(Box::new([])))],
    );
}

#[test]
fn parameterized_operation_over_nested_comprehension_activates() {
    let source = "out := [[z | z <- rest] + rest | [head | rest] <- signal<[[f64]:1,3]:1,2>]";
    let artifact = compile(source).compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    activate(
        ReactiveInstanceId::new(0x557, 0),
        &decoded,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap_or_else(|error| {
        panic!("turn-shaped ordinary-operation local must activate: {error:?}")
    });
}

#[test]
fn parameterized_match_intermediate_operation_activates() {
    let source = "out := signal<[f64]:1,3> ? | [head | rest] => stats/sum/column(stats/sum/row([z | z <- rest] + rest)) | * => [0.0]";
    let artifact = compile(source).compile_artifact().unwrap();
    let matched = artifact
        .nodes()
        .iter()
        .find_map(|node| match &node.body {
            mech_engine::ExecutableNodeBody::Match(matched) => Some(matched),
            _ => None,
        })
        .expect("match node");
    assert!(matched.arms[0].body.operations.iter().any(|operation| {
        matches!(
            operation.body,
            mech_engine::ControlOperationBody::Operation { .. }
        ) && !artifact
            .schemas()
            .get(operation.schema)
            .unwrap()
            .dimension_parameters()
            .is_empty()
    }));
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x558, 0),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap_or_else(|error| panic!("turn-shaped match intermediate must activate: {error:?}"));
    let slot = instance.plan.inputs[0].slot;
    instance
        .turn(&[CapturedSignalInput {
            slot,
            value: ResidentValueRef::F64(&[1.0, 2.0, 3.0]),
        }])
        .expect("turn-shaped match intermediates must execute");
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        matrix(&[10.0])
    );
}

#[test]
fn comprehension_computed_patterns_evaluate_in_lexical_order() {
    execute(
        "y := [1 | signal<f64> + 1 <- [2 3]]",
        [
            (vec![ResidentValueRef::F64(&[1.0])], matrix(&[1.0])),
            (vec![ResidentValueRef::F64(&[2.0])], matrix(&[1.0])),
            (
                vec![ResidentValueRef::F64(&[3.0])],
                Data::Matrix(Box::new([])),
            ),
        ],
    );
    execute(
        "y := [x | x <- [1 2], x + 1 <- [2 4]]",
        [(vec![], matrix(&[1.0]))],
    );
    execute(
        "y := {x | (x, 1 + 1) <- {(1, 2)}}",
        [(vec![], Data::Set(vec![f(1.0)].into_boxed_slice()))],
    );
}

#[test]
fn comprehension_rejects_computed_pattern_with_incompatible_schema() {
    let source = "y := [1 | 1 > 0 <- [2 3]]";
    let error = CanonicalSourceFrontend
        .compile_definition(&definition(source))
        .err()
        .expect("computed pattern must match its generator element before artifact lowering");
    assert_eq!(
        error.code,
        "source-semantics/unsupported-comprehension-control"
    );
    assert!(
        error.message.contains("computed pattern differs"),
        "{error:?}"
    );
}

#[test]
fn nested_comprehension_rejects_inconsistent_element_shapes_before_publish() {
    let source = "x := [[z | z <- [1 2], z <= item] | item <- [1 2]]";
    let artifact = compile(source).compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let error = activate(
        ReactiveInstanceId::new(0x556, 0),
        &decoded,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .expect_err("different yielded element shapes cannot publish one outer matrix");
    assert!(matches!(
        error,
        mech_engine::__resident::ResidentActivationError::ActivationKernel { .. }
    ));
}

#[test]
fn mutable_definition_publishes_its_resolved_initial_state() {
    execute(
        "~state<u8> := 1",
        [(vec![], Data::U8(1)), (vec![], Data::U8(1))],
    );
}

#[test]
fn unresolved_empty_and_unknown_calls_are_anchored_user_errors() {
    for (source, expected, offending) in [
        (
            "x := _",
            "source-semantics/unresolved-empty-expression",
            "_",
        ),
        (
            "x := not-declared(1)",
            "source-semantics/unknown-function",
            "not-declared",
        ),
    ] {
        let syntax = definition(source);
        let error = CanonicalSourceFrontend
            .compile_definition(&syntax)
            .err()
            .expect("invalid source cannot become a placeholder executable node");
        assert_eq!(error.code, expected, "{source}");
        assert_eq!(error.anchor.document, DocumentId(0x555));
        assert_eq!(error.anchor.revision, Revision(1));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            offending,
            "{source}",
        );
    }
}

#[test]
fn structured_patterns_read_live_components_and_reject_partial_matches() {
    let tuple = |a, b| Data::Tuple(vec![f(a), f(b)].into_boxed_slice());
    let row_pair = |left: &[f64], right: &[f64]| {
        Data::Tuple(vec![matrix(left), matrix(right)].into_boxed_slice())
    };
    for (source, inputs, expected) in [
        (
            "out := [x + y | (x,y) <- signal<[(f64,f64)]:1,2>]",
            [
                vec![tuple(1.0, 2.0), tuple(3.0, 4.0)],
                vec![tuple(8.0, 2.0), tuple(3.0, 4.0)],
            ],
            [vec![3.0, 7.0], vec![10.0, 7.0]],
        ),
        (
            "out := [x | (x,x) <- signal<[(f64,f64)]:1,2>]",
            [
                vec![tuple(2.0, 2.0), tuple(3.0, 3.0)],
                vec![tuple(8.0, 2.0), tuple(3.0, 3.0)],
            ],
            [vec![2.0, 3.0], vec![3.0]],
        ),
        (
            "out := [x | [x,x] <- signal<[[f64]:1,2]:1,2>]",
            [
                vec![matrix(&[2.0, 2.0]), matrix(&[3.0, 3.0])],
                vec![matrix(&[8.0, 2.0]), matrix(&[3.0, 3.0])],
            ],
            [vec![2.0, 3.0], vec![3.0]],
        ),
        (
            "out := [rest[1] + rest[2] | [head | rest] <- signal<[[f64]:1,3]:1,2>]",
            [
                vec![matrix(&[1.0, 2.0, 3.0]), matrix(&[4.0, 5.0, 6.0])],
                vec![matrix(&[7.0, 8.0, 9.0]), matrix(&[10.0, 11.0, 12.0])],
            ],
            [vec![5.0, 11.0], vec![17.0, 23.0]],
        ),
        (
            "out := [head | ([head | rest], [other | rest]) <- signal<[([f64]:1,3,[f64]:1,3)]:1,2>]",
            [
                vec![
                    row_pair(&[1.0, 2.0, 3.0], &[9.0, 2.0, 3.0]),
                    row_pair(&[4.0, 5.0, 6.0], &[8.0, 5.0, 7.0]),
                ],
                vec![
                    row_pair(&[7.0, 8.0, 9.0], &[0.0, 8.0, 9.0]),
                    row_pair(&[10.0, 11.0, 12.0], &[1.0, 11.0, 13.0]),
                ],
            ],
            [vec![1.0], vec![7.0]],
        ),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let inputs = inputs.map(|items| {
            [Some(
                mech_core::ValueDraft {
                    schema: artifact.inputs()[0].schema,
                    shape_values: Box::new([]),
                    data: Data::Matrix(items.into_boxed_slice()),
                }
                .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                    artifact.schemas(),
                ))
                .unwrap(),
            )]
        });
        execute(
            source,
            inputs.iter().zip(expected).map(|(input, expected)| {
                (vec![ResidentValueRef::Snapshot(input)], matrix(&expected))
            }),
        );
    }
}
