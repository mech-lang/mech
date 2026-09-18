#![cfg(all(feature = "source_default", feature = "resident-artifact"))]

use mech_core::{
    FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, SchemaBody, Value, ValueData,
    snapshot::SequenceView,
};
use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram, SourceNodeOutput};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document, reconstruct_source,
};

fn document(source: &str) -> DocumentSyntax {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x570), Revision(7), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.diagnostics.is_empty(),
        "{source:?}: {:?}",
        parsed.diagnostics
    );
    assert_eq!(
        reconstruct_source(&parsed.root, &parsed.source).unwrap(),
        source
    );
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    assert!(!document.syntax().flags().intersects(
        mech_syntax::document::NodeFlags::ERROR | mech_syntax::document::NodeFlags::CONTAINS_ERROR
    ));
    document
}

fn compiled(source: &str) -> CanonicalSourceProgram {
    CanonicalSourceFrontend
        .compile_document(&document(source))
        .unwrap_or_else(|error| panic!("{source:?}: {error}"))
}

fn turns(source: &str, expected: &[f64]) {
    turns_for_output(
        source,
        expected,
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

fn turns_for_output(source: &str, expected: &[f64], kind: mech_engine::SourceDocumentOutputKind) {
    compiled_turns(compiled(source), source, expected, kind);
}

fn compiled_turns(
    compiled: CanonicalSourceProgram,
    source: &str,
    expected: &[f64],
    kind: mech_engine::SourceDocumentOutputKind,
) {
    assert!(compiled.program().inputs.is_empty());
    for state in 0..compiled.program().states.len() as u32 {
        assert_eq!(
            compiled
                .program()
                .nodes
                .iter()
                .flat_map(|node| node.outputs.iter())
                .filter(|output| **output == SourceNodeOutput::State(state))
                .count(),
            1,
            "each state retains one writer"
        );
    }
    let artifact = compiled
        .compile_artifact()
        .expect("document must construct a canonical artifact");
    let bytecode = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytecode).unwrap();
    for artifact in [artifact, decoded] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x570, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| {
            panic!(
                "{source:?}: document activation: {error:?}; nodes: {:?}",
                compiled.source_map().nodes
            )
        });
        let output = compiled
            .document_outputs()
            .iter()
            .find(|binding| binding.kind == kind)
            .unwrap()
            .output as usize;
        for expected in expected {
            instance
                .turn(&[])
                .expect("state update must execute and publish");
            let output = instance.copied_output(output).unwrap();
            let ValueData::F64(actual) = output.data() else {
                panic!("expected a scalar f64 result: {output:?}")
            };
            assert_eq!(actual.to_f64(), *expected, "{source:?}");
        }
    }
}

fn matrix_shape(value: &Value) -> (usize, usize) {
    let schemas = value
        .schemas()
        .expect("matrix output retains its schema table");
    let SchemaBody::Matrix { dimensions, .. } = schemas
        .entry(value.schema())
        .expect("matrix output schema exists")
        .schema()
        .body()
    else {
        panic!("expected matrix output: {value:?}")
    };
    let [rows, columns] = dimensions.as_ref() else {
        panic!("matrix output has two dimensions")
    };
    (
        value.shape().resolve_dimension(rows).unwrap() as usize,
        value.shape().resolve_dimension(columns).unwrap() as usize,
    )
}

fn matrix_values(value: &Value) -> Vec<f64> {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("expected matrix output: {value:?}")
    };
    match matrix.elements() {
        SequenceView::F64(values) => values.iter().map(|value| value.to_f64()).collect(),
        SequenceView::Values(values) => values
            .iter()
            .map(|value| match value {
                ValueData::F64(value) => value.to_f64(),
                other => panic!("expected f64 matrix element: {other:?}"),
            })
            .collect(),
        other => panic!("expected f64 matrix storage: {other:?}"),
    }
}

fn f32_matrix_values(value: &Value) -> Vec<f32> {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("expected matrix output: {value:?}")
    };
    match matrix.elements() {
        SequenceView::F32(values) => values.iter().map(|value| value.to_f32()).collect(),
        SequenceView::Values(values) => values
            .iter()
            .map(|value| match value {
                ValueData::F32(value) => value.to_f32(),
                other => panic!("expected f32 matrix element: {other:?}"),
            })
            .collect(),
        other => panic!("expected f32 matrix storage: {other:?}"),
    }
}

fn bool_matrix_values(value: &Value) -> Vec<bool> {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("expected matrix output: {value:?}")
    };
    match matrix.elements() {
        SequenceView::Bool(values) => values.to_vec(),
        SequenceView::Values(values) => values
            .iter()
            .map(|value| match value {
                ValueData::Bool(value) => *value,
                other => panic!("expected bool matrix element: {other:?}"),
            })
            .collect(),
        other => panic!("expected bool matrix storage: {other:?}"),
    }
}

fn index_matrix_values(value: &Value) -> Vec<u64> {
    let ValueData::Matrix(matrix) = value.data() else {
        panic!("expected matrix output: {value:?}")
    };
    match matrix.elements() {
        SequenceView::Index(values) => values.to_vec(),
        SequenceView::Values(values) => values
            .iter()
            .map(|value| match value {
                ValueData::Index(value) => *value,
                other => panic!("expected index matrix element: {other:?}"),
            })
            .collect(),
        other => panic!("expected index matrix storage: {other:?}"),
    }
}

fn closed_matrix_turns(source: &str, assert_output: impl Fn(&Value)) {
    let compiled = compiled(source);
    assert!(compiled.program().inputs.is_empty(), "{source:?}");
    let output = compiled
        .document_outputs()
        .iter()
        .find(|binding| binding.kind == mech_engine::SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let artifact = compiled.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    for artifact in [artifact, decoded] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x570, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source:?}: matrix activation: {error:?}"));
        for _ in 0..2 {
            instance
                .turn(&[])
                .unwrap_or_else(|error| panic!("{source:?}: matrix turn: {error:?}"));
            assert_output(&instance.copied_output(output).unwrap());
        }
    }
}

fn variable_matrix_turns(source: &str, turns: &[(Option<[f64; 2]>, (usize, usize), &[f64])]) {
    let compiled = compiled(source);
    assert_eq!(
        compiled.program().inputs.len(),
        usize::from(turns.iter().any(|(input, _, _)| input.is_some())),
        "{source:?}"
    );
    let output = compiled
        .document_outputs()
        .iter()
        .find(|binding| binding.kind == mech_engine::SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let artifact = compiled.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    for artifact in [artifact, decoded] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x570, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source:?}: matrix activation: {error:?}"));
        for (input, expected_shape, expected_values) in turns {
            let inputs = input
                .as_ref()
                .map(|input| {
                    vec![CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::F64(input),
                    }]
                })
                .unwrap_or_default();
            instance
                .turn(&inputs)
                .unwrap_or_else(|error| panic!("{source:?}: matrix turn: {error:?}"));
            let actual = instance.copied_output(output).unwrap();
            assert_eq!(matrix_shape(&actual), *expected_shape, "{source:?}");
            assert_eq!(matrix_values(&actual), *expected_values, "{source:?}");
        }
    }
}

fn variable_bool_matrix_turns(source: &str, turns: &[(Option<[f64; 2]>, (usize, usize), &[bool])]) {
    let compiled = compiled(source);
    assert_eq!(
        compiled.program().inputs.len(),
        usize::from(turns.iter().any(|(input, _, _)| input.is_some())),
        "{source:?}"
    );
    let output = compiled
        .document_outputs()
        .iter()
        .find(|binding| binding.kind == mech_engine::SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let artifact = compiled.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    for artifact in [artifact, decoded] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x570, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source:?}: bool matrix activation: {error:?}"));
        for (input, expected_shape, expected_values) in turns {
            let inputs = input
                .as_ref()
                .map(|input| {
                    vec![CapturedSignalInput {
                        slot: instance.plan.inputs[0].slot,
                        value: ResidentValueRef::F64(input),
                    }]
                })
                .unwrap_or_default();
            instance
                .turn(&inputs)
                .unwrap_or_else(|error| panic!("{source:?}: bool matrix turn: {error:?}"));
            let actual = instance.copied_output(output).unwrap();
            assert_eq!(matrix_shape(&actual), *expected_shape, "{source:?}");
            assert_eq!(bool_matrix_values(&actual), *expected_values, "{source:?}");
        }
    }
}

#[test]
fn closed_comprehension_concatenation_retains_exact_shape() {
    variable_matrix_turns(
        "xs := [1 2]\ny := [x | x <- xs]\n[y y]\n",
        &[
            (None, (1, 4), &[1.0, 2.0, 1.0, 2.0]),
            (None, (1, 4), &[1.0, 2.0, 1.0, 2.0]),
        ],
    );
}

#[test]
fn closed_comprehension_initializer_shape_flows_through_snapshot_arithmetic_and_state() {
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x + 1 | x <- samples]\nnegated := -values\n~a := negated\na\n",
        &[
            (None, (1, 3), &[-2.0, -3.0, -4.0]),
            (None, (1, 3), &[-2.0, -3.0, -4.0]),
        ],
    );
}

#[test]
fn closed_comprehension_binary_arithmetic_keeps_its_live_shape() {
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\ndoubled := values + values\n~a := doubled\na\n",
        &[
            (None, (1, 3), &[2.0, 4.0, 6.0]),
            (None, (1, 3), &[2.0, 4.0, 6.0]),
        ],
    );
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\nshifted := values + 1\n~a := shifted\na\n",
        &[
            (None, (1, 3), &[2.0, 3.0, 4.0]),
            (None, (1, 3), &[2.0, 3.0, 4.0]),
        ],
    );
}

#[test]
fn runtime_shaped_arithmetic_accepts_dense_matrix_operands_in_both_orders() {
    for (source, expected_shape, expected_values) in [
        (
            "samples := 1..=3\nvalues := [x | x <- samples]\nvalues + [10 20 30]\n",
            (1, 3),
            &[11.0, 22.0, 33.0][..],
        ),
        (
            "samples := 1..=3\nvalues := [x | x <- samples]\n[10 20 30] + values\n",
            (1, 3),
            &[11.0, 22.0, 33.0],
        ),
        (
            "samples := 1..=3\nrow := [x | x <- samples]\nvalues := [row; row + 3]\nvalues + [10 20 30]\n",
            (2, 3),
            &[11.0, 22.0, 33.0, 14.0, 25.0, 36.0],
        ),
        (
            "samples := 1..=3\nrow := [x | x <- samples]\nvalues := [row; row + 3]\nvalues + [10; 20]\n",
            (2, 3),
            &[11.0, 12.0, 13.0, 24.0, 25.0, 26.0],
        ),
    ] {
        variable_matrix_turns(
            source,
            &[
                (None, expected_shape, expected_values),
                (None, expected_shape, expected_values),
            ],
        );
    }
}

#[test]
fn runtime_shaped_arithmetic_rejects_incompatible_dense_matrix_geometry() {
    let source = "samples := 1..=3\nvalues := [x | x <- samples]\nvalues + [10 20]\n";
    let program = compiled(source);
    let artifact = program.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        assert!(
            activate(
                ReactiveInstanceId::new(0x570, 0),
                &artifact,
                &catalog.build().unwrap(),
                &ActivationFacts::default(),
            )
            .is_err(),
            "{source:?}: incompatible live and dense axes must be rejected"
        );
    }
}

#[test]
fn closed_comprehension_kind_conversion_keeps_its_live_shape() {
    let source = "values := [x | x <- signal<[f64]:1,2>, x > 0]\nconverted<[u8]> := values\nshifted := converted + 1\nshifted\n";
    let compiled = compiled(source);
    assert!(compiled.program().nodes.iter().any(|node| {
        node.operation()
            .is_some_and(|operation| operation.canonical_name() == "convert/kind")
    }));
    variable_matrix_turns(
        source,
        &[
            (Some([1.0, 2.0]), (1, 2), &[2.0, 3.0]),
            (Some([-1.0, 2.0]), (1, 1), &[3.0]),
            (Some([-1.0, -2.0]), (1, 0), &[]),
        ],
    );
}

