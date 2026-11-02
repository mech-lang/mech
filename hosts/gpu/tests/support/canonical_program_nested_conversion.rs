//! Actual ordinary annotations, original/decoded artifacts, resident and JIT.
use super::{equal, limits, reference, reference_result};
use mech_core::{
    CanonicalNominalPath, DimensionExpr, ManagedMemoryBudget, SchemaBody, Value,
    ValueDataDraft as Data, shape_for_schema_components,
    snapshot::{EnumDraft, F64Bits, OptionDraft, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

#[path = "nested_set_conversion_cases/budget.rs"]
mod budget;

#[path = "nested_set_conversion_cases/binding.rs"]
mod binding;

fn variants(source: &str) -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x534554), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let product = CanonicalSourceFrontend
        .with_nominal_origin(
            CanonicalNominalPath::new(vec!["tests".into(), "nested-set".into()]).unwrap(),
        )
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap();
    let original = product.compile_artifact().unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

fn matrix(values: impl IntoIterator<Item = Data>) -> Data {
    Data::Matrix(values.into_iter().collect())
}
fn strings(values: &[&str]) -> Data {
    matrix(values.iter().map(|value| Data::String((*value).into())))
}
fn set(values: impl IntoIterator<Item = Data>) -> Data {
    Data::Set(values.into_iter().collect())
}
fn option(value: Option<Data>) -> Data {
    Data::Option(OptionDraft {
        present: value.is_some(),
        value: value.map(Box::new),
    })
}

// Test geometry is explicit: constant axes remain unchanged; a free matrix
// fixture is a 1 x N row. The maintained semantic component solver binds the
// actual schema parameters (including differently numbered nested axes).
fn fixture_body(body: &SchemaBody, data: Option<&Data>) -> SchemaBody {
    match body {
        SchemaBody::Option(inner) => {
            let data = match data {
                Some(Data::Option(v)) => v.value.as_deref(),
                _ => None,
            };
            SchemaBody::Option(Box::new(fixture_body(inner, data)))
        }
        SchemaBody::Matrix {
            element,
            dimensions,
        } => {
            let values = match data {
                Some(Data::Matrix(values)) => Some(values.as_ref()),
                _ => None,
            };
            let count = values.map_or(0, |values| values.len() as u64);
            let extents = [if values.is_some() { 1 } else { 0 }, count];
            SchemaBody::Matrix {
                element: Box::new(fixture_body(element, values.and_then(|v| v.first()))),
                dimensions: dimensions
                    .iter()
                    .enumerate()
                    .map(|(axis, dimension)| match dimension {
                        DimensionExpr::Constant(_) => dimension.clone(),
                        _ => DimensionExpr::Constant(extents[axis]),
                    })
                    .collect(),
            }
        }
        _ => body.clone(),
    }
}
fn args(artifact: &ProgramArtifact, data: &Data, memory: &ManagedMemoryBudget) -> Vec<Value> {
    let schema = artifact.schemas().get(artifact.inputs()[0].schema).unwrap();
    args_with_body(
        artifact,
        data,
        fixture_body(schema.body(), Some(data)),
        memory,
    )
}
fn args_with_body(
    artifact: &ProgramArtifact,
    data: &Data,
    body: SchemaBody,
    memory: &ManagedMemoryBudget,
) -> Vec<Value> {
    assert_eq!(artifact.inputs().len(), 1);
    let input = &artifact.inputs()[0];
    let schema = artifact.schemas().get(input.schema).unwrap();
    let shape = shape_for_schema_components(schema, &[(schema.body(), body)], None).unwrap();
    let value = ValueDraft {
        schema: input.schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data: data.clone(),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap();
    let mut reservation = memory
        .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
        .unwrap();
    vec![value.into_memory_budget(&mut reservation).unwrap()]
}

#[test]
fn matrix_key_current_axes_change_without_reusing_old_output_geometry() {
    for artifact in variants("unique<{[u8]}> := values<[[u8]]>\nunique\n") {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let data = matrix([matrix([Data::U8(1), Data::U8(2), Data::U8(3), Data::U8(4)])]);
        let mut output = vec![None; artifact.outputs().len()];
        let mut shapes = Vec::new();
        for axes in [[1, 4], [2, 2], [4, 1]] {
            let schema = artifact.schemas().get(artifact.inputs()[0].schema).unwrap();
            let mut body = fixture_body(schema.body(), Some(&data));
            let SchemaBody::Matrix { element, .. } = &mut body else {
                panic!("matrix input")
            };
            let SchemaBody::Matrix { dimensions, .. } = &mut **element else {
                panic!("matrix keys")
            };
            *dimensions = axes.map(DimensionExpr::Constant).into();
            let input = args_with_body(&artifact, &data, body, &memory);
            let ordinary = reference(&artifact, &input);
            jit.evaluate_into(&input, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &ordinary);
            shapes.push(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .shape()
                    .parameter_values()
                    .to_vec(),
            );
        }
        assert_ne!(shapes[0], shapes[1]);
        assert_ne!(shapes[1], shapes[2]);
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

fn exercise(source: &str, cases: &[(Data, Data)]) {
    for artifact in variants(source) {
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(jit.descriptor().plan().blocks().iter().filter(|block| matches!(
            block.action(), mech_engine::native_artifact::NativeIterationAction::LibraryCall { template, .. }
                if template.primitive() == mech_engine::native_artifact::NativeCollectionPrimitive::MatrixToSetConversion
        )).count(), 1);
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(!object.bytes().is_empty());
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let mut output = vec![None; artifact.outputs().len()];
        for (input, expected) in cases {
            let input = args(&artifact, input, &memory);
            let ordinary = reference(&artifact, &input);
            assert_eq!(
                ordinary.last().unwrap().canonical_data_draft().unwrap(),
                *expected
            );
            let report = jit
                .evaluate_into(&input, &mut output, limits(), &memory)
                .unwrap();
            assert!(report.visits > 0);
            equal(&artifact, &output, &ordinary);
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                *expected
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn nested_option_payload_present_empty_and_unicode_geometry_are_live() {
    exercise(
        "unique<{string}?> := values<[string]?>\nunique\n",
        &[
            (option(None), option(None)),
            (option(Some(strings(&[]))), option(Some(set([])))),
            (
                option(Some(strings(&["é", "é", "é"]))),
                option(Some(set([
                    Data::String("é".into()),
                    Data::String("é".into()),
                ]))),
            ),
            (
                option(Some(strings(&["z"]))),
                option(Some(set([Data::String("z".into())]))),
            ),
        ],
    );
    // An already-optional input selects one recursive convert/set. A bare
    // Matrix source would instead add an independent option/some node.
    let wrapped = "<maybe> := <{string}?>\nunique<maybe?> := values<[string]?>\nunique\n";
    for artifact in variants(wrapped) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let template = jit.descriptor().plan().blocks().iter().find_map(|block| {
            match block.action() {
                mech_engine::native_artifact::NativeIterationAction::LibraryCall { template, .. }
                    if template.primitive() == mech_engine::native_artifact::NativeCollectionPrimitive::MatrixToSetConversion => Some(template),
                _ => None,
            }
        }).unwrap();
        let resolve = |schema: &mech_core::Schema| {
            mech_core::ResolvedType::from_schema(
                schema,
                &mech_core::shape_for_declared_lower_bounds(schema).unwrap(),
            )
            .unwrap()
        };
        let conversion = mech_core::plan_explicit_cast(
            &resolve(&template.inputs()[0]),
            &resolve(template.output()),
        )
        .unwrap();
        assert_eq!(
            mech_core::conversion_option_present_depth(&conversion.step),
            1
        );
        let mech_core::ConversionStep::OptionPayload(payload) = conversion.step else {
            panic!("actual optional source")
        };
        let mech_core::ConversionStep::OptionPresent(inner) = payload.step else {
            panic!("one actual new wrapper")
        };
        assert!(matches!(
            inner.step,
            mech_core::ConversionStep::MatrixToSet(_)
        ));
    }
    exercise(
        wrapped,
        &[
            (
                option(Some(strings(&["b", "a", "b"]))),
                option(Some(option(Some(set([
                    Data::String("a".into()),
                    Data::String("b".into()),
                ]))))),
            ),
            (
                option(Some(strings(&[]))),
                option(Some(option(Some(set([]))))),
            ),
            (option(None), option(None)),
        ],
    );
}

#[test]
fn nested_matrix_elements_and_set_valued_keys_use_actual_cardinality() {
    let input = matrix([strings(&["b", "a"]), strings(&["b", "a"])]);
    let key = set([Data::String("a".into()), Data::String("b".into())]);
    // MatrixElements keeps the maintained planner's equal outer axes and
    // parameter-environment rule. Other tests retain free source/key axes.
    exercise(
        "unique<[{string}:2]:1,2> := values<[[string]:1,2]:1,2>\nunique\n",
        &[(input.clone(), matrix([key.clone(), key.clone()]))],
    );
    exercise(
        "unique<{{string}}> := values<[[string]]>\nunique\n",
        &[(input, set([key]))],
    );
}

#[test]
fn declared_nominal_enum_and_matrix_keys_keep_identity_and_parameter_axes() {
    let event = |ordinal, text: Option<&str>| {
        Data::Enum(EnumDraft {
            ordinal,
            payload: text.map(|text| Box::new(Data::String(text.into()))),
        })
    };
    exercise(
        "<event> := :idle | :word<string>\nunique<{event}> := values<[event]>\nunique\n",
        &[(
            matrix([event(1, Some("é")), event(0, None), event(1, Some("é"))]),
            set([event(0, None), event(1, Some("é"))]),
        )],
    );
    let a = matrix([Data::U8(1), Data::U8(2), Data::U8(3), Data::U8(4)]);
    let b = matrix([Data::U8(4), Data::U8(3), Data::U8(2), Data::U8(1)]);
    for source in [
        "unique<{[u8]}> := values<[[u8]]>\nunique\n",
        "unique<{[u8]:2,2}> := values<[[u8]:2,2]>\nunique\n",
    ] {
        exercise(
            source,
            &[(
                matrix([b.clone(), a.clone(), a.clone()]),
                set([a.clone(), b.clone()]),
            )],
        );
    }
}
