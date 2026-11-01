//! Structural authority and durable identity of horizontal matrix blocks.
#![cfg(feature = "compiler")]

use mech_core::{
    ConstantStoreBuilder, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, FloatWidth, Schema, SchemaBody, SchemaDraft,
    SchemaTableBuilder,
};
use mech_engine::*;

fn parameter(id: u32, lifetime: DimensionLifetime, lower: u64) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(id),
        origin: DimensionParameterOrigin::Explicit,
        lifetime,
        lower_bound: DimensionExpr::Constant(lower),
        upper_bound: None,
    }
}

fn axis(id: u32) -> DimensionExpr {
    DimensionExpr::Parameter(DimensionParameterId::new(id))
}

fn matrix(
    element: SchemaBody,
    dimensions: [DimensionExpr; 2],
    parameters: Vec<DimensionParameterDeclaration>,
) -> Schema {
    SchemaDraft {
        dimension_parameters: parameters.into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: Box::new(dimensions),
        },
    }
    .finalize()
    .unwrap()
}

fn fixed(rows: u64, columns: u64) -> Schema {
    matrix(
        SchemaBody::FloatingPoint(FloatWidth::W64),
        [
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ],
        vec![],
    )
}

fn result(element: SchemaBody, rows: MatrixBlockRows) -> Schema {
    let (rows, parameters, column) = match rows {
        MatrixBlockRows::Fixed(rows) => (
            DimensionExpr::Constant(rows),
            vec![parameter(0, DimensionLifetime::Turn, 0)],
            axis(0),
        ),
        _ => (
            axis(0),
            vec![
                parameter(0, DimensionLifetime::Turn, 0),
                parameter(1, DimensionLifetime::Turn, 0),
            ],
            axis(1),
        ),
    };
    matrix(element, [rows, column], parameters)
}

fn build(
    yielded: Schema,
    output: Schema,
    rows: MatrixBlockRows,
    witness: Option<Schema>,
) -> Result<ProgramArtifact, ArtifactBuildError> {
    build_kind(
        yielded,
        output,
        ComprehensionKind::MatrixHorizontalBlocks { rows },
        witness,
    )
}

fn build_kind(
    yielded: Schema,
    output: Schema,
    kind: ComprehensionKind,
    witness: Option<Schema>,
) -> Result<ProgramArtifact, ArtifactBuildError> {
    let mut schemas = SchemaTableBuilder::new();
    let yielded = schemas.insert(yielded).unwrap();
    let output = schemas.insert(output).unwrap();
    let witness = witness.map(|schema| schemas.insert(schema).unwrap());
    let build = schemas.finish().unwrap();
    let yielded = build.resolve(yielded).unwrap();
    let output = build.resolve(output).unwrap();
    let witness = witness.map(|schema| build.resolve(schema).unwrap());
    let (schemas, _) = build.into_parts();
    let (constants, _) = ConstantStoreBuilder::new(&schemas)
        .finish()
        .unwrap()
        .into_parts();
    let mut inputs = vec![SourceInput {
        name: "block".to_owned(),
        schema: yielded,
    }];
    if let Some(schema) = witness {
        inputs.push(SourceInput {
            name: "rows".to_owned(),
            schema,
        });
    }
    let graph = SourceProgram {
        inputs: inputs.into_boxed_slice(),
        nodes: vec![SourceNode {
            body: SourceNodeBody::Comprehension(ComprehensionDeclaration {
                id: ControlBlockId(0),
                kind,
                steps: Box::new([]),
                yield_value: ComprehensionValue::Input(0),
            }),
            inputs: (0..if witness.is_some() { 2 } else { 1 })
                .map(SourceValue::Input)
                .collect(),
            outputs: vec![SourceNodeOutput::Derived { schema: output }].into_boxed_slice(),
        }]
        .into_boxed_slice(),
        outputs: vec![SourceOutput {
            name: "result".to_owned(),
            interactive_symbol: None,
            source: SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            },
            schema: output,
        }]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    compile_source_program_with_control_contracts(
        &graph,
        &mut ArtifactBuildContext::new(&schemas, &constants),
        &[None],
    )
}

fn invalid(result: Result<ProgramArtifact, ArtifactBuildError>) {
    assert!(
        matches!(result, Err(ArtifactBuildError::InvalidControl { .. })),
        "{result:?}"
    );
}