#[test]
fn closed_comprehension_binary_math_keeps_its_live_shape() {
    let quarter_turn = std::f64::consts::FRAC_PI_4;
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\nangles := math/atan2(values, values)\n~a := angles\na\n",
        &[
            (None, (1, 3), &[quarter_turn, quarter_turn, quarter_turn]),
            (None, (1, 3), &[quarter_turn, quarter_turn, quarter_turn]),
        ],
    );
}

#[test]
fn runtime_shaped_binary_math_accepts_dense_matrix_operands_in_both_orders() {
    for (source, expected) in [
        (
            "samples := 1..=3\nvalues := [x | x <- samples]\nmath/atan2(values, [1 1 1])\n",
            [1.0_f64.atan2(1.0), 2.0_f64.atan2(1.0), 3.0_f64.atan2(1.0)],
        ),
        (
            "samples := 1..=3\nvalues := [x | x <- samples]\nmath/atan2([1 1 1], values)\n",
            [1.0_f64.atan2(1.0), 1.0_f64.atan2(2.0), 1.0_f64.atan2(3.0)],
        ),
    ] {
        variable_matrix_turns(
            source,
            &[(None, (1, 3), &expected), (None, (1, 3), &expected)],
        );
    }
}

#[test]
fn runtime_shaped_comparisons_accept_dense_matrix_operands_in_both_orders() {
    for (source, expected) in [
        (
            "samples := 1..=3\nrow := [x | x <- samples]\nvalues := [row; row + 3]\nvalues == [1 0 3; 4 5 0]\n",
            &[true, false, true, true, true, false][..],
        ),
        (
            "samples := 1..=3\nrow := [x | x <- samples]\nvalues := [row; row + 3]\nvalues < [2 2 4]\n",
            &[true, false, true, false, false, false],
        ),
        (
            "samples := 1..=3\nrow := [x | x <- samples]\nvalues := [row; row + 3]\n[2; 5] < values\n",
            &[false, false, true, false, false, true],
        ),
    ] {
        variable_bool_matrix_turns(
            source,
            &[(None, (2, 3), expected), (None, (2, 3), expected)],
        );
    }
}

#[test]
fn closed_comprehension_unary_comparison_and_logic_keep_live_shapes() {
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [sample * sample | sample <- samples]\nrooted := math/sqrt(values)\n~a := rooted\na\n",
        &[
            (None, (1, 3), &[1.0, 2.0, 3.0]),
            (None, (1, 3), &[1.0, 2.0, 3.0]),
        ],
    );
    variable_bool_matrix_turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\nless := values < values\nequal := values == values\ncombined := !less && equal\n~a := combined\na\n",
        &[
            (None, (1, 3), &[true, true, true]),
            (None, (1, 3), &[true, true, true]),
        ],
    );
    variable_bool_matrix_turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\ngreater := values > 1\ncombined := greater && true\n~a := combined\na\n",
        &[
            (None, (1, 3), &[false, true, true]),
            (None, (1, 3), &[false, true, true]),
        ],
    );
    variable_bool_matrix_turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\nmatrix := [values; values]\ngreater := matrix > 2\n~a := greater\na\n",
        &[
            (None, (2, 3), &[false, false, true, false, false, true]),
            (None, (2, 3), &[false, false, true, false, false, true]),
        ],
    );
}

#[test]
fn closed_comprehension_reductions_resolve_live_axes() {
    for (reduction, expected_shape, expected_values) in [
        ("stats/sum/column", (2, 1), &[6.0, 6.0][..]),
        ("stats/sum/row", (1, 3), &[2.0, 4.0, 6.0][..]),
    ] {
        let source = format!(
            "+> stats\nsamples := 1..=3\nvalues := [sample | sample <- samples]\nmatrix := [values; values]\nreduced := {reduction}(matrix)\n~a := reduced\na\n"
        );
        variable_matrix_turns(
            &source,
            &[
                (None, expected_shape, expected_values),
                (None, expected_shape, expected_values),
            ],
        );
    }
}

#[test]
fn closed_comprehension_matrix_dot_publishes_a_dense_scalar() {
    turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\nresult := matrix/dot(values, values)\nresult\n",
        &[14.0, 14.0],
    );
}

#[test]
fn runtime_shaped_matrix_dot_accepts_fixed_dense_operands_in_both_orders() {
    turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\nresult := matrix/dot(values, [1 2 3])\nresult\n",
        &[14.0, 14.0],
    );
    turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\nresult := matrix/dot([1 2 3], values)\nresult\n",
        &[14.0, 14.0],
    );
}

#[test]
fn all_elements_access_flattens_the_live_snapshot_shape() {
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\nflattened := values[:]\nflattened\n",
        &[
            (None, (3, 1), &[1.0, 2.0, 3.0]),
            (None, (3, 1), &[1.0, 2.0, 3.0]),
        ],
    );
}

#[test]
fn runtime_shaped_n_choose_k_accepts_a_dense_selection() {
    variable_matrix_turns(
        "samples := 1..=4\nvalues := [x | x <- samples]\ncombinations := combinatorics/n-choose-k(values, 2)\ncombinations\n",
        &[
            (
                None,
                (2, 6),
                &[1.0, 1.0, 1.0, 2.0, 2.0, 3.0, 2.0, 3.0, 4.0, 3.0, 4.0, 4.0],
            ),
            (
                None,
                (2, 6),
                &[1.0, 1.0, 1.0, 2.0, 2.0, 3.0, 2.0, 3.0, 4.0, 3.0, 4.0, 4.0],
            ),
        ],
    );
}

#[test]
fn runtime_shaped_n_choose_k_accepts_an_activation_derived_selection() {
    variable_matrix_turns(
        "selection := (true ? | true => 2 | false => 1)\ncombinations := combinatorics/n-choose-k([1 2 3 4], selection)\ncombinations\n",
        &[
            (
                None,
                (2, 6),
                &[1.0, 1.0, 1.0, 2.0, 2.0, 3.0, 2.0, 3.0, 4.0, 3.0, 4.0, 4.0],
            ),
            (
                None,
                (2, 6),
                &[1.0, 1.0, 1.0, 2.0, 2.0, 3.0, 2.0, 3.0, 4.0, 3.0, 4.0, 4.0],
            ),
        ],
    );
}

#[test]
fn activation_derived_selectors_gather_from_fixed_dense_matrices() {
    variable_matrix_turns(
        "samples := 1..=3\nselectors := [x | x <- samples, x != 2]\nvalues := [10 20 30]\nselected := values[selectors]\nselected\n",
        &[(None, (2, 1), &[10.0, 30.0]), (None, (2, 1), &[10.0, 30.0])],
    );
}

#[test]
fn closed_comprehension_matmul_resolves_live_product_dimensions() {
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\nproduct := matrix/matmul(values', values)\n~state := product\nstate\n",
        &[
            (None, (3, 3), &[1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 3.0, 6.0, 9.0]),
            (None, (3, 3), &[1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 3.0, 6.0, 9.0]),
        ],
    );
}

#[test]
fn closed_comprehension_matmul_publishes_a_dense_scalar() {
    closed_matrix_turns(
        "samples := 1..=3\nvalues := [sample | sample <- samples]\nproduct := matrix/matmul(values, values')\n~state := product\nstate\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (1, 1));
            assert_eq!(matrix_values(actual), [14.0]);
        },
    );
}

#[test]
fn runtime_shaped_matmul_accepts_asymmetric_dense_matrix_operands_in_both_orders() {
    variable_matrix_turns(
        "samples := 1..=3\nrow := [x | x <- samples]\nvalues := [row; row + 3]\nproduct := matrix/matmul(values, [7 8; 9 10; 11 12])\nproduct\n",
        &[
            (None, (2, 2), &[58.0, 64.0, 139.0, 154.0]),
            (None, (2, 2), &[58.0, 64.0, 139.0, 154.0]),
        ],
    );
    variable_matrix_turns(
        "samples := 1..=3\nrow := [x | x <- samples]\nvalues := [row; row + 3]\nproduct := matrix/matmul([7 8; 9 10], values)\nproduct\n",
        &[
            (None, (2, 3), &[39.0, 54.0, 69.0, 49.0, 68.0, 87.0]),
            (None, (2, 3), &[39.0, 54.0, 69.0, 49.0, 68.0, 87.0]),
        ],
    );
}

#[test]
fn runtime_shaped_f64_matrices_reach_the_existing_solve_semantics() {
    variable_matrix_turns(
        "solution := [1 2; 1 4] \\ [5; 9]\nsolution\n",
        &[(None, (2, 1), &[1.0, 2.0]), (None, (2, 1), &[1.0, 2.0])],
    );
    variable_matrix_turns(
        "first := [x | x <- [1 2]]\nsecond := [x | x <- [1 4]]\ncoefficients := [first; second]\nright-row := [x | x <- [5 9]]\nright := right-row'\nsolution := coefficients \\ right\n~state := solution\nstate\n",
        &[(None, (2, 1), &[1.0, 2.0]), (None, (2, 1), &[1.0, 2.0])],
    );
}

#[test]
fn runtime_shaped_matrix_solve_accepts_fixed_dense_operands_in_both_positions() {
    variable_matrix_turns(
        "first := [x | x <- [1 2]]\nsecond := [x | x <- [1 4]]\ncoefficients := [first; second]\nsolution := coefficients \\ [5; 9]\nsolution\n",
        &[(None, (2, 1), &[1.0, 2.0]), (None, (2, 1), &[1.0, 2.0])],
    );
    variable_matrix_turns(
        "right-row := [x | x <- [5 9]]\nright := right-row'\nsolution := [1 2; 1 4] \\ right\nsolution\n",
        &[(None, (2, 1), &[1.0, 2.0]), (None, (2, 1), &[1.0, 2.0])],
    );
}

#[test]
fn runtime_shaped_state_supports_whole_value_updates_after_initialization() {
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\n~state := values\nstate += state\nstate\n",
        &[
            (None, (1, 3), &[2.0, 4.0, 6.0]),
            (None, (1, 3), &[4.0, 8.0, 12.0]),
        ],
    );
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\n~state := values\nnext := state + 1\nstate = next\nstate\n",
        &[
            (None, (1, 3), &[2.0, 3.0, 4.0]),
            (None, (1, 3), &[3.0, 4.0, 5.0]),
        ],
    );
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\n~state := values\nstate += 1\nstate\n",
        &[
            (None, (1, 3), &[2.0, 3.0, 4.0]),
            (None, (1, 3), &[3.0, 4.0, 5.0]),
        ],
    );
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\n~state := values\nstate += [10 20 30]\nstate\n",
        &[
            (None, (1, 3), &[11.0, 22.0, 33.0]),
            (None, (1, 3), &[21.0, 42.0, 63.0]),
        ],
    );
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\n~state := values\nstate = [10 20 30]\nstate\n",
        &[
            (None, (1, 3), &[10.0, 20.0, 30.0]),
            (None, (1, 3), &[10.0, 20.0, 30.0]),
        ],
    );
    variable_matrix_turns(
        "samples := 1..=3\nvalues := [x | x <- samples]\n~state := values\nstate[2] = 10\nstate\n",
        &[
            (None, (1, 3), &[1.0, 10.0, 3.0]),
            (None, (1, 3), &[1.0, 10.0, 3.0]),
        ],
    );
    variable_matrix_turns(
        "samples := 1..=2\nrow := [x | x <- samples]\nvalues := [row; row + 2]\n~state := values\nstate[2,:] = [10 20]\nstate\n",
        &[
            (None, (2, 2), &[1.0, 2.0, 10.0, 20.0]),
            (None, (2, 2), &[1.0, 2.0, 10.0, 20.0]),
        ],
    );
}

