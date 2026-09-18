//! Behavioral acceptance for the canonical frontend, independent of IR fingerprints.
//! These examples retain their specified results while lowering is completed.
#![cfg(all(feature = "source", feature = "resident-artifact"))]

use mech_core::snapshot::{MapEntryDraft, NamedValueDraft, TableColumnDraft};
use mech_core::{
    FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft as Data,
};
use mech_engine::__resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2i_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, SyntaxNode, TextSnapshot, VariableDefineSyntax,
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

fn f(value: f64) -> Data {
    Data::F64(mech_core::snapshot::F64Bits::from_f64(value))
}

fn matrix(values: &[f64]) -> Data {
    Data::Matrix(values.iter().copied().map(f).collect())
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