#[test]
fn horizontal_block_policies_are_round_trip_revision_authority() {
    let scalar = SchemaBody::FloatingPoint(FloatWidth::W64);
    let mut revisions = Vec::new();
    for rows in [
        MatrixBlockRows::InferOrZero,
        MatrixBlockRows::Fixed(6),
        MatrixBlockRows::InputMatrixElements { input: 1 },
    ] {
        let artifact = build(
            fixed(6, 2),
            result(scalar.clone(), rows),
            rows,
            Some(fixed(2, 3)),
        )
        .unwrap();
        let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert_eq!(artifact.nodes(), decoded.nodes());
        assert_eq!(artifact.revision(), decoded.revision());
        assert_eq!(
            bytes,
            encode_program_artifact_bytecode_v1(&decoded).unwrap()
        );
        revisions.push(artifact.revision());
    }
    assert_ne!(revisions[0], revisions[1]);
    assert_ne!(revisions[0], revisions[2]);
    assert_ne!(revisions[1], revisions[2]);
    // Declared block rows do not prove a body will execute. Actual yielded
    // blocks, not skipped bodies, are checked against this witness at runtime.
    build(
        fixed(6, 2),
        result(scalar.clone(), MatrixBlockRows::InferOrZero),
        MatrixBlockRows::InputMatrixElements { input: 0 },
        Some(fixed(2, 3)),
    )
    .unwrap();
    build(
        fixed(2, 1),
        result(scalar.clone(), MatrixBlockRows::Fixed(3)),
        MatrixBlockRows::Fixed(3),
        None,
    )
    .unwrap();
    // The input's declared lower bounds are not a row value. This witness
    // resolves from its current captured shape only when the artifact runs.
    build(
        fixed(2, 1),
        result(scalar.clone(), MatrixBlockRows::InferOrZero),
        MatrixBlockRows::InputMatrixElements { input: 1 },
        Some(matrix(
            scalar,
            [axis(0), axis(1)],
            vec![
                parameter(0, DimensionLifetime::Turn, 7),
                parameter(1, DimensionLifetime::Turn, 11),
            ],
        )),
    )
    .unwrap();
}

#[test]
fn declared_constant_cardinality_is_schema_authority_for_elements_and_blocks() {
    let scalar = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::FloatingPoint(FloatWidth::W64),
    }
    .finalize()
    .unwrap();
    for (kind, yielded, height) in [
        (ComprehensionKind::Matrix, scalar, 1),
        (
            ComprehensionKind::MatrixHorizontalBlocks {
                rows: MatrixBlockRows::Fixed(2),
            },
            fixed(2, 1),
            2,
        ),
    ] {
        let mut revisions = Vec::new();
        for width in [0, 2] {
            let artifact = build_kind(yielded.clone(), fixed(height, width), kind, None).unwrap();
            let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
            let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();
            assert_eq!(
                bytes,
                encode_program_artifact_bytecode_v1(&decoded).unwrap()
            );
            assert_eq!(artifact.revision(), decoded.revision());
            assert!(
                matches!(&decoded.nodes()[0].body, ExecutableNodeBody::Comprehension(control) if control.kind == kind)
            );
            revisions.push(artifact.revision());
        }
        assert_ne!(revisions[0], revisions[1]);
        invalid(build_kind(yielded, fixed(height + 1, 2), kind, None));
    }
}