#[test]
fn runtime_shaped_state_resolves_snapshot_rhs_axes_for_indexed_assignment() {
    closed_matrix_turns(
        "samples := 1..=2\nraw-values := [x | x <- samples]\nvalues<[f32]> := raw-values\nreplacement-raw := [x | x <- [9]]\nreplacement-values<[f32]> := replacement-raw\nreplacement := replacement-values[1]\n~state := values\nstate[2] = replacement\nstate\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (1, 2));
            assert_eq!(f32_matrix_values(actual), [1.0, 9.0]);
        },
    );
    let turn = std::cell::Cell::new(0);
    closed_matrix_turns(
        "samples := 1..=2\nrow := [x | x <- samples]\nvalues<[f32]> := [row; row + 2]\n~state := values\nstate[2,:] += [10 20]\nstate\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (2, 2));
            let expected = if turn.get() % 2 == 0 {
                [1.0, 2.0, 13.0, 24.0]
            } else {
                [1.0, 2.0, 23.0, 44.0]
            };
            assert_eq!(f32_matrix_values(actual), expected);
            turn.set(turn.get() + 1);
        },
    );
}

#[test]
fn runtime_shaped_state_assigns_all_dense_primitive_rhs_kinds() {
    closed_matrix_turns(
        "samples := [true false]\nvalues := [x | x <- samples]\n~state := values\nstate = [false true]\nstate\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (1, 2));
            assert_eq!(bool_matrix_values(actual), [false, true]);
        },
    );
    closed_matrix_turns(
        "samples := [1<index> 2<index>]\nvalues := [x | x <- samples]\n~state := values\nstate[2] = 7<index>\nstate\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (1, 2));
            assert_eq!(index_matrix_values(actual), [1, 7]);
        },
    );
    closed_matrix_turns(
        "values := (true ? | true => [\"a\" \"b\"] | false => [\"c\" \"d\"])\n~state := values\nstate[2] = \"longer replacement\"\nstate\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (1, 2));
            let ValueData::Matrix(matrix) = actual.data() else {
                panic!("expected String matrix: {actual:?}")
            };
            let SequenceView::String(values) = matrix.elements() else {
                panic!("expected packed String matrix: {matrix:?}")
            };
            assert_eq!(
                values.iter().map(AsRef::as_ref).collect::<Vec<&str>>(),
                ["a", "longer replacement"]
            );
        },
    );
}

#[test]
fn runtime_shaped_matrices_use_semantic_strict_equality_with_dense_operands() {
    for (source, expected) in [
        (
            "samples := 1..=3\nvalues := [x | x <- samples]\nvalues === [1 2 3]\n",
            true,
        ),
        (
            "samples := 1..=3\nvalues := [x | x <- samples]\n[1 2 3] !== values\n",
            false,
        ),
        (
            "samples := 1..=2\nvalues := [x | x <- samples]\nvalues === [1 2 3]\n",
            false,
        ),
        (
            "values := [x | x <- [true false]]\nvalues === [true false]\n",
            true,
        ),
        (
            "values := [x | x <- [1<index> 2<index>]]\n[1<index> 2<index>] === values\n",
            true,
        ),
        (
            "values := (true ? | true => [\"a\" \"b\"] | false => [\"c\" \"d\"])\nvalues === [\"a\" \"b\"]\n",
            true,
        ),
    ] {
        let compiled = compiled(source);
        let output = compiled
            .document_outputs()
            .iter()
            .find(|binding| binding.kind == mech_engine::SourceDocumentOutputKind::Program)
            .unwrap()
            .output as usize;
        let artifact = compiled.compile_artifact().unwrap();
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        for artifact in [
            artifact,
            mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap(),
        ] {
            let mut catalog = FunctionCatalogBuilder::new();
            mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x570, 0),
                &artifact,
                &catalog.build().unwrap(),
                &ActivationFacts::default(),
            )
            .unwrap_or_else(|error| panic!("{source:?}: strict activation: {error:?}"));
            for _ in 0..2 {
                instance
                    .turn(&[])
                    .unwrap_or_else(|error| panic!("{source:?}: strict turn: {error:?}"));
                let actual = instance.copied_output(output).unwrap();
                let ValueData::Bool(actual) = actual.data() else {
                    panic!("{source:?}: expected Bool output: {actual:?}")
                };
                assert_eq!(*actual, expected, "{source:?}");
            }
        }
    }
}

#[test]
fn closed_comprehension_positional_selector_keeps_live_cardinality() {
    let source = "samples := 1..=3\nselectors := [sample | sample <- samples, sample != 2]\nvalues := [sample * 10 | sample <- samples]\nselected := values[selectors]\n~a := selected\na\n";
    variable_matrix_turns(
        source,
        &[(None, (2, 1), &[10.0, 30.0]), (None, (2, 1), &[10.0, 30.0])],
    );
}

#[test]
fn changing_comprehension_concatenation_publishes_each_shape() {
    variable_matrix_turns(
        "y := [x | x <- signal<[f64]:1,2>, x > 0]\n[y y]\n",
        &[
            (Some([1.0, 2.0]), (1, 4), &[1.0, 2.0, 1.0, 2.0]),
            (Some([-1.0, 2.0]), (1, 2), &[2.0, 2.0]),
            (Some([-1.0, -2.0]), (1, 0), &[]),
        ],
    );
}

#[test]
fn changing_comprehension_transpose_publishes_each_shape() {
    variable_matrix_turns(
        "y := [x | x <- signal<[f64]:1,2>, x > 0]\ny'\n",
        &[
            (Some([1.0, 2.0]), (2, 1), &[1.0, 2.0]),
            (Some([-1.0, 2.0]), (1, 1), &[2.0]),
            (Some([-1.0, -2.0]), (0, 1), &[]),
        ],
    );
}

#[test]
fn empty_comprehension_publishes_its_nullary_matrix() {
    variable_matrix_turns(
        "samples := 1..=3\nempty := [sample | sample <- samples, sample > 4]\nempty\n",
        &[(None, (1, 0), &[]), (None, (1, 0), &[])],
    );
}

#[test]
fn changing_comprehension_concatenates_with_fixed_dense_operands() {
    variable_matrix_turns(
        "y := [x | x <- signal<[f64]:1,2>, x > 0]\n[y 3 [4 5]]\n",
        &[
            (Some([1.0, 2.0]), (1, 5), &[1.0, 2.0, 3.0, 4.0, 5.0]),
            (Some([-1.0, 2.0]), (1, 4), &[2.0, 3.0, 4.0, 5.0]),
            (Some([-1.0, -2.0]), (1, 3), &[3.0, 4.0, 5.0]),
        ],
    );
}

#[test]
fn changing_boolean_comprehension_transpose_publishes_each_shape() {
    variable_bool_matrix_turns(
        "y := [x > 1 | x <- signal<[f64]:1,2>, x > 0]\ny'\n",
        &[
            (Some([1.0, 2.0]), (2, 1), &[false, true]),
            (Some([-1.0, 2.0]), (1, 1), &[true]),
            (Some([-1.0, -2.0]), (0, 1), &[]),
        ],
    );
}

#[test]
fn index_comprehension_transpose_preserves_snapshot_elements() {
    closed_matrix_turns(
        "xs := [1<index> 2<index>]\ny := [x | x <- xs]\ny'\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (2, 1));
            assert_eq!(index_matrix_values(actual), [1, 2]);
        },
    );
}

#[test]
fn index_snapshot_comparisons_preserve_exact_values_and_broadcasting() {
    for (comparison, expected) in [
        ("values == 1<index>", &[true, false][..]),
        ("values != 1<index>", &[false, true]),
        ("values < 2<index>", &[true, false]),
        ("values > 1<index>", &[false, true]),
        ("values <= other", &[true, true]),
        ("values >= other", &[true, true]),
    ] {
        let source = format!(
            "xs := [1<index> 2<index>]\nvalues := [x | x <- xs]\nother := [x | x <- xs]\n{comparison}\n"
        );
        variable_bool_matrix_turns(
            &source,
            &[(None, (1, 2), expected), (None, (1, 2), expected)],
        );
    }
}

#[test]
fn interactive_fixture_executes_and_retains_state_across_turns() {
    let source = include_str!("../../../tests/fixtures/syntax-source-boundary/interactive.mec");
    assert_eq!(source, "~answer := 0\nanswer += 1\nanswer\n");
    turns(source, &[1.0, 2.0]);
}

#[test]
fn indexed_document_updates_use_canonical_assignment_and_preserve_state_order() {
    for (source, expected) in [
        (
            "~numbers := [1, 2, 3, 4, 5]\nnumbers[2] = 10\nnumbers[2]\n",
            [10.0, 10.0],
        ),
        (
            "~numbers := [1, 2, 3]\nnumbers[2] += 1\nnumbers[2]\n",
            [3.0, 4.0],
        ),
        (
            "~numbers := [1, 2, 3]\nnumbers[2] = 10\nbefore := numbers[2]\nnumbers[2] += 2\nbefore + numbers[2]\n",
            [22.0, 22.0],
        ),
        (
            "~numbers := [1, 2, 3]\nnumbers[1..=2] = 10\nnumbers[1] + numbers[2] + numbers[3]\n",
            [23.0, 23.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:,:] = [5, 6; 7, 8]\nnumbers[2,2]\n",
            [8.0, 8.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:,:] = 9\nnumbers[1,1] + numbers[2,2]\n",
            [18.0, 18.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:] = [5, 6; 7, 8]\nnumbers[2,2]\n",
            [8.0, 8.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:] = 9\nnumbers[1,1] + numbers[2,2]\n",
            [18.0, 18.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[2,1] = 10\nnumbers[2,1]\n",
            [10.0, 10.0],
        ),
        (
            "~record := {value: 1}\nrecord.value += 1\nrecord.value\n",
            [2.0, 3.0],
        ),
        (
            "~record := {values: [1, 2]}\nrecord.values[2] += 1\nrecord.values[2]\n",
            [3.0, 4.0],
        ),
        (
            "~numbers := [1, 2]\nnumbers[1] += 1\nnumbers[2] = 2\nnumbers[1] + numbers[2]\n",
            [4.0, 5.0],
        ),
        ("~pair := (1, 2)\npair.2 += 1\npair.2\n", [3.0, 4.0]),
        (
            "~record := {a: 1, b: 2}\nrecord.b,a = (3, 4)\nrecord.a + record.b\n",
            [7.0, 7.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[2,:] = 10\nnumbers[2,1] + numbers[2,2]\n",
            [20.0, 20.0],
        ),
        (
            "~numbers := [1, 2; 3, 4]\nnumbers[:,2] += 1\nnumbers[1,2] + numbers[2,2]\n",
            [8.0, 10.0],
        ),
    ] {
        turns(source, &expected);
    }
}

#[test]
fn whole_value_document_assignment_does_not_emit_a_discarded_selection() {
    for selection in [":", ":,:"] {
        let source =
            format!("~matrix := [1, 2; 3, 4]\nmatrix[{selection}] = [5, 6; 7, 8]\nmatrix\n");
        let compiled = compiled(&source);
        let operations = compiled
            .program()
            .nodes
            .iter()
            .map(|node| {
                node.operation()
                    .expect("ordinary source operation")
                    .canonical_name()
            })
            .collect::<Vec<_>>();
        assert!(
            operations
                .iter()
                .any(|operation| operation == "core/assign/whole-value"),
            "{source}"
        );
        assert!(
            !operations
                .iter()
                .any(|operation| operation == "access/range"),
            "{source}: {operations:?}"
        );
        compiled.compile_artifact().unwrap();
    }
}

#[test]
fn document_bindings_preserve_empty_errors_and_contextual_optional_state() {
    for source in [
        "x := _\nx\n",
        "~x := _\nx\n",
        "~x := 1\nx = _\nx\n",
        "_\n1\n",
        "```mech\n_\n1\n```\n",
    ] {
        let error = CanonicalSourceFrontend
            .compile_document(&document(source))
            .err()
            .expect("an unresolved binding must not become an executable value");
        assert_eq!(error.code, "source-semantics/unresolved-empty-expression");
        assert_eq!(error.anchor.document, DocumentId(0x570));
        assert_eq!(error.anchor.revision, Revision(7));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            "_",
        );
    }
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for source in ["~x<u8?> := _\nx\n", "~x<u8?> := 1u8\nx = _\nx\n"] {
        let program = compiled(source);
        assert!(program.program().inputs.is_empty());
        let artifact = program.compile_artifact().unwrap();
        let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x578, 0),
            &decoded,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            assert!(
                matches!(
                    instance.copied_output(0).unwrap().data(),
                    ValueData::Option(None)
                ),
                "{source}"
            );
        }
    }
}

