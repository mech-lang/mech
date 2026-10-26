//! Retained comprehension encodings preserve their bytes and program revisions.
#![cfg(feature = "compiler")]

use mech_core::{
    CardinalitySpec, ConstantStoreBuilder, DimensionExpr, DimensionLifetime,
    DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin, FloatWidth,
    Schema, SchemaBody, SchemaDraft, SchemaTableBuilder, ValueDraft,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDataDraft},
};
use mech_engine::*;
use sha2::{Digest, Sha256};

fn schema(body: SchemaBody, dynamic: bool) -> Schema {
    SchemaDraft {
        body,
        dimension_parameters: if dynamic {
            vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            }]
            .into_boxed_slice()
        } else {
            Box::new([])
        },
    }
    .finalize()
    .unwrap()
}

fn matrix(element: SchemaBody, rows: DimensionExpr, columns: DimensionExpr) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: Box::new([rows, columns]),
    }
}

fn fixture(tag: u8, nested: bool) -> ProgramArtifact {
    let scalar = SchemaBody::FloatingPoint(FloatWidth::W64);
    let block = matrix(
        scalar.clone(),
        DimensionExpr::Constant(2),
        DimensionExpr::Constant(1),
    );
    let mut schemas = SchemaTableBuilder::new();
    let scalar_handle = schemas.insert(schema(scalar.clone(), false)).unwrap();
    let block_handle = schemas.insert(schema(block.clone(), false)).unwrap();
    let output_body = match tag {
        0 => matrix(
            if nested { block } else { scalar.clone() },
            DimensionExpr::Constant(1),
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
        ),
        1 => SchemaBody::Set {
            element: Box::new(scalar.clone()),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        },
        2 => matrix(
            scalar,
            DimensionExpr::Constant(2),
            DimensionExpr::Constant(1),
        ),
        _ => unreachable!(),
    };
    let output_handle = schemas.insert(schema(output_body, tag == 0)).unwrap();
    let build = schemas.finish().unwrap();
    let scalar = build.resolve(scalar_handle).unwrap();
    let block = build.resolve(block_handle).unwrap();
    let output = build.resolve(output_handle).unwrap();
    let (schemas, _) = build.into_parts();
    let mut constants = ConstantStoreBuilder::new(&schemas);
    let scalar_value = constants
        .insert(
            ValueDraft {
                schema: scalar,
                shape_values: Box::new([]),
                data: ValueDataDraft::F64(F64Bits::from_f64(3.0)),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap(),
        )
        .unwrap();
    let block_value = constants
        .insert(
            ValueDraft {
                schema: block,
                shape_values: Box::new([]),
                data: ValueDataDraft::Matrix(
                    vec![
                        ValueDataDraft::F64(F64Bits::from_f64(3.0)),
                        ValueDataDraft::F64(F64Bits::from_f64(7.0)),
                    ]
                    .into_boxed_slice(),
                ),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .unwrap(),
        )
        .unwrap();
    let build = constants.finish().unwrap();
    let scalar_value = build.resolve(scalar_value).unwrap();
    let block_value = build.resolve(block_value).unwrap();
    let (constants, _) = build.into_parts();
    let declaration = ComprehensionDeclaration {
        id: ControlBlockId(0),
        kind: match tag {
            0 => ComprehensionKind::Matrix,
            1 => ComprehensionKind::Set,
            2 => ComprehensionKind::MatrixPreserveShape,
            _ => unreachable!(),
        },
        steps: if tag == 2 {
            vec![ComprehensionStep::Generator {
                source: ComprehensionValue::Constant(block_value),
                pattern: CollectionPattern::Bind {
                    local: 0,
                    schema: scalar,
                },
            }]
            .into_boxed_slice()
        } else {
            Box::new([])
        },
        yield_value: if tag == 2 {
            ComprehensionValue::Local(0)
        } else {
            ComprehensionValue::Constant(if nested { block_value } else { scalar_value })
        },
    };
    let source = SourceProgram {
        nodes: vec![SourceNode {
            body: SourceNodeBody::Comprehension(declaration),
            inputs: Box::new([]),
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
        &source,
        &mut ArtifactBuildContext::new(&schemas, &constants),
        &[None],
    )
    .unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
#[ignore = "explicit capture/verification directory required"]
fn capture_or_verify_legacy_comprehension_bytes_and_revisions() {
    let capture = std::env::var_os("MECH_MATRIX_BLOCK_LEGACY_CAPTURE");
    let verify = std::env::var_os("MECH_MATRIX_BLOCK_LEGACY_VERIFY");
    assert!(
        capture.is_some() ^ verify.is_some(),
        "choose exactly one mode"
    );
    let directory = std::path::PathBuf::from(capture.as_ref().or(verify.as_ref()).unwrap());
    qualify_legacy(&directory, capture.is_some());
}

#[test]
fn retained_legacy_comprehension_bytes_and_revisions_remain_stable() {
    let directory = std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/comprehension-legacy-v1"
    ));
    qualify_legacy(directory, false);
}

fn qualify_legacy(directory: &std::path::Path, capture: bool) {
    if capture {
        assert!(!directory.exists(), "capture must use a fresh directory");
        std::fs::create_dir_all(&directory).unwrap();
    }
    let mut records = Vec::new();
    for (name, tag, nested) in [
        ("matrix-scalar", 0, false),
        ("matrix-nested", 0, true),
        ("set-scalar", 1, false),
        ("matrix-preserve-shape", 2, false),
    ] {
        let artifact = fixture(tag, nested);
        let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert_eq!(decoded.revision(), artifact.revision());
        assert_eq!(
            encode_program_artifact_bytecode_v1(&decoded).unwrap(),
            bytes
        );
        assert_eq!(decoded.nodes(), artifact.nodes());
        let sections = encode_program_artifact_sections(&artifact).unwrap();
        let graph: serde_json::Value = serde_json::from_slice(&sections.nodes).unwrap();
        assert_eq!(graph["nodes"][0]["body"]["Comprehension"]["kind"], tag);
        assert!(
            graph["nodes"][0]["body"]["Comprehension"]
                .get("rows")
                .is_none()
        );
        let file = format!("{name}.mecb");
        if capture {
            std::fs::write(directory.join(&file), &bytes).unwrap();
        } else {
            assert_eq!(
                std::fs::read(directory.join(&file)).unwrap(),
                bytes,
                "{name}"
            );
        }
        records.push(serde_json::json!({
            "file": file, "kind_tag": tag, "nested_element": nested,
            "bytes": bytes.len(), "sha256": hex(&Sha256::digest(&bytes)),
            "program_revision": hex(artifact.revision().as_bytes()),
        }));
    }
    let manifest = serde_json::json!({
        "schema": "mech.matrix-block-legacy-fixtures.v1",
        "origin": "ordinary-source-program-compiler",
        "fixtures": records,
    });
    if capture {
        std::fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
    } else {
        let retained: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(retained, manifest);
    }
}