#[test]
fn block_policy_rejects_invalid_input_geometry_leaf_and_output_axes() {
    let scalar = SchemaBody::FloatingPoint(FloatWidth::W64);
    let inferred = MatrixBlockRows::InferOrZero;
    for input in [1, u16::MAX] {
        invalid(build(
            fixed(2, 1),
            result(scalar.clone(), inferred),
            MatrixBlockRows::InputMatrixElements { input },
            None,
        ));
    }
    let scalar_schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: scalar.clone(),
    }
    .finalize()
    .unwrap();
    invalid(build(
        fixed(2, 1),
        result(scalar.clone(), inferred),
        MatrixBlockRows::InputMatrixElements { input: 1 },
        Some(scalar_schema.clone()),
    ));
    invalid(build(
        scalar_schema,
        result(scalar.clone(), inferred),
        inferred,
        None,
    ));
    let rank3 = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Matrix {
            element: Box::new(scalar.clone()),
            dimensions: Box::new([
                DimensionExpr::Constant(1),
                DimensionExpr::Constant(1),
                DimensionExpr::Constant(1),
            ]),
        },
    }
    .finalize()
    .unwrap();
    invalid(build(
        fixed(2, 1),
        result(scalar.clone(), inferred),
        MatrixBlockRows::InputMatrixElements { input: 1 },
        Some(rank3),
    ));
    invalid(build(
        fixed(2, 1),
        result(SchemaBody::String, inferred),
        inferred,
        None,
    ));
    // Both variable axes belong to this invocation and may be zero. Neither an
    // activation seed nor a positive lower bound is a valid empty-row witness.
    for parameters in [
        vec![
            parameter(0, DimensionLifetime::Activation, 0),
            parameter(1, DimensionLifetime::Turn, 0),
        ],
        vec![
            parameter(0, DimensionLifetime::Turn, 1),
            parameter(1, DimensionLifetime::Turn, 0),
        ],
    ] {
        invalid(build(
            fixed(2, 1),
            matrix(scalar.clone(), [axis(0), axis(1)], parameters),
            inferred,
            None,
        ));
    }
    invalid(build(
        fixed(2, 1),
        matrix(
            scalar,
            [axis(0), axis(0)],
            vec![parameter(0, DimensionLifetime::Turn, 0)],
        ),
        inferred,
        None,
    ));
}

#[test]
fn blocks_flatten_one_layer_and_remap_only_the_leaf_dimension_environment() {
    let scalar = SchemaBody::FloatingPoint(FloatWidth::W64);
    let nested = SchemaBody::Matrix {
        element: Box::new(scalar),
        dimensions: Box::new([axis(1), DimensionExpr::Constant(2)]),
    };
    let yielded = matrix(
        nested,
        [axis(0), axis(2)],
        vec![
            parameter(0, DimensionLifetime::Turn, 3),
            parameter(1, DimensionLifetime::Activation, 1),
            parameter(2, DimensionLifetime::Turn, 4),
        ],
    );
    let leaf = SchemaBody::Matrix {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        dimensions: Box::new([axis(0), DimensionExpr::Constant(2)]),
    };
    let output = matrix(
        leaf.clone(),
        [axis(1), axis(2)],
        vec![
            parameter(0, DimensionLifetime::Activation, 1),
            parameter(1, DimensionLifetime::Turn, 0),
            parameter(2, DimensionLifetime::Turn, 0),
        ],
    );
    let artifact = build(yielded.clone(), output, MatrixBlockRows::InferOrZero, None).unwrap();
    let decoded = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let schema = decoded.schemas().get(decoded.outputs()[0].schema).unwrap();
    let SchemaBody::Matrix { element, .. } = schema.body() else {
        panic!()
    };
    assert_eq!(element.as_ref(), &leaf);
    assert_eq!(schema.dimension_parameters().len(), 3);
    // A second flattening or retaining the block's unrelated positive row/
    // column bounds is a different schema, even when a seed shape happens to fit.
    invalid(build(
        yielded,
        result(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            MatrixBlockRows::InferOrZero,
        ),
        MatrixBlockRows::InferOrZero,
        None,
    ));
}

#[test]
fn bytecode_rejects_missing_or_legacy_attached_block_policy() {
    let artifact = build(
        fixed(2, 1),
        result(
            SchemaBody::FloatingPoint(FloatWidth::W64),
            MatrixBlockRows::InferOrZero,
        ),
        MatrixBlockRows::InferOrZero,
        None,
    )
    .unwrap();
    let sections = encode_program_artifact_sections(&artifact).unwrap();
    for kind in 0..=3 {
        let mut altered = sections.clone();
        let mut graph: serde_json::Value = serde_json::from_slice(&altered.nodes).unwrap();
        let declaration = &mut graph["nodes"][0]["body"]["Comprehension"];
        declaration["kind"] = kind.into();
        if kind == 3 {
            declaration.as_object_mut().unwrap().remove("rows");
        }
        altered.nodes = serde_json::to_vec(&graph).unwrap();
        assert!(matches!(
            decode_program_artifact_sections(&altered),
            Err(ArtifactBytecodeError::InvalidWireTag {
                section: "comprehension kind",
                ..
            })
        ));
    }
}