#[test]
fn discarded_candidate_does_not_advance_document_state() {
    let compiled = compiled("~answer := 0\nanswer += 1\nanswer\n");
    let artifact = compiled.compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x571, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    {
        let candidate = instance.prepare_turn(&[]).unwrap();
        let output = candidate.copied_output(0).unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!("scalar candidate")
        };
        assert_eq!(value.to_f64(), 1.0);
    }
    for expected in [1.0, 2.0] {
        instance.turn(&[]).unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!("scalar published output")
        };
        assert_eq!(value.to_f64(), expected);
    }
}

#[test]
fn indexed_candidates_abort_without_mutating_published_state() {
    for failing in [false, true] {
        let source = if failing {
            "~numbers := [1, 2]\nnumbers[2] += 1\nnumbers[0] = 9\nnumbers[2]\n"
        } else {
            "~numbers := [1, 2]\nnumbers[2] += 1\nnumbers[2]\n"
        };
        let compiled = compiled(source);
        let bytes =
            mech_engine::encode_program_artifact_bytecode_v1(&compiled.compile_artifact().unwrap())
                .unwrap();
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x572, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let before = instance.published_state_hash();
        if failing {
            assert!(instance.turn(&[]).is_err());
            assert_eq!(instance.published_state_hash(), before);
            assert!(instance.turn_without_summary(&[]).is_err());
            assert_eq!(instance.published_state_hash(), before);
        } else {
            {
                let candidate = instance.prepare_turn(&[]).unwrap();
                let output = candidate.copied_output(0).unwrap();
                let ValueData::F64(value) = output.data() else {
                    panic!("scalar candidate")
                };
                assert_eq!(value.to_f64(), 3.0);
            }
            assert_eq!(instance.published_state_hash(), before);
            instance.turn_without_summary(&[]).unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::F64(value) = output.data() else {
                panic!("scalar output")
            };
            assert_eq!(value.to_f64(), 3.0);
        }
    }
}

#[test]
fn sequential_updates_read_the_preceding_candidate_and_keep_one_writer() {
    turns(
        "~answer := 0\nanswer += 1\nanswer *= 2\nanswer\n",
        &[2.0, 6.0],
    );
    turns(
        "~answer := 1\nanswer += 1\nanswer += answer\nanswer\n",
        &[4.0, 10.0],
    );
    turns(
        "~answer := 0\nanswer = 3\nanswer += 1\nanswer\n",
        &[4.0, 4.0],
    );
    turns(
        "~left := 0\n~right := 1\nleft += right\nright += left\nright\n",
        &[2.0, 5.0],
    );
}

#[test]
fn immutable_reads_retain_their_source_order_value() {
    turns(
        "~answer := 0\nbefore := answer\nanswer += 1\nbefore\n",
        &[0.0, 1.0],
    );
    turns(
        "~answer := 0\nanswer += 1\nafter := answer\nanswer += 1\nafter\n",
        &[1.0, 3.0],
    );
}

#[test]
fn each_assignment_operator_uses_maintained_arithmetic() {
    for (source, expected) in [
        ("~answer := 8\nanswer -= 2\nanswer\n", [6.0, 4.0]),
        ("~answer := 8\nanswer /= 2\nanswer\n", [4.0, 2.0]),
        ("~answer := 2\nanswer ^= 2\nanswer\n", [4.0, 16.0]),
    ] {
        turns(source, &expected);
    }
}

#[test]
fn assignment_errors_are_anchored_to_the_target_or_value() {
    for (source, code, anchor_text) in [
        (
            "answer += 1\n",
            "source-semantics/unknown-assignment-target",
            "answer",
        ),
        (
            "answer := 0\nanswer += 1\n",
            "source-semantics/immutable-assignment-target",
            "answer",
        ),
        (
            "~answer := 0\ncopy := answer\ncopy += 1\n",
            "source-semantics/immutable-assignment-target",
            "copy",
        ),
        (
            "~answer := 0\nanswer = true\n",
            "source-semantics/incompatible-assignment-kind",
            "true",
        ),
        (
            "~answer := 0\nanswer[1] += 1\n",
            "source-semantics/unsupported-assignment-target",
            "[1]",
        ),
    ] {
        let error = CanonicalSourceFrontend
            .compile_document(&document(source))
            .err()
            .expect("invalid assignment must fail");
        assert_eq!(error.code, code, "{source:?}");
        assert_eq!(error.anchor.document, DocumentId(0x570));
        assert_eq!(error.anchor.revision, Revision(7));
        assert_eq!(
            &source[error.anchor.range.start.0 as usize..error.anchor.range.end.0 as usize],
            anchor_text
        );
    }
}

#[test]
fn document_display_and_child_scopes_do_not_execute_updates() {
    turns(
        "~answer := 0\n\nDisplayed {{answer += 100}}.\n\nanswer += 1\nanswer\n",
        &[1.0, 2.0],
    );
    turns(
        "~answer := 0\n~∘~⸢answer += 100\n⸥\nanswer += 1\nanswer\n",
        &[1.0, 2.0],
    );
    turns_for_output(
        "~answer := 0\nanswer += 1\n\nEvaluated {answer + 10}.\n",
        &[11.0, 12.0],
        mech_engine::SourceDocumentOutputKind::Inline,
    );
}

#[test]
fn comprehension_reads_the_source_order_state_candidate_after_writer_reordering() {
    let compiled = compiled("~answer := 0\nanswer += 1\n[answer + x | x <- [1 2]]\n");
    let bytes =
        mech_engine::encode_program_artifact_bytecode_v1(&compiled.compile_artifact().unwrap())
            .unwrap();
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x571, 0),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let output = compiled
        .document_outputs()
        .iter()
        .find(|binding| binding.kind == mech_engine::SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    for expected in [[2.0, 3.0], [3.0, 4.0]] {
        instance.turn(&[]).unwrap();
        assert_eq!(
            instance
                .copied_output(output)
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            mech_core::ValueDataDraft::Matrix(
                expected
                    .into_iter()
                    .map(|value| mech_core::ValueDataDraft::F64(
                        mech_core::snapshot::F64Bits::from_f64(value)
                    ))
                    .collect()
            )
        );
    }
}

#[test]
fn named_document_scopes_execute_separately_and_share_repeated_fences() {
    let source = "~counter := 100\ncounter += 10\ncounter\n\n```mech:left\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:right\n~counter := 10\ncounter += 5\ncounter\n```\n\n```mech:left\ncounter += 2\ncounter\n```\n";
    turns(source, &[110.0, 120.0]);
    for (name, expected, fences) in [("left", [3.0, 6.0], 2), ("right", [15.0, 20.0], 1)] {
        let program = CanonicalSourceFrontend
            .compile_named_document_scope(&document(source), name)
            .unwrap();
        assert_eq!(program.document_outputs().len(), fences + 1);
        assert_eq!(program.program().states.len(), 1);
        compiled_turns(
            program,
            source,
            &expected,
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

#[test]
fn absent_named_scopes_never_fall_back_to_the_root_program() {
    let source = "answer := 42\n\n```mech:disabled\nanswer := 99\n```\n";
    for name in ["missing", "disabled", "hidden", ""] {
        let error = CanonicalSourceFrontend
            .compile_named_document_scope(&document(source), name)
            .err()
            .expect("missing scope has no executable units");
        assert_eq!(error.code, "source-semantics/empty-document");
    }
}

#[test]
fn derived_updates_compare_final_output_to_the_value_before_seeding() {
    use mech_core::{
        BoundResidentKernel, ChangeDetectionPolicy, OutputConstruction, ResidentKernelError,
        ResidentKernelInputs, ResidentValueMut,
    };
    use mech_engine::resident::ActivatedTurnStep;

    fn leave_seed_unchanged(
        _: &BoundResidentKernel,
        _: &dyn ResidentKernelInputs,
        _: ResidentValueMut<'_>,
    ) -> Result<bool, ResidentKernelError> {
        Ok(false)
    }

    for (exact_scalar, source) in [
        (
            false,
            "~tick := 0\ntick += 1\n~values := [0]\nvalues[1] = tick\nvalues[1] = 0\nvalues[1]\n",
        ),
        (
            false,
            "~tick := 0\ntick += 1\n~values := {value: 0}\nvalues.value = tick\nvalues.value = 0\nvalues.value\n",
        ),
        (
            true,
            "~values := [0]\nvalues[1] += 1\nvalues[1] += 1\nvalues[1]\n",
        ),
    ] {
        let artifact = compiled(source).compile_artifact().unwrap();
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instances = (0..2)
            .map(|id| {
                activate(
                    ReactiveInstanceId::new(0x580, id),
                    &artifact,
                    &catalog,
                    &ActivationFacts::default(),
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let mut summaries = Vec::new();
        for (forced, instance) in instances.iter_mut().enumerate() {
            instance.turn(&[]).unwrap();
            let update = instance
                .plan
                .steps
                .iter()
                .enumerate()
                .filter_map(|(index, step)| match step {
                    ActivatedTurnStep::Kernel(node)
                        if artifact.slots()[node.write.slot.get() as usize].role
                            == mech_engine::SlotRole::Derived
                            && matches!(
                                node.construction,
                                OutputConstruction::ReadModifyWrite { .. }
                            ) =>
                    {
                        Some(index)
                    }
                    _ => None,
                })
                .last()
                .unwrap();
            if exact_scalar {
                instance.plan.replace_kernel_for_test(
                    update,
                    BoundResidentKernel::new(leave_seed_unchanged, Box::new([])),
                );
            }
            instance.plan.set_change_detection_for_test(
                update,
                if forced == 1 {
                    ChangeDetectionPolicy::AlwaysChanged
                } else if exact_scalar {
                    ChangeDetectionPolicy::ExactScalar
                } else {
                    ChangeDetectionPolicy::KernelReported
                },
            );
            summaries.push(instance.turn(&[]).unwrap());
            let output = instance.copied_output(0).unwrap();
            let ValueData::F64(value) = output.data() else {
                panic!("scalar result")
            };
            assert_eq!(value.to_f64(), if exact_scalar { 3.0 } else { 0.0 });
        }
        if !exact_scalar {
            assert!(
                summaries[0].dirty_nodes < summaries[1].dirty_nodes,
                "an unchanged final update must not dirty its consumers"
            );
        }
    }
}

#[test]
fn sequential_derived_updates_share_only_the_largest_rmw_backup_region() {
    use mech_core::{AllocationRole, MemoryLifetime, MemoryObjectOwner};

    let source = "~tick := 0\ntick += 1\n~small := [0, 0]\nsmall[1] = tick\nsmall[1] = 0\n~large := [0, 0, 0, 0]\nlarge[1] = tick\nlarge[1] = 0\nsmall[1] + large[1]\n";
    let artifact = compiled(&source).compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x582, 0),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let backups = instance
        .plan
        .memory_plan
        .allocations
        .iter()
        .filter(|allocation| {
            allocation.role == AllocationRole::Scratch
                && matches!(allocation.owner, MemoryObjectOwner::TransactionStage { .. })
                && matches!(allocation.lifetime, MemoryLifetime::Turn { .. })
        })
        .collect::<Vec<_>>();
    assert!(backups.len() >= 4, "{:#?}", instance.plan.memory_plan);
    let group = backups[0].reuse_group.unwrap();
    let arena_id = backups[0].placement.arena;
    assert!(backups.iter().all(|backup| {
        backup.reuse_group == Some(group)
            && backup.placement.arena == arena_id
            && backup.placement.offset == 0
    }));
    let maximum = backups
        .iter()
        .map(|backup| backup.capacity_bytes)
        .max()
        .unwrap();
    let sum = backups
        .iter()
        .map(|backup| backup.capacity_bytes)
        .sum::<u64>();
    assert!(sum > maximum);
    let arena = instance
        .plan
        .memory_plan
        .arenas
        .iter()
        .find(|arena| arena.id == arena_id)
        .unwrap();
    assert_eq!(arena.capacity_bytes, maximum);
    instance.turn(&[]).unwrap();
    let output = instance.copied_output(0).unwrap();
    let ValueData::F64(value) = output.data() else {
        panic!("scalar result")
    };
    assert_eq!(value.to_f64(), 0.0);
}

#[test]
fn derived_snapshot_updates_release_budgeted_prior_outputs_on_abort_and_drop() {
    use mech_engine::resident::{ResidentActivationOptions, activate_with_options};
    let artifact =
        compiled("~record := {values: [1, 2]}\nrecord.values[2] += 1\nrecord.values[2]\n")
            .compile_artifact()
            .unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let budget = mech_core::ManagedMemoryBudget::new(1 << 24);
    let mut instance = activate_with_options(
        ReactiveInstanceId::new(0x581, 0),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentActivationOptions {
            memory_budget: Some(budget.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    let before = instance.published_state_hash();
    for _ in 0..3 {
        drop(instance.prepare_turn(&[]).unwrap());
        assert_eq!(instance.published_state_hash(), before);
    }
    for expected in [3.0, 4.0, 5.0] {
        instance.turn_without_summary(&[]).unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!("scalar result")
        };
        assert_eq!(value.to_f64(), expected);
    }
    drop(instance);
    assert_eq!(budget.used_bytes(), 0);
}

#[test]
fn configured_fences_execute_updates_when_their_result_is_hidden() {
    for suffix in ["", ":worker"] {
        let source = format!(
            "```mech{suffix}{{output: false, color: red}}\n~counter := 0\ncounter += 1\ncounter\n```\n"
        );
        let document = document(&source);
        let compiled = if suffix.is_empty() {
            CanonicalSourceFrontend.compile_document(&document)
        } else {
            CanonicalSourceFrontend.compile_named_document_scope(&document, "worker")
        }
        .unwrap();
        assert_eq!(
            compiled.document_outputs().len(),
            1,
            "only the program result is published"
        );
        compiled_turns(
            compiled,
            &source,
            &[1.0, 2.0],
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
    let source = "```mech:hidden{output: true}\n~counter := 0\ncounter += 1\ncounter\n```\n";
    let compiled = compiled(source);
    assert_eq!(
        compiled.document_outputs().len(),
        1,
        "a hidden fence executes without a presentation output"
    );
    compiled_turns(
        compiled,
        source,
        &[1.0, 2.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

#[test]
fn colonless_named_fences_execute_and_share_state_with_colon_spelling() {
    for info in ["mechworker", "mecworker", "🤖worker", "mechmechmec🤖worker"] {
        let source = format!(
            "~counter := 100\ncounter\n\n```{info}\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:worker\ncounter += 2\ncounter\n```\n"
        );
        turns(&source, &[100.0, 100.0]);
        let program = CanonicalSourceFrontend
            .compile_named_document_scope(&document(&source), "worker")
            .unwrap();
        compiled_turns(
            program,
            &source,
            &[3.0, 6.0],
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

#[test]
fn mika_bodies_execute_independently_of_parent_sibling_and_nested_state() {
    let source = "~counter := 100\ncounter += 10\ncounter\n\n~∘~⸢~counter := 0\ncounter += 1\ncounter\n\n╭◉╮⸢~counter := 20\ncounter += 2\ncounter\n⸥\n⸥\n\n~∘~⸢~counter := 40\ncounter += 4\ncounter\n⸥\n";
    let document = document(source);
    compiled_turns(
        CanonicalSourceFrontend.compile_document(&document).unwrap(),
        source,
        &[110.0, 120.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 3);
    for (scope, expected) in scopes.iter().zip([[1.0, 2.0], [22.0, 24.0], [44.0, 48.0]]) {
        let program = CanonicalSourceFrontend
            .compile_mika_section(&scope.section)
            .unwrap();
        compiled_turns(
            program,
            source,
            &expected,
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

#[test]
fn mika_named_fences_share_only_their_local_owner_and_keep_output_options() {
    let source = "```mech:worker\n~counter := 100\ncounter += 10\ncounter\n```\n\n~∘~⸢```mechworker{output: false}\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:worker\ncounter += 2\ncounter\n```\n⸥\n";
    let document = document(source);
    let root = CanonicalSourceFrontend
        .compile_named_document_scope(&document, "worker")
        .unwrap();
    compiled_turns(
        root,
        source,
        &[110.0, 120.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 1);
    let local = CanonicalSourceFrontend
        .compile_named_mika_scope(&scopes[0].section, "worker")
        .unwrap();
    assert_eq!(local.document_outputs().len(), 2);
    compiled_turns(
        local,
        source,
        &[3.0, 6.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

#[test]
fn finalized_streams_execute_configured_root_and_normalized_named_scopes() {
    use mech_syntax::document::{DocumentStream, StreamProgress};
    for scope in ["", "worker"] {
        let source = format!(
            "```mech{scope}{{output: false, color: red}}\n~counter := 0\ncounter += 1\ncounter\n```\n"
        );
        let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
        for ch in source.chars() {
            let mut progress = stream.append(&ch.to_string(), 17).unwrap().progress;
            for _ in 0..100_000 {
                if progress != StreamProgress::NeedsProcessing {
                    break;
                }
                progress = stream.advance(17).progress;
            }
            assert_eq!(progress, StreamProgress::NeedInput);
        }
        let mut progress = stream.finish(17).progress;
        for _ in 0..100_000 {
            if progress != StreamProgress::NeedsProcessing {
                break;
            }
            progress = stream.advance(17).progress;
        }
        assert_eq!(progress, StreamProgress::Finished);
        let snapshot = stream.materialize().unwrap();
        assert!(snapshot.is_strictly_clean());
        let document = DocumentSyntax::cast(snapshot.syntax()).unwrap();
        let program = if scope.is_empty() {
            CanonicalSourceFrontend.compile_document(&document)
        } else {
            CanonicalSourceFrontend.compile_named_document_scope(&document, scope)
        }
        .unwrap();
        assert_eq!(program.document_outputs().len(), 1);
        compiled_turns(
            program,
            &source,
            &[1.0, 2.0],
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

fn streamed_document(source: &str) -> DocumentSyntax {
    use mech_syntax::document::{DocumentStream, StreamProgress};
    let mut stream = DocumentStream::new(DocumentId(826), ParseConfig::default());
    for ch in source.chars() {
        let mut progress = stream.append(&ch.to_string(), 19).unwrap().progress;
        while progress == StreamProgress::NeedsProcessing {
            progress = stream.advance(19).progress;
        }
        assert_eq!(progress, StreamProgress::NeedInput);
    }
    let mut progress = stream.finish(19).progress;
    while progress == StreamProgress::NeedsProcessing {
        progress = stream.advance(19).progress;
    }
    assert_eq!(progress, StreamProgress::Finished);
    let snapshot = stream.materialize().unwrap();
    assert!(snapshot.is_strictly_clean());
    DocumentSyntax::cast(snapshot.syntax()).unwrap()
}

#[test]
fn finalized_streams_mika_bodies_execute_independently_of_parent_sibling_and_nested_state() {
    let source = "~counter := 100\ncounter += 10\ncounter\n\n~∘~⸢~counter := 0\ncounter += 1\ncounter\n\n╭◉╮⸢~counter := 20\ncounter += 2\ncounter\n⸥\n⸥\n\n~∘~⸢~counter := 40\ncounter += 4\ncounter\n⸥\n";
    let document = streamed_document(source);
    compiled_turns(
        CanonicalSourceFrontend.compile_document(&document).unwrap(),
        source,
        &[110.0, 120.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 3);
    for (scope, expected) in scopes.iter().zip([[1.0, 2.0], [22.0, 24.0], [44.0, 48.0]]) {
        let program = CanonicalSourceFrontend
            .compile_mika_section(&scope.section)
            .unwrap();
        compiled_turns(
            program,
            source,
            &expected,
            mech_engine::SourceDocumentOutputKind::Program,
        );
    }
}

#[test]
fn finalized_streams_mika_named_fences_share_only_their_local_owner_and_keep_output_options() {
    let source = "```mech:worker\n~counter := 100\ncounter += 10\ncounter\n```\n\n~∘~⸢```mechworker{output: false}\n~counter := 0\ncounter += 1\ncounter\n```\n\n```mech:worker\ncounter += 2\ncounter\n```\n⸥\n";
    let document = streamed_document(source);
    let root = CanonicalSourceFrontend
        .compile_named_document_scope(&document, "worker")
        .unwrap();
    compiled_turns(
        root,
        source,
        &[110.0, 120.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
    let scopes = document.mika_scopes();
    assert_eq!(scopes.len(), 1);
    let local = CanonicalSourceFrontend
        .compile_named_mika_scope(&scopes[0].section, "worker")
        .unwrap();
    assert_eq!(local.document_outputs().len(), 2);
    compiled_turns(
        local,
        source,
        &[3.0, 6.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

#[test]
fn repeated_matrix_compound_selectors_accumulate_in_occurrence_order() {
    for (source, expected) in [
        (
            "~a := [10 20; 30 40]\na[[1 1],:] += [1 2; 3 4]\na[1,1]\n",
            vec![14.0, 18.0],
        ),
        (
            "~a := [10 20; 30 40]\na[:,[1 1]] += [1 2; 3 4]\na[2,1]\n",
            vec![37.0, 44.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],[2 2]] += [1 2; 3 4]\na[1,2]\n",
            vec![30.0, 40.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1]] += [1; 3]\na[1,1]\n",
            vec![14.0, 18.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],:] += 2\na[1,2]\n",
            vec![24.0, 28.0],
        ),
        (
            "~a := [60 60; 30 40]\na[[1 1],:] /= [2 2; 3 3]\na[1,1]\n",
            vec![10.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],:] *= [2 2; 3 3]\na[1,1]\n",
            vec![60.0, 360.0],
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],:] -= [1 2; 3 4]\na[1,1]\n",
            vec![6.0, 2.0],
        ),
        (
            "~a := [2 2; 3 4]\na[[1 1],:] ^= [2 2; 3 3]\na[1,1]\n",
            vec![64.0],
        ),
    ] {
        turns(source, &expected);
    }
}

#[test]
fn repeated_compound_selections_preserve_exact_numeric_kinds() {
    for kind in ["i32", "u32", "f32"] {
        for selection in ["[1 1],:", ":,[1 1]", "[1 1],[2 2]", "[1 1]"] {
            let source = format!(
                "~a := [10<{kind}> 20<{kind}>; 30<{kind}> 40<{kind}>]\na[{selection}] += 2<{kind}>\ntotal := a[1,1] + a[1,2] + a[2,1] + a[2,2]\nanswer := total<f64>\nanswer\n"
            );
            let increment = if selection == "[1 1]" { 4.0 } else { 8.0 };
            turns(&source, &[100.0 + increment, 100.0 + 2.0 * increment]);
        }
    }
}

#[test]
fn repeated_compound_overflow_rejects_without_publishing_and_valid_retry_succeeds() {
    use mech_core::ResidentKernelError;
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::{CapturedValueInput, ResidentExecutionError};
    let source = "changes := signal<[i8]:2,2>\n~a := [120<i8> 0<i8>; 0<i8> 0<i8>]\na[[1 1],:] += changes\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x589, 0),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let before = instance.published_state_hash();
        for (deltas, expected) in [([1i8, 0, 20, 0], None), ([1, 0, 2, 0], Some(123.0))] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(deltas.into_iter().map(ValueDataDraft::I8).collect()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let input = CapturedValueInput {
                slot: instance.plan.inputs[0].slot,
                value: &value,
            };
            let result = instance
                .prepare_turn_values(&[input])
                .and_then(|prepared| prepared.publish());
            if let Some(expected) = expected {
                result.unwrap();
                let value = instance.copied_output(0).unwrap();
                assert!(matches!(value.data(), ValueData::F64(bits) if bits.to_f64() == expected));
            } else {
                assert!(
                    matches!(
                        result,
                        Err(ResidentExecutionError::Kernel {
                            error: ResidentKernelError::Arithmetic,
                            ..
                        })
                    ),
                    "{result:?}"
                );
                assert_eq!(instance.published_state_hash(), before);
                assert_eq!(instance.published_epoch(), mech_core::InstanceEpoch::ZERO);
            }
        }
    }
}

#[test]
fn selected_compound_assignment_preserves_arithmetic_before_destination_conversion() {
    for (target, source) in [("a", "~a := -1<i32>"), ("a[[1]]", "~a := [-1<i32>]")] {
        let result = if target == "a" { "a" } else { "a[1]" };
        turns(
            &format!(
                "{source}\n{target} += 0.5\nselected := {result}\nanswer := selected<f64>\nanswer\n"
            ),
            &[0.0],
        );
    }
}

#[test]
fn ordered_retained_roots_link_live_exports_and_preserve_caller_output_order() {
    use mech_engine::{CanonicalOrderedDocument, CanonicalOrderedImport};
    use std::collections::{BTreeMap, BTreeSet};
    let root = |identity, source: &str| {
        let parsed = parse_canonical_document(
            TextSnapshot::new(DocumentId(0x590 + identity as u64), Revision(1), source).unwrap(),
            ParseConfig::default(),
        );
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        CanonicalOrderedDocument {
            identity,
            document: DocumentSyntax::cast(parsed.syntax()).unwrap(),
            input_schemas: BTreeMap::new(),
            resource_writes: BTreeMap::new(),
            imports: BTreeMap::new(),
            resolved_modules: BTreeSet::new(),
        }
    };
    let dependency = root(1, "~counter := 0\ncounter += 1\n<+ counter\ncounter\n");
    let mut main = root(0, "answer := dep/counter + 1\nanswer\n");
    main.imports.insert(
        "dep/counter".to_owned(),
        CanonicalOrderedImport::RootExport {
            root: 1,
            name: "counter".to_owned(),
        },
    );
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let program = CanonicalSourceFrontend
        .compile_ordered_documents_with_catalog(
            &[dependency, main],
            std::sync::Arc::new(catalog.build().unwrap()),
        )
        .unwrap();
    assert_eq!(
        program
            .program()
            .outputs
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        ["answer", "counter"]
    );
    assert_eq!(program.program().states.len(), 1);
    assert!(
        program
            .source_map()
            .nodes
            .iter()
            .any(|anchor| anchor.anchor.document == DocumentId(0x590))
    );
    assert!(
        program
            .source_map()
            .nodes
            .iter()
            .any(|anchor| anchor.anchor.document == DocumentId(0x591))
    );
    compiled_turns(
        program,
        "ordered live roots",
        &[2.0, 3.0, 4.0],
        mech_engine::SourceDocumentOutputKind::Program,
    );
}

#[test]
fn terminal_logical_updates_do_not_require_a_gather_population_at_activation() {
    for target in ["a[mask,:]", "a[:,mask]", "a[mask,mask]", "a[mask]"] {
        let mask = if target == "a[mask]" {
            "[1 2; 3 4] <= n"
        } else {
            "[1; 2] <= n"
        };
        for (operator, expected) in [("=", [10.0, 10.0, 10.0]), ("+=", [11.0, 21.0, 31.0])] {
            let source = format!(
                "~a := [1 2; 3 4]\n~n := 0\nn += 1\nmask := {mask}\n{target} {operator} 10\na[1,1]\n"
            );
            turns(&source, &expected);
        }
    }
}

#[test]
fn promoted_repeated_compound_selectors_accumulate_each_occurrence() {
    turns(
        "~a := [10<i32> 20<i32>; 30<i32> 40<i32>]\na[[1 1],:] += 2.5\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n",
        &[14.0, 18.0],
    );
}

#[test]
fn nested_repeated_compound_selectors_accumulate_each_occurrence() {
    turns(
        "~a := [10 20; 30 40]\na[[1 1],:][:,1] += 2\na[1,1]\n",
        &[14.0, 18.0],
    );
}

#[test]
fn promoted_repeated_updates_use_canonical_conversion_after_each_operation() {
    for (kind, right) in [
        ("i8", "2.5"),
        ("i16", "2.5"),
        ("i32", "2.5"),
        ("u8", "2.5"),
        ("u16", "2.5"),
        ("u32", "2.5"),
        // The maintained Number contract intentionally has no lossless
        // i64/u64/i128/u128 + f64 promotion. Exercise their integer promotions.
        ("i64", "2<i128>"),
        ("i128", "2<i64>"),
        ("u64", "2<u128>"),
        ("u128", "2<u64>"),
    ] {
        let source = format!(
            "~a := [10<{kind}> 20<{kind}>; 30<{kind}> 40<{kind}>]\na[[1 1],:] += {right}\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n"
        );
        turns(&source, &[14.0, 18.0]);
    }
    for (start, operation, right, expected) in [
        ("-1", "+=", "0.5", 0.0),
        ("2", "*=", "1.5", 4.0),
        ("60", "/=", "2.5", 9.0),
        ("10", "-=", "2.5", 4.0),
    ] {
        let source = format!(
            "~a := [{start}<i32> 20<i32>; 30<i32> 40<i32>]\na[[1 1],:] {operation} {right}\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n"
        );
        turns(&source, &[expected]);
    }
}

#[test]
fn nested_compound_updates_keep_row_major_occurrence_order_and_promotions() {
    for (source, expected) in [
        (
            "~a := [10 20; 30 40]\na[[1 1],:][:,1] += [1;3]\na[1,1]\n",
            14.0,
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],[2 2]][:,:] += [1 2;3 4]\na[1,2]\n",
            30.0,
        ),
        (
            "~a := [60 20; 30 40]\na[[1 1],:][:,1] /= [2;3]\na[1,1]\n",
            10.0,
        ),
        (
            "~a := [10 20; 30 40]\na[[1 1],:][:,1][:] += [1;3]\na[1,1]\n",
            14.0,
        ),
        (
            "~a := [10<i32> 20<i32>; 30<i32> 40<i32>]\na[[1 1],:][:,1] += 2.5\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n",
            14.0,
        ),
        (
            "~a := [10<i32> 20<i32>; 30<i32> 40<i32>]\na[[1 1],[2 2]][:,:] += [1<i32> 2<i32>;3<i32> 4<i32>]\nselected := a[1,2]\nanswer := selected<f64>\nanswer\n",
            30.0,
        ),
    ] {
        turns(source, &[expected]);
    }
}

#[test]
fn nested_promoted_failure_preserves_state_and_valid_retry_accumulates() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::CapturedValueInput;
    let source = "divisors := signal<[i16]:2,1>\n~a := [120<i8> 0<i8>;0<i8> 0<i8>]\na[[1 1],:][:,1] /= divisors\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x595, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let before = instance.published_state_hash();
        for (values, succeeds) in [([2i16, 0], false), ([2i16, 3], true)] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(values.into_iter().map(ValueDataDraft::I16).collect()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let input = CapturedValueInput {
                slot: instance.plan.inputs[0].slot,
                value: &value,
            };
            let result = instance
                .prepare_turn_values(&[input])
                .and_then(|prepared| prepared.publish());
            if succeeds {
                result.unwrap();
                assert!(
                    matches!(instance.copied_output(0).unwrap().data(), ValueData::F64(v) if v.to_f64() == 20.0)
                );
            } else {
                assert!(
                    matches!(
                        result,
                        Err(mech_engine::resident::ResidentExecutionError::Kernel {
                            error: mech_core::ResidentKernelError::Arithmetic,
                            ..
                        })
                    ),
                    "{result:?}"
                );
                assert_eq!(instance.published_state_hash(), before);
            }
        }
    }
}

#[test]
fn nested_selected_rational_power_keeps_its_integer_exponent_contract() {
    turns(
        "~a := [2<r64> 3<r64>;4<r64> 5<r64>]\na[[1 1],:][1,1] ^= 2<i32>\nselected := a[1,1]\nanswer := selected<f64>\nanswer\n",
        &[4.0, 16.0],
    );
}

#[test]
fn promoted_boolean_masks_preserve_canonical_destination_coordinates() {
    turns(
        "~a := [10<i32> 20<i32>;30<i32>40<i32>]\na[[false true;false false]] += 2.5\nselected := a[1,2]\nanswer := selected<f64>\nanswer\n",
        &[22.0, 24.0],
    );
}

#[test]
fn promoted_selected_updates_preserve_row_and_column_broadcasts() {
    for (selection, rhs, expected) in [
        (
            "a[[1 2],:]",
            "[1.5 2.5 3.5]",
            [[11, 22, 33, 41, 52, 63], [12, 24, 36, 42, 54, 66]],
        ),
        (
            "a[:,[1 2 3]]",
            "[1.5;2.5]",
            [[11, 21, 31, 42, 52, 62], [12, 22, 32, 44, 54, 64]],
        ),
        (
            "a[[1 1],:][:,[1 2 3]]",
            "[1.5 2.5 3.5]",
            [[12, 24, 36, 40, 50, 60], [14, 28, 42, 40, 50, 60]],
        ),
        (
            "a[[1 1],:][:,[1 2 3]]",
            "[1.5;2.5]",
            [[13, 23, 33, 40, 50, 60], [16, 26, 36, 40, 50, 60]],
        ),
        (
            "a[[1 1],:]",
            "[1<i64> 2<i64> 3<i64>]",
            [[12, 24, 36, 40, 50, 60], [14, 28, 42, 40, 50, 60]],
        ),
        (
            "a[[1 1],:][:,[1 2 3]]",
            "[1<i32>;2<i32>]",
            [[13, 23, 33, 40, 50, 60], [16, 26, 36, 40, 50, 60]],
        ),
    ] {
        let source = format!(
            "~a := [10<i32> 20<i32> 30<i32>;40<i32> 50<i32> 60<i32>]\n{selection} += {rhs}\na\n"
        );
        let artifact = compiled(&source).compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        for artifact in [
            artifact,
            mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
        ] {
            let mut catalog = FunctionCatalogBuilder::new();
            mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x599, 0),
                &artifact,
                &catalog.build().unwrap(),
                &ActivationFacts::default(),
            )
            .unwrap();
            for expected in expected {
                instance.turn(&[]).unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::Matrix(matrix) = output.data() else {
                    panic!("expected complete matrix: {output:?}")
                };
                let mech_core::snapshot::SequenceView::I32(values) = matrix.elements() else {
                    panic!("expected i32 matrix: {output:?}")
                };
                assert_eq!(
                    values, &expected,
                    "complete source/decoded matrix for {source:?}"
                );
            }
        }
    }
}

#[test]
fn promoted_broadcast_failure_preserves_the_complete_state_before_retry() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::CapturedValueInput;
    let source = "divisors := signal<[i16]:1,3>\n~a := [120<i8> 120<i8> 120<i8>;60<i8> 60<i8> 60<i8>]\na[[1 1],:][:,[1 2 3]] /= divisors\na\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59a, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let before = instance.published_state_hash();
        for (values, succeeds) in [([2i16, 0, 3], false), ([2i16, 3, 4], true)] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(values.into_iter().map(ValueDataDraft::I16).collect()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let input = CapturedValueInput {
                slot: instance.plan.inputs[0].slot,
                value: &value,
            };
            let result = instance
                .prepare_turn_values(&[input])
                .and_then(|prepared| prepared.publish());
            if succeeds {
                result.unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::Matrix(matrix) = output.data() else {
                    panic!("expected complete matrix: {output:?}")
                };
                let mech_core::snapshot::SequenceView::I8(values) = matrix.elements() else {
                    panic!("expected i8 matrix: {output:?}")
                };
                assert_eq!(values, &[30, 13, 7, 60, 60, 60]);
            } else {
                assert!(
                    matches!(
                        result,
                        Err(mech_engine::resident::ResidentExecutionError::Kernel {
                            error: mech_core::ResidentKernelError::Arithmetic,
                            ..
                        })
                    ),
                    "{result:?}"
                );
                assert_eq!(instance.published_state_hash(), before);
            }
        }
    }
}

#[test]
fn review_nested_composite_matrix_field_update() {
    turns(
        "~a := [{value: 1} {value: 4}]\na[1].value += 1\na[1].value + a[2].value\n",
        &[6.0, 7.0],
    );
}

fn matrix_turns(source: &str, expected: &[Vec<f64>]) {
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59b, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
        for expected in expected {
            instance.turn(&[]).unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::Matrix(matrix) = output.data() else {
                panic!("{output:?}")
            };
            let actual: Vec<f64> = match matrix.elements() {
                mech_core::snapshot::SequenceView::I32(values) => {
                    values.iter().map(|v| f64::from(*v)).collect()
                }
                mech_core::snapshot::SequenceView::F64(values) => {
                    values.iter().map(|v| v.to_f64()).collect()
                }
                _ => panic!("{output:?}"),
            };
            assert_eq!(
                &actual, expected,
                "complete source/decoded output for {source:?}"
            );
        }
    }
}

#[test]
fn turn_mask_broadcast_uses_live_population_and_preserves_complete_matrices() {
    for kind in ["i32", "f64"] {
        for (selection, rhs, thresholds, increments) in [
            (
                "a[mask,:]",
                "[1.5 2.5 3.5]",
                "[1;2]",
                [1.5, 2.5, 3.5, 1.5, 2.5, 3.5],
            ),
            (
                "a[:,mask]",
                "[1.5;2.5]",
                "[1 2 2]",
                [1.5, 1.5, 1.5, 2.5, 2.5, 2.5],
            ),
            (
                "a[mask,[1 2 3]]",
                "[1.5 2.5 3.5]",
                "[1;2]",
                [1.5, 2.5, 3.5, 1.5, 2.5, 3.5],
            ),
            (
                "a[mask,:][:,[1 2 3]]",
                "[1.5 2.5 3.5]",
                "[1;2]",
                [1.5, 2.5, 3.5, 1.5, 2.5, 3.5],
            ),
        ] {
            let source = format!(
                "~a := [10<{kind}> 20<{kind}> 30<{kind}>;40<{kind}> 50<{kind}> 60<{kind}>]\n~n := -1\nn += 1\nmask := {thresholds} <= n\n{selection} += {rhs}\na\n"
            );
            let initial = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0];
            let mut next = initial.clone();
            let mut expected = vec![initial];
            for population in 1..=2 {
                for index in 0..6 {
                    let selected = if selection == "a[:,mask]" {
                        index % 3 == 0 || population == 2
                    } else {
                        index < 3 || population == 2
                    };
                    if selected {
                        next[index] += if kind == "i32" {
                            f64::trunc(increments[index])
                        } else {
                            increments[index]
                        };
                    }
                }
                expected.push(next.clone());
            }
            matrix_turns(&source, &expected);
        }
    }
}

#[test]
fn logical_selected_updates_route_full_base_rhs_by_destination_position() {
    for (masks, selection, expected) in [
        (
            "rmask := [true;false]",
            "a[rmask,:]",
            vec![11.0, 22.0, 33.0, 40.0, 50.0, 60.0],
        ),
        (
            "cmask := [true false true]",
            "a[:,cmask]",
            vec![11.0, 20.0, 33.0, 50.0, 50.0, 90.0],
        ),
        (
            "rmask := [true;false]\ncmask := [false true true]",
            "a[rmask,cmask]",
            vec![10.0, 22.0, 33.0, 40.0, 50.0, 60.0],
        ),
    ] {
        let source = format!(
            "~a := [10<i32> 20<i32> 30<i32>;40<i32> 50<i32> 60<i32>]\n{masks}\nrhs := [1.5 2.5 3.5;10.5 20.5 30.5]\n{selection} += rhs\na\n"
        );
        matrix_turns(&source, &[expected]);
    }
}

#[test]
fn nested_logical_routing_uses_immediate_view_coordinates() {
    for (selection, rhs, expected) in [
        (
            "a[[2 1],:][[true;false],:]",
            "[1.5 2.5;10.5 20.5]",
            vec![10.0, 20.0, 31.0, 42.0],
        ),
        (
            "a[:,[2 1]][:,[true false]]",
            "[1.5 2.5;10.5 20.5]",
            vec![10.0, 21.0, 30.0, 50.0],
        ),
        (
            "a[[4 1]][[true;false]]",
            "[1.5;10.5]",
            vec![10.0, 20.0, 30.0, 41.0],
        ),
    ] {
        let source = format!(
            "~a := [10<i32> 20<i32>;30<i32> 40<i32>]\nrhs := {rhs}\n{selection} += rhs\na\n"
        );
        matrix_turns(&source, &[expected]);
    }
    matrix_turns(
        "~a := [10<i32> 20<i32>;30<i32> 40<i32>;50<i32> 60<i32>]\n\
         rhs := [1.5 2.5;10.5 20.5]\n\
         a[[3 1],:][[true;false],:] += rhs\n\
         a\n",
        &[vec![10.0, 20.0, 30.0, 40.0, 51.0, 62.0]],
    );
}

#[test]
fn sparse_nested_update_does_not_materialize_base_sized_addresses() {
    // Keep the executable witness at the resident target's 65,536-element
    // output ceiling. The artifact assertions are the scale-independent proof
    // that no base-sized identity/address helper survives at larger sizes.
    const SIZE: usize = 256;
    let row = std::iter::repeat_n("1<f64>", SIZE)
        .collect::<Vec<_>>()
        .join(" ");
    let rows = std::iter::repeat_n("row", SIZE)
        .collect::<Vec<_>>()
        .join(";");
    let source = format!("row := [{row}]\n~a := [{rows}]\na[1,:][1] += 1\na[1,1]\n");
    let artifact = compiled(&source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let operations = artifact
            .nodes()
            .iter()
            .filter_map(|node| node.as_operation())
            .map(|operation| {
                (
                    operation.operation.module_path.as_ref(),
                    operation.operation.operation_name.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert!(
            operations
                .iter()
                .any(|(path, name)| { *path == ["core", "assign", "nested"] && *name == "add" })
        );
        assert!(!operations.iter().any(|(path, name)| {
            *path == ["core", "assign"]
                && matches!(*name, "identity-indices" | "broadcast" | "selection-order")
        }));
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59d, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        instance.turn(&[]).unwrap();
        assert!(
            matches!(instance.copied_output(0).unwrap().data(), ValueData::F64(value) if value.to_f64() == 2.0)
        );
    }
}

#[test]
fn nested_snapshot_helpers_report_signed_zero_representation_changes() {
    use mech_core::snapshot::{F32Bits, SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::CapturedValueInput;

    let source = "delta := signal<[f32]:1,1>\n~a := [-1<f32> 2<f32>;-1<f32> 2<f32>]\na[[1 2],:][:,1] *= delta\nselected := a[1,1]\nanswer := selected<f64>\n1 / answer\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59e, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for (delta, expected) in [(0.0_f32, f64::NEG_INFINITY), (-0.0, f64::INFINITY)] {
            let value = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(
                    vec![ValueDataDraft::F32(F32Bits::from_f32(delta))].into_boxed_slice(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let input = CapturedValueInput {
                slot: instance.plan.inputs[0].slot,
                value: &value,
            };
            instance
                .prepare_turn_values(&[input])
                .unwrap()
                .publish()
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let ValueData::F64(actual) = output.data() else {
                panic!("expected f64 output: {output:?}")
            };
            assert_eq!(
                actual.to_f64().to_bits(),
                expected.to_bits(),
                "delta {delta:?}"
            );
        }
    }
}

#[test]
fn review_asymmetric_rectangle_admission() {
    let row = std::iter::repeat_n("10<i32>", 1000)
        .collect::<Vec<_>>()
        .join(" ");
    let columns = (1..=1000)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    turns(
        &format!(
            "~a := [{row}]\na[[1],[{columns}]] += 2.5\nselected := a[1,1000]\nanswer := selected<f64>\nanswer\n"
        ),
        &[12.0, 14.0],
    );
}

#[test]
fn selected_f64_matrix_updates_propagate_signed_zero_changes() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_engine::resident::CapturedValueInput;

    fn checked_update(
        kernel: &mech_core::BoundResidentKernel,
        inputs: &dyn mech_core::ResidentKernelInputs,
        output: mech_core::ResidentValueMut<'_>,
    ) -> Result<bool, mech_core::ResidentKernelError> {
        struct Inputs<'a>(&'a dyn mech_core::ResidentKernelInputs);
        impl mech_core::ResidentKernelInputs for Inputs<'_> {
            fn len(&self) -> usize {
                self.0.len()
            }
            fn get(&self, index: usize) -> Option<mech_core::ResidentValueRef<'_>> {
                self.0.get(index)
            }
        }
        let mech_core::ResidentValueMut::F64(target) = output else {
            panic!("expected dense F64 selected update")
        };
        let before: Vec<_> = target.iter().map(|x| x.to_bits()).collect();
        let changed = kernel
            .retained_state::<mech_core::BoundResidentKernel>()
            .unwrap()
            .execute(&Inputs(inputs), mech_core::ResidentValueMut::F64(target))?;
        assert_eq!(
            changed,
            before
                .iter()
                .zip(target.iter())
                .any(|(old, new)| *old != new.to_bits()),
            "kernel must report representation changes"
        );
        Ok(changed)
    }

    for selection in ["a[1,:]", "a[:,1]", "a[[1],[1 2]]"] {
        let source = format!(
            "delta := signal<f64>\n~a := [-0.0 -0.0;-0.0 -0.0]\n{selection} += [delta]\n1 / a[1,1]\n"
        );
        let artifact = compiled(&source).compile_artifact().unwrap();
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
        for artifact in [
            artifact,
            mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
        ] {
            let mut catalog = FunctionCatalogBuilder::new();
            mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x59c, 0),
                &artifact,
                &catalog.build().unwrap(),
                &ActivationFacts::default(),
            )
            .unwrap();
            let updates: Vec<_> = instance
                .plan
                .steps
                .iter()
                .enumerate()
                .filter_map(|(index, step)| match step {
                    mech_engine::resident::ActivatedTurnStep::Kernel(node)
                        if matches!(
                            node.construction,
                            mech_core::OutputConstruction::ReadModifyWrite { .. }
                        ) =>
                    {
                        Some((index, node.kernel.clone()))
                    }
                    _ => None,
                })
                .collect();
            assert!(!updates.is_empty());
            for (index, kernel) in updates {
                instance.plan.replace_kernel_for_test(
                    index,
                    mech_core::BoundResidentKernel::new(checked_update, Box::new([]))
                        .with_retained_state(std::sync::Arc::new(kernel)),
                );
            }
            for (delta, expected) in [
                (-0.0, f64::NEG_INFINITY),
                (0.0, f64::INFINITY),
                (-0.0, f64::INFINITY),
            ] {
                let value = ValueDraft {
                    schema: artifact.inputs()[0].schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(delta)),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let input = CapturedValueInput {
                    slot: instance.plan.inputs[0].slot,
                    value: &value,
                };
                instance
                    .prepare_turn_values(&[input])
                    .unwrap()
                    .publish()
                    .unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::F64(actual) = output.data() else {
                    panic!("{output:?}")
                };
                assert_eq!(
                    actual.to_f64().to_bits(),
                    expected.to_bits(),
                    "{selection}: delta {delta:?}"
                );
            }
        }
    }
}

#[test]
fn closed_match_initializer_runs_once_before_state_turns() {
    turns(
        "x := (true ? | true => 1 | false => 2)\n~a := x\na += 1\na\n",
        &[2.0, 3.0],
    );
}

#[test]
fn closed_match_converts_dense_matrix_capture_to_snapshot_state() {
    closed_matrix_turns(
        "xs := [1 2]\nx := (true ? | true => xs | false => xs)\n~a := x\na\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (1, 2));
            assert_eq!(matrix_values(actual), [1.0, 2.0]);
        },
    );
    variable_matrix_turns(
        "x := (true ? | true => signal<[f64]:1,2> | false => [0 0])\nx\n",
        &[
            (Some([1.0, 2.0]), (1, 2), &[1.0, 2.0]),
            (Some([3.0, 4.0]), (1, 2), &[3.0, 4.0]),
        ],
    );
    variable_matrix_turns(
        "input := signal<[f64]:1,2>\nmatched := (true ? | true => input | false => [0 0])\n~a := [0 0]\na = matched\n[1 / a[1] 1 / a[2]]\n",
        &[
            (Some([0.0, 0.0]), (1, 2), &[f64::INFINITY, f64::INFINITY]),
            (
                Some([-0.0, 0.0]),
                (1, 2),
                &[f64::NEG_INFINITY, f64::INFINITY],
            ),
        ],
    );
}

#[test]
fn dense_match_snapshot_materialization_is_admitted_before_allocation() {
    use mech_engine::resident::{
        CapturedSignalInput, ResidentActivationOptions, ResidentExecutionError,
        activate_with_options,
    };

    let direct = compiled("signal<[string]:1,2>\n")
        .compile_artifact()
        .unwrap();
    let matched =
        compiled("x := (true ? | true => signal<[string]:1,2> | false => [\"\" \"\"])\nx\n")
            .compile_artifact()
            .unwrap();
    let matched_bytes = mech_engine::encode_program_artifact_bytecode_v1(&matched).unwrap();
    let large = ["x".repeat(64 * 1024), "y".repeat(64 * 1024)];
    let small = ["ok".to_owned(), "retry".to_owned()];
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();

    let fits = |artifact: &mech_engine::ProgramArtifact, limit: u64, turn: bool| {
        let budget = mech_core::ManagedMemoryBudget::new(limit);
        let result = activate_with_options(
            ReactiveInstanceId::new(0x5a1, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
            ResidentActivationOptions {
                memory_budget: Some(budget.clone()),
                ..Default::default()
            },
        );
        let admitted = match result {
            Ok(mut instance) => {
                let admitted = if turn {
                    let slot = instance.plan.inputs[0].slot;
                    instance
                        .turn(&[CapturedSignalInput {
                            slot,
                            value: ResidentValueRef::String(&large),
                        }])
                        .is_ok()
                } else {
                    true
                };
                drop(instance);
                admitted
            }
            Err(_) => false,
        };
        assert_eq!(budget.used_bytes(), 0, "limit {limit} leaked ownership");
        admitted
    };
    let minimum = |artifact: &mech_engine::ProgramArtifact, turn| {
        let mut low = 0_u64;
        let mut high = 1_u64 << 24;
        assert!(fits(artifact, high, turn));
        while low < high {
            let middle = low + (high - low) / 2;
            if fits(artifact, middle, turn) {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        low
    };
    let direct_turn = minimum(&direct, true);
    let match_activation = minimum(&matched, false);
    let match_turn = minimum(&matched, true);
    let failure_limit = match_turn - 1;
    assert!(failure_limit >= direct_turn);
    assert!(failure_limit >= match_activation);

    for artifact in [
        matched,
        mech_engine::decode_program_artifact_bytecode_v1(&matched_bytes).unwrap(),
    ] {
        let budget = mech_core::ManagedMemoryBudget::new(failure_limit);
        let mut instance = activate_with_options(
            ReactiveInstanceId::new(0x5a1, 1),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
            ResidentActivationOptions {
                memory_budget: Some(budget.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        let error = instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&large),
            }])
            .unwrap_err();
        assert!(matches!(
            error,
            ResidentExecutionError::MemoryRuntime { .. }
        ));
        instance
            .turn(&[CapturedSignalInput {
                slot,
                value: ResidentValueRef::String(&small),
            }])
            .unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::Matrix(matrix) = output.data() else {
            panic!("expected String matrix: {output:?}")
        };
        let SequenceView::String(values) = matrix.elements() else {
            panic!("expected packed String matrix: {matrix:?}")
        };
        assert_eq!(
            values.iter().map(AsRef::as_ref).collect::<Vec<&str>>(),
            small.iter().map(String::as_str).collect::<Vec<_>>()
        );
        drop(output);
        drop(instance);
        assert_eq!(budget.used_bytes(), 0);
    }
}

#[test]
fn closed_comprehension_scalar_initializer_runs_once_before_state_turns() {
    turns(
        "samples := 1..=3\nvalues := [sample + 1 | sample <- samples]\n~a := values[1]\na += 1\na\n",
        &[3.0, 4.0],
    );
}

#[test]
fn activation_derived_range_endpoints_initialize_runtime_shaped_state() {
    variable_matrix_turns(
        "start := (true ? | true => 1 | false => 2)\nvalues := start..=3\n~state := values\nstate\n",
        &[
            (None, (1, 3), &[1.0, 2.0, 3.0]),
            (None, (1, 3), &[1.0, 2.0, 3.0]),
        ],
    );
}

#[test]
fn activation_derived_index_range_endpoints_initialize_runtime_shaped_state() {
    closed_matrix_turns(
        "start := (true ? | true => 1<index> | false => 2<index>)\nvalues := start..=3<index>\n~state := values\nstate\n",
        |actual| {
            assert_eq!(matrix_shape(actual), (1, 3));
            assert_eq!(index_matrix_values(actual), [1, 2, 3]);
        },
    );
}

#[test]
fn runtime_shaped_matrix_can_be_wrapped_in_an_option() {
    use mech_core::{
        ValueDataDraft,
        snapshot::{F64Bits, OptionDraft},
    };

    closed_matrix_turns(
        "values := [x | x <- [1 2 3]]\nwrapped<[f64]?> := values\nwrapped\n",
        |actual| {
            assert_eq!(
                actual.canonical_data_draft().unwrap(),
                ValueDataDraft::Option(OptionDraft {
                    present: true,
                    value: Some(Box::new(ValueDataDraft::Matrix(
                        [1.0, 2.0, 3.0]
                            .into_iter()
                            .map(|value| ValueDataDraft::F64(F64Bits::from_f64(value)))
                            .collect(),
                    ))),
                })
            );
        },
    );
}

#[test]
fn activation_match_compares_snapshot_backed_scalar_literals() {
    for source in [
        "selected := (1u8 ? | 1u8 => 2u8 | * => 3u8)\n~state := selected\nstate\n",
        "selected := (-0.0<f32> ? | 0.0<f32> => 2u8 | * => 3u8)\n~state := selected\nstate\n",
    ] {
        closed_matrix_turns(source, |actual| {
            assert_eq!(
                actual.canonical_data_draft().unwrap(),
                mech_core::ValueDataDraft::U8(2)
            );
        });
    }
}

#[test]
fn runtime_shaped_selection_resolves_complete_result_geometry() {
    for (selection, expected_shape, expected_values) in [
        ("a[1,:]", (1, 2), &[3.0, 4.0][..]),
        ("a[:,[1 2]]", (1, 2), &[3.0, 4.0][..]),
        ("a[[1],[1 2]]", (1, 2), &[3.0, 4.0][..]),
    ] {
        let source = format!(
            "samples := [1 2 3]\nvalues := [sample + 1 | sample <- samples, sample > 1]\n~a := values\n{selection}\n"
        );
        variable_matrix_turns(
            &source,
            &[
                (None, expected_shape, expected_values),
                (None, expected_shape, expected_values),
            ],
        );
    }

    variable_matrix_turns(
        "samples := signal<[f64]:1,2>\nvalues := [sample + 1 | sample <- samples, sample > 0]\nvalues[1,:]\n",
        &[
            (Some([1.0, 2.0]), (1, 2), &[2.0, 3.0]),
            (Some([-1.0, 2.0]), (1, 1), &[3.0]),
        ],
    );
}

#[test]
fn closed_set_comprehension_initializes_once_outside_the_turn_schedule() {
    let source = "samples := 1..=3\nvalues := {sample + 1 | sample <- samples}\n~a := values\na\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(0x59e, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let control = artifact
            .nodes()
            .iter()
            .find(|node| matches!(node.body, mech_engine::ExecutableNodeBody::Comprehension(_)))
            .unwrap()
            .node;
        assert!(instance.plan.activation_nodes.contains(&control));
        assert!(
            instance
                .plan
                .topology
                .linear_node_order
                .iter()
                .all(|index| instance.plan.steps[index.get() as usize].artifact_node() != control)
        );
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            let output = instance.copied_output(0).unwrap();
            assert_eq!(
                output.canonical_data_draft().unwrap(),
                mech_core::ValueDataDraft::Set(
                    [2.0, 3.0, 4.0]
                        .into_iter()
                        .map(|value| mech_core::ValueDataDraft::F64(
                            mech_core::snapshot::F64Bits::from_f64(value)
                        ))
                        .collect()
                )
            );
        }
    }
}

#[test]
fn closed_comprehension_initializer_runs_once_before_state_turns() {
    turns(
        "samples := 1..=3\nx-row := [1.0 | sample <- samples]\ny-row := [2.0 | sample <- samples]\nx := x-row'\ny := y-row'\n~trail := [x y]\nnew-row := ([7.0 8.0])\nnext-trail := matrix/vertcat(trail[2..=3,:], new-row)\ntrail = next-trail\ntrail[3,2]\n",
        &[8.0, 8.0],
    );
}

#[test]
fn control_initializer_rejects_live_input_dependencies() {
    for source in [
        "condition := signal<bool>\nx := (condition ? | true => 1 | false => 2)\n~a := x\na\n",
        "items := signal<[f64]:1,3>\nx := [item + 1 | item <- items]\n~a := x\na\n",
    ] {
        assert_unavailable_control_initializer(source);
    }
}

#[test]
fn control_initializer_rejects_state_dependencies() {
    for source in [
        "~condition := true\nx := (condition ? | true => 1 | false => 2)\n~a := x\na\n",
        "~items := [1 2 3]\nx := [item + 1 | item <- items]\n~a := x\na\n",
    ] {
        assert_unavailable_control_initializer(source);
    }
}

fn assert_unavailable_control_initializer(source: &str) {
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
        let error = activate(
            ReactiveInstanceId::new(0x59d, 0),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .err()
        .expect("live-only control dependencies cannot initialize persistent state");
        assert!(matches!(error, mech_engine::resident::ResidentActivationError::InitializerUnavailableAtActivation { .. }), "{source:?}: {error:?}");
    }
}

#[test]
fn closed_control_activation_reports_budget_exhaustion_and_releases_ownership() {
    use mech_engine::resident::{
        ResidentActivationError, ResidentActivationOptions, activate_with_options,
    };
    let source = "samples := 1..=32\nvalues := {sample + 1 | sample <- samples}\n~a := values\na\n";
    let artifact = compiled(source).compile_artifact().unwrap();
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [
        artifact,
        mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap(),
    ] {
        let mut admitted = false;
        // Exercise allocation failures both before and during control execution,
        // then admit that same artifact without changing its semantic contract.
        for limit in (512..=262_144).step_by(512) {
            let budget = mech_core::ManagedMemoryBudget::new(limit);
            let result = activate_with_options(
                ReactiveInstanceId::new(0x59f, 0),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
                ResidentActivationOptions {
                    memory_budget: Some(budget.clone()),
                    ..Default::default()
                },
            );
            match result {
                Ok(instance) => {
                    assert!(budget.used_bytes() > 0);
                    drop(instance);
                    admitted = true;
                }
                Err(error) => assert!(
                    matches!(error, ResidentActivationError::MemoryRuntime { .. }),
                    "budget {limit}: {error:?}"
                ),
            }
            assert_eq!(budget.used_bytes(), 0, "budget {limit} leaked ownership");
            if admitted {
                break;
            }
        }
        assert!(
            admitted,
            "closed control must activate within its finite budget"
        );
    }
}
