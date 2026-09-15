//! Exact schema/publication regressions for canonical constant binding (R03/G25).
//!
//! These tests use canonical Value snapshots directly. RuntimeHostInputValue is
//! deliberately not involved: its admitted input and returned-default subsets
//! are separate product contracts. Tests are strict, and each case is a separate
//! Rust test so a failing binding does not hide the remaining cases.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use std::collections::BTreeMap;
use std::sync::Arc;

use mech_core::snapshot::{
    Complex32Bits, Complex64Bits, F32Bits, F64Bits, MapEntryDraft, NamedValueDraft, OptionDraft,
    SnapshotValidationContext, TableColumnDraft,
};
use mech_core::{
    CanonicalNominalPath, CardinalitySpec, DimensionExpr, FloatWidth, IntegerWidth, NominalKey,
    NominalKind, ReactiveInstanceId, SchemaBody, SchemaDraft, SchemaField, SchemaTableBuilder,
    Value, ValueData, ValueDataDraft as D, ValueDraft,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram, ProgramArtifact};
use mech_runtime::SourceDocument;
use mech_syntax::document::{ParseConfig, Revision};

struct Case {
    id: &'static str,
    annotation: Option<&'static str>,
    schema: SchemaBody,
    values: [Value; 2],
    incompatible: Option<Value>,
}

fn snapshot(body: SchemaBody, data: D) -> Value {
    let mut builder = SchemaTableBuilder::new();
    // An unrelated schema makes this an independently owned schema table,
    // not an artifact-local schema index masquerading as a detached value.
    builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Id,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let handle = builder
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = builder.finish().unwrap();
    let schema = built.resolve(handle).unwrap();
    let (schemas, _) = built.into_parts();
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .expect("audit fixture must be a valid independent canonical snapshot")
}

fn canonical(case: &Case, catalog: Arc<mech_core::FunctionCatalog>) -> CanonicalSourceProgram {
    let signal = case
        .annotation
        .map(|kind| format!("signal<{kind}>"))
        .unwrap_or_else(|| "signal".to_owned());
    let source = format!("answer := {signal}\nanswer\n");
    let document = SourceDocument::parse_resolved(
        &format!("audit:schema/{}", case.id),
        Revision(7),
        source.as_str(),
        ParseConfig::default(),
    )
    .unwrap();
    document.index().expect("audit source is strictly indexed");
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog_and_input_schemas(
            &document.document(),
            catalog,
            BTreeMap::from([("signal".to_owned(), case.schema.clone())]),
        )
        .unwrap_or_else(|error| panic!("{}: frontend: {error:?}", case.id));
    assert_eq!(
        program.program().inputs.len(),
        1,
        "{}: one source input",
        case.id
    );
    assert_eq!(program.program().inputs[0].name, "signal");
    let declared = program
        .schemas()
        .get(program.program().inputs[0].schema)
        .unwrap();
    assert_eq!(
        declared.body(),
        &case.schema,
        "{}: exact source input schema",
        case.id
    );
    program
}

fn assert_exact(actual: &Value, expected: &Value, label: &str) {
    assert_eq!(
        actual.schema_key(),
        expected.schema_key(),
        "{label}: exact schema key"
    );
    let actual_schemas = actual
        .schemas()
        .expect("published value retains schema owner");
    let expected_schemas = expected
        .schemas()
        .expect("reference fixture retains schema owner");
    assert_eq!(
        actual_schemas
            .get(actual.schema())
            .unwrap()
            .closed_body(actual.shape())
            .unwrap(),
        expected_schemas
            .get(expected.schema())
            .unwrap()
            .closed_body(expected.shape())
            .unwrap(),
        "{label}: exact closed schema body",
    );
    assert_eq!(
        actual.shape().parameter_values(),
        expected.shape().parameter_values(),
        "{label}: shape values"
    );
    // Expected is constructed from the fixture's exact ValueDataDraft, never
    // copied from either execution route. Hash identity includes nested dynamic
    // schema identities, unlike language equality between promoted numbers.
    assert_eq!(
        actual.value_hash(&actual_schemas).unwrap(),
        expected.value_hash(&expected_schemas).unwrap(),
        "{label}: exact canonical value identity",
    );
    match (actual.data(), expected.data()) {
        (ValueData::Dynamic(actual), ValueData::Dynamic(expected)) => {
            let actual = actual.value().unwrap();
            let expected = expected.value().unwrap();
            assert_exact(&actual, &expected, &format!("{label}: dynamic payload"));
        }
        _ => assert_eq!(
            actual.canonical_data_draft().unwrap(),
            expected.canonical_data_draft().unwrap(),
            "{label}: exact data tags and values"
        ),
    }
}

fn decoded(artifact: &ProgramArtifact) -> ProgramArtifact {
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(artifact).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap(),
        bytes
    );
    decoded
}

fn check(case: Case) {
    eprintln!("CONSTANT_BINDING_START {}", case.id);
    let catalog = mech_stdlib::source_catalog();
    let direct = canonical(&case, Arc::clone(&catalog))
        .compile_artifact()
        .unwrap();
    let bytecode = decoded(&direct);
    for (route, artifact) in [("source", &direct), ("bytecode", &bytecode)] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{}: {route} live activation: {error:?}", case.id));
        assert_eq!(instance.plan.inputs.len(), 1);
        let declared = instance.plan.inputs[0].clone();
        for (turn, expected) in case.values.iter().enumerate() {
            // Raw CapturedSignalInput is a physical arena-lane API. Detached
            // canonical Values enter through the schema-checked value API,
            // after the documented cross-table Value::rebind operation.
            // Extending first retains foreign Dynamic payload schemas too.
            let schemas = artifact
                .schemas()
                .extend_preserving_ids(&expected.schemas().unwrap())
                .unwrap();
            let admitted = expected
                .rebind(declared.schema, &declared.shape, &schemas)
                .unwrap_or_else(|error| panic!("{}: {route} rebind: {error:?}", case.id));
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: declared.slot,
                    value: &admitted,
                }])
                .and_then(|prepared| prepared.publish())
                .unwrap_or_else(|error| panic!("{}: {route} live turn{turn}: {error:?}", case.id));
            assert_exact(
                &instance.copied_output(0).unwrap(),
                expected,
                &format!("{}:{route}:turn{turn}", case.id),
            );
        }
        eprintln!("CONSTANT_BINDING_PHASE {} {route}-live-pass", case.id);
    }
    let bound = canonical(&case, Arc::clone(&catalog))
        .bind_input_constants(&[(0, case.values[0].clone())])
        .unwrap_or_else(|error| panic!("{}: bind_input_constants: {error:?}", case.id));
    assert!(
        bound.program().inputs.is_empty(),
        "{}: bound input removed",
        case.id
    );
    let bound_direct = bound.compile_artifact().unwrap();
    let check_bound_publication = |route: &str, artifact: &ProgramArtifact| {
        assert!(artifact.inputs().is_empty());
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 1),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap_or_else(|error| panic!("{}: {route} activation: {error:?}", case.id));
        for turn in 0..2 {
            instance
                .turn(&[])
                .unwrap_or_else(|error| panic!("{}: {route} turn{turn}: {error:?}", case.id));
            assert_exact(
                &instance.copied_output(0).unwrap(),
                &case.values[0],
                &format!("{}:{route}:turn{turn}", case.id),
            );
        }
        eprintln!("CONSTANT_BINDING_PHASE {} {route}-pass", case.id);
    };
    // Record direct bound execution before decoding: a codec rejection must
    // not hide whether the same constant-bound artifact executes correctly.
    check_bound_publication("bound-source", &bound_direct);
    let bound_bytecode = decoded(&bound_direct);
    check_bound_publication("bound-bytecode", &bound_bytecode);
    eprintln!("CONSTANT_BINDING_PASS {}", case.id);
}

fn case(
    id: &'static str,
    annotation: Option<&'static str>,
    schema: SchemaBody,
    values: [D; 2],
) -> Case {
    let incompatible = if schema == SchemaBody::String {
        snapshot(SchemaBody::Bool, D::Bool(true))
    } else {
        snapshot(SchemaBody::String, D::String("wrong-type".into()))
    };
    Case {
        id,
        annotation,
        values: values.map(|value| snapshot(schema.clone(), value)),
        schema,
        incompatible: Some(incompatible),
    }
}

#[test]
fn exact_scalar_u8() {
    check(case(
        concat!("QT-", "u8"),
        Some("u8"),
        SchemaBody::UnsignedInteger(IntegerWidth::W8),
        [D::U8(1), D::U8(2)],
    ));
}
#[test]
fn exact_scalar_u16() {
    check(case(
        concat!("QT-", "u16"),
        Some("u16"),
        SchemaBody::UnsignedInteger(IntegerWidth::W16),
        [D::U16(1), D::U16(2)],
    ));
}
#[test]
fn exact_scalar_u32() {
    check(case(
        concat!("QT-", "u32"),
        Some("u32"),
        SchemaBody::UnsignedInteger(IntegerWidth::W32),
        [D::U32(1), D::U32(2)],
    ));
}
#[test]
fn exact_scalar_u64() {
    check(case(
        concat!("QT-", "u64"),
        Some("u64"),
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        [D::U64(1), D::U64(2)],
    ));
}
#[test]
fn exact_scalar_u128() {
    check(case(
        concat!("QT-", "u128"),
        Some("u128"),
        SchemaBody::UnsignedInteger(IntegerWidth::W128),
        [D::U128(1), D::U128(2)],
    ));
}
#[test]
fn exact_scalar_i8() {
    check(case(
        concat!("QT-", "i8"),
        Some("i8"),
        SchemaBody::SignedInteger(IntegerWidth::W8),
        [D::I8(1), D::I8(2)],
    ));
}
#[test]
fn exact_scalar_i16() {
    check(case(
        concat!("QT-", "i16"),
        Some("i16"),
        SchemaBody::SignedInteger(IntegerWidth::W16),
        [D::I16(1), D::I16(2)],
    ));
}
#[test]
fn exact_scalar_i32() {
    check(case(
        concat!("QT-", "i32"),
        Some("i32"),
        SchemaBody::SignedInteger(IntegerWidth::W32),
        [D::I32(1), D::I32(2)],
    ));
}
#[test]
fn exact_scalar_i64() {
    check(case(
        concat!("QT-", "i64"),
        Some("i64"),
        SchemaBody::SignedInteger(IntegerWidth::W64),
        [D::I64(1), D::I64(2)],
    ));
}
#[test]
fn exact_scalar_i128() {
    check(case(
        concat!("QT-", "i128"),
        Some("i128"),
        SchemaBody::SignedInteger(IntegerWidth::W128),
        [D::I128(1), D::I128(2)],
    ));
}
#[test]
fn exact_scalar_f32() {
    check(case(
        concat!("QT-", "f32"),
        Some("f32"),
        SchemaBody::FloatingPoint(FloatWidth::W32),
        [
            D::F32(F32Bits::from_f32(1.25)),
            D::F32(F32Bits::from_f32(2.5)),
        ],
    ));
}
#[test]
fn exact_scalar_f64() {
    check(case(
        concat!("QT-", "f64"),
        Some("f64"),
        SchemaBody::FloatingPoint(FloatWidth::W64),
        [
            D::F64(F64Bits::from_f64(1.25)),
            D::F64(F64Bits::from_f64(2.5)),
        ],
    ));
}
#[test]
fn exact_scalar_c32() {
    check(case(
        concat!("QT-", "c32"),
        Some("c32"),
        SchemaBody::Complex(FloatWidth::W32),
        [
            D::Complex32(Complex32Bits::new(
                F32Bits::from_f32(1.0),
                F32Bits::from_f32(2.0),
            )),
            D::Complex32(Complex32Bits::new(
                F32Bits::from_f32(3.0),
                F32Bits::from_f32(4.0),
            )),
        ],
    ));
}
#[test]
fn exact_scalar_c64() {
    check(case(
        concat!("QT-", "c64"),
        Some("c64"),
        SchemaBody::Complex(FloatWidth::W64),
        [
            D::Complex64(Complex64Bits::new(
                F64Bits::from_f64(1.0),
                F64Bits::from_f64(2.0),
            )),
            D::Complex64(Complex64Bits::new(
                F64Bits::from_f64(3.0),
                F64Bits::from_f64(4.0),
            )),
        ],
    ));
}
#[test]
fn exact_scalar_r64() {
    check(case(
        concat!("QT-", "r64"),
        Some("r64"),
        SchemaBody::Rational64,
        [
            D::Rational64 {
                numerator: 1,
                denominator: 2,
            },
            D::Rational64 {
                numerator: 2,
                denominator: 3,
            },
        ],
    ));
}
#[test]
fn exact_scalar_bool() {
    check(case(
        concat!("QT-", "bool"),
        Some("bool"),
        SchemaBody::Bool,
        [D::Bool(false), D::Bool(true)],
    ));
}
#[test]
fn exact_scalar_string() {
    check(case(
        concat!("QT-", "string"),
        Some("string"),
        SchemaBody::String,
        [D::String(String::new()), D::String("Δ".into())],
    ));
}

fn atom_schema(name: &str) -> SchemaBody {
    let path = CanonicalNominalPath::new(vec![name.to_owned()]).unwrap();
    SchemaBody::Atom(NominalKey::from_path(NominalKind::Atom, &path))
}

fn atom_case() -> Case {
    let mut fixture = case("QG-Atom", None, atom_schema("ready"), [D::Atom, D::Atom]);
    fixture.incompatible = Some(snapshot(atom_schema("other"), D::Atom));
    fixture
}

#[test]
fn exact_generic_atom() {
    check(atom_case());
}

fn optional(value: Option<D>) -> D {
    D::Option(OptionDraft {
        present: value.is_some(),
        value: value.map(Box::new),
    })
}

#[test]
fn exact_generic_option() {
    check(case(
        "QG-Option",
        None,
        SchemaBody::Option(Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8))),
        [optional(Some(D::U8(1))), optional(None)],
    ));
}

#[test]
fn exact_generic_tuple() {
    check(case(
        "QG-Tuple",
        None,
        SchemaBody::Tuple(
            vec![
                SchemaBody::UnsignedInteger(IntegerWidth::W8),
                SchemaBody::Bool,
            ]
            .into_boxed_slice(),
        ),
        [
            D::Tuple(vec![D::U8(1), D::Bool(true)].into_boxed_slice()),
            D::Tuple(vec![D::U8(2), D::Bool(false)].into_boxed_slice()),
        ],
    ));
}

fn fields() -> Box<[SchemaField]> {
    vec![
        SchemaField {
            name: "x".into(),
            schema: SchemaBody::UnsignedInteger(IntegerWidth::W8),
        },
        SchemaField {
            name: "ready".into(),
            schema: SchemaBody::Bool,
        },
    ]
    .into_boxed_slice()
}

#[test]
fn exact_generic_record() {
    let record = |number, ready| {
        D::Record(
            vec![
                NamedValueDraft {
                    name: "x".into(),
                    value: D::U8(number),
                },
                NamedValueDraft {
                    name: "ready".into(),
                    value: D::Bool(ready),
                },
            ]
            .into_boxed_slice(),
        )
    };
    check(case(
        "QG-Record",
        None,
        SchemaBody::Record(fields()),
        [record(1, true), record(2, false)],
    ));
}

fn matrix_schema(rows: u64, columns: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    }
}

fn matrix_case() -> Case {
    let mut fixture = case(
        "QG-Matrix",
        None,
        matrix_schema(2, 3),
        [
            D::Matrix((1..=6).map(D::U8).collect()),
            D::Matrix((7..=12).map(D::U8).collect()),
        ],
    );
    fixture.incompatible = Some(snapshot(
        matrix_schema(3, 2),
        D::Matrix((1..=6).map(D::U8).collect()),
    ));
    fixture
}

#[test]
fn exact_generic_matrix() {
    check(matrix_case());
}

#[test]
fn exact_generic_table() {
    let table = |numbers: [u8; 2], ready: [bool; 2]| {
        D::Table(
            vec![
                TableColumnDraft {
                    name: "x".into(),
                    values: numbers.into_iter().map(D::U8).collect(),
                },
                TableColumnDraft {
                    name: "ready".into(),
                    values: ready.into_iter().map(D::Bool).collect(),
                },
            ]
            .into_boxed_slice(),
        )
    };
    check(case(
        "QG-Table",
        None,
        SchemaBody::Table {
            columns: fields(),
            rows: DimensionExpr::Constant(2).into(),
        },
        [table([1, 2], [true, false]), table([3, 4], [false, true])],
    ));
}

#[test]
fn exact_generic_set() {
    let schema = SchemaBody::Set {
        element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
        cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
    };
    let fixture = case(
        "QG-Set",
        None,
        schema.clone(),
        [
            D::Set(vec![D::U8(1), D::U8(2)].into_boxed_slice()),
            D::Set(vec![D::U8(2), D::U8(3)].into_boxed_slice()),
        ],
    );
    let reverse = snapshot(schema, D::Set(vec![D::U8(2), D::U8(1)].into_boxed_slice()));
    assert_exact(
        &reverse,
        &fixture.values[0],
        "QG-Set: distinct keys canonicalize in order",
    );
    let schemas = fixture.values[0].schemas().unwrap();
    let duplicate = ValueDraft {
        schema: fixture.values[0].schema(),
        shape_values: Box::new([]),
        data: D::Set(vec![D::U8(1), D::U8(1)].into_boxed_slice()),
    };
    assert!(
        duplicate
            .finalize(&SnapshotValidationContext::new(&schemas))
            .is_err()
    );
    check(fixture);
}

fn map_case() -> Case {
    let map = |value| {
        D::Map(
            vec![MapEntryDraft {
                items: vec![D::String("x".into()), optional(value)].into_boxed_slice(),
            }]
            .into_boxed_slice(),
        )
    };
    let mut fixture = case(
        "QG-Map",
        None,
        SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::Option(Box::new(SchemaBody::UnsignedInteger(
                IntegerWidth::W8,
            )))),
            cardinality: DimensionExpr::Constant(1).into(),
        },
        [map(Some(D::U8(1))), map(None)],
    );
    fixture.incompatible = Some(snapshot(
        SchemaBody::Map {
            key: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            value: Box::new(SchemaBody::Option(Box::new(SchemaBody::UnsignedInteger(
                IntegerWidth::W8,
            )))),
            cardinality: DimensionExpr::Constant(1).into(),
        },
        D::Map(
            vec![MapEntryDraft {
                items: vec![D::U8(1), optional(None)].into_boxed_slice(),
            }]
            .into_boxed_slice(),
        ),
    ));
    fixture
}

#[test]
fn exact_generic_map() {
    check(map_case());
}

// Negative admission obligations are separate tests: a mistaken rejection
// expectation must never hide the 26 exact positive publication witnesses.
// These boundaries enforce semantic schema identity; raw arena captures do not.
fn check_rejections(case: Case) {
    let incompatible = case.incompatible.as_ref().unwrap();
    let catalog = mech_stdlib::source_catalog();
    let artifact = canonical(&case, Arc::clone(&catalog))
        .compile_artifact()
        .unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x58c, 2),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    let declared = instance.plan.inputs[0].clone();
    let good = case.values[0]
        .rebind(declared.schema, &declared.shape, artifact.schemas())
        .unwrap();
    instance
        .prepare_turn_values(&[CapturedValueInput {
            slot: declared.slot,
            value: &good,
        }])
        .and_then(|prepared| prepared.publish())
        .unwrap();
    let prior = instance.copied_output(0).unwrap();
    let epoch = instance.published_epoch();
    assert_ne!(incompatible.schema_key(), good.schema_key());
    assert!(
        incompatible
            .rebind(declared.schema, &declared.shape, artifact.schemas())
            .is_err(),
        "{}: equivalent-schema rebind rejects incompatible definition",
        case.id
    );
    assert!(
        instance
            .prepare_turn_values(&[CapturedValueInput {
                slot: declared.slot,
                value: incompatible,
            }])
            .is_err(),
        "{}: schema-aware Value admission rejects incompatible definition",
        case.id
    );
    assert_eq!(instance.published_epoch(), epoch);
    assert_exact(
        &instance.copied_output(0).unwrap(),
        &prior,
        &format!(
            "{}: rejected Value admission preserves publication",
            case.id
        ),
    );
    assert!(
        canonical(&case, Arc::clone(&catalog))
            .bind_input_constants(&[(0, incompatible.clone())])
            .is_err(),
        "{}: canonical constant binding rejects incompatible definition",
        case.id
    );
}

#[test]
fn reject_incompatible_snapshot_scalar_schema() {
    check_rejections(case(
        "QN-scalar-schema",
        Some("u8"),
        SchemaBody::UnsignedInteger(IntegerWidth::W8),
        [D::U8(1), D::U8(2)],
    ));
}

#[test]
fn reject_incompatible_native_scalar_schema() {
    check_rejections(case(
        "QN-native-scalar-schema",
        Some("bool"),
        SchemaBody::Bool,
        [D::Bool(false), D::Bool(true)],
    ));
}

#[test]
fn reject_incompatible_atom_nominal_identity() {
    check_rejections(atom_case());
}

#[test]
fn reject_incompatible_matrix_extent() {
    check_rejections(matrix_case());
}

#[test]
fn reject_incompatible_map_key_schema() {
    check_rejections(map_case());
}

fn compile_source(source: &str) -> CanonicalSourceProgram {
    let document = SourceDocument::parse_resolved(
        "constant-binding.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        document.is_strictly_clean(),
        "{source}: {:?}",
        document.snapshot().diagnostics
    );
    CanonicalSourceFrontend
        .compile_document_with_catalog(&document.document(), mech_stdlib::source_catalog())
        .unwrap()
}

fn number(value: f64) -> Value {
    snapshot(
        SchemaBody::FloatingPoint(FloatWidth::W64),
        D::F64(F64Bits::from_f64(value)),
    )
}

fn bind_named(
    program: CanonicalSourceProgram,
    bindings: &[(&str, Value)],
) -> CanonicalSourceProgram {
    let bindings = bindings
        .iter()
        .map(|(name, value)| {
            let input = program
                .program()
                .inputs
                .iter()
                .position(|input| input.name == *name)
                .unwrap();
            (input as u32, value.clone())
        })
        .collect::<Vec<_>>();
    program.bind_input_constants(&bindings).unwrap()
}

fn assert_bound(program: &CanonicalSourceProgram, expected: &Value) {
    let direct = program.compile_artifact().unwrap();
    for artifact in [direct.clone(), decoded(&direct)] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 2),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            assert_exact(
                &instance.copied_output(0).unwrap(),
                expected,
                "bound control",
            );
        }
    }
}

#[test]
fn bound_match_relocates_literal_patterns_and_arm_constants() {
    for (signal, expected) in [(0.0, 11.0), (0.5, 22.0)] {
        let program = compile_source("answer := signal<f64> ? | 0 => 11 | * => 22.\nanswer\n");
        assert_bound(
            &bind_named(program, &[("signal", number(signal))]),
            &number(expected),
        );
    }
}

#[test]
fn bound_nested_match_relocates_parameters_operations_and_yields() {
    let program = compile_source(
        "answer := signal<f64> ? | item => (item ? | 0 => item + 1 | * => item + 2).\nanswer\n",
    );
    assert_bound(
        &bind_named(program, &[("signal", number(0.5))]),
        &number(2.5),
    );
}

#[test]
fn bound_guard_relocates_captures_and_keeps_remaining_input_live() {
    let program = compile_source(
        "answer := flag<bool> ? | x, !x => signal<f64> + 2 | * => signal<f64> + 1.\nanswer\n",
    );
    let program = bind_named(
        program,
        &[("flag", snapshot(SchemaBody::Bool, D::Bool(false)))],
    );
    assert_eq!(program.program().inputs.len(), 1);
    assert_eq!(program.program().inputs[0].name, "signal");
    let direct = program.compile_artifact().unwrap();
    for artifact in [direct.clone(), decoded(&direct)] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 3),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let input = instance.plan.inputs[0].clone();
        for value in [0.5, 3.5] {
            let signal = number(value)
                .rebind(input.schema, &input.shape, artifact.schemas())
                .unwrap();
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: input.slot,
                    value: &signal,
                }])
                .and_then(|prepared| prepared.publish())
                .unwrap();
            assert_exact(
                &instance.copied_output(0).unwrap(),
                &number(value + 2.0),
                "live captured input",
            );
        }
    }
    // A second binding pass must accept and relocate the already rebuilt table.
    assert_bound(
        &bind_named(program, &[("signal", number(0.5))]),
        &number(2.5),
    );
}

#[test]
fn bound_comprehension_relocates_pattern_filter_operation_and_yield() {
    let body = SchemaBody::Matrix {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)].into_boxed_slice(),
    };
    let input = snapshot(
        body,
        D::Matrix(
            vec![-1.0, 0.5, 2.0]
                .into_iter()
                .map(|n| D::F64(F64Bits::from_f64(n)))
                .collect(),
        ),
    );
    let program = compile_source("answer := [x + 1 | x <- signal<[f64]:1,3>, x > 0]\nanswer\n");
    let program = bind_named(program, &[("signal", input)]);
    let direct = program.compile_artifact().unwrap();
    for artifact in [direct.clone(), decoded(&direct)] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 4),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            let output = instance.copied_output(0).unwrap();
            assert_eq!(
                output.canonical_data_draft().unwrap(),
                D::Matrix(
                    vec![
                        D::F64(F64Bits::from_f64(1.5)),
                        D::F64(F64Bits::from_f64(3.0))
                    ]
                    .into_boxed_slice()
                )
            );
        }
    }
}

#[test]
fn binding_relocates_existing_state_initializer_and_output_constants() {
    let program = compile_source("~counter := 11\ncounter += signal<f64>\ncounter\n");
    let program = bind_named(program, &[("signal", number(0.5))]);
    let direct = program.compile_artifact().unwrap();
    for artifact in [direct.clone(), decoded(&direct)] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 5),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for expected in [11.5, 12.0] {
            instance.turn(&[]).unwrap();
            assert_exact(
                &instance.copied_output(0).unwrap(),
                &number(expected),
                "bound state",
            );
        }
    }
}

#[test]
fn simultaneous_bindings_are_order_independent() {
    let source = "answer := left<f64> * 10 + right<f64>\nanswer\n";
    let left = ("left", number(0.5));
    let right = ("right", number(2.0));
    let forward = bind_named(compile_source(source), &[left.clone(), right.clone()]);
    let reverse = bind_named(compile_source(source), &[right, left]);
    assert_bound(&forward, &number(7.0));
    assert_bound(&reverse, &number(7.0));
    assert_eq!(
        mech_engine::encode_program_artifact_bytecode_v1(&forward.compile_artifact().unwrap())
            .unwrap(),
        mech_engine::encode_program_artifact_bytecode_v1(&reverse.compile_artifact().unwrap())
            .unwrap(),
    );
}

#[test]
fn binding_relocates_dynamic_descendants_in_existing_constants() {
    let payload_body = SchemaBody::Tuple(vec![SchemaBody::Dynamic].into_boxed_slice());
    let mut schemas = SchemaTableBuilder::new();
    let mut insert = |body| {
        schemas
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap()
    };
    let child = insert(SchemaBody::Tuple(
        vec![SchemaBody::FloatingPoint(FloatWidth::W64), SchemaBody::Bool].into_boxed_slice(),
    ));
    let payload = insert(payload_body.clone());
    let root = insert(SchemaBody::Tuple(
        vec![
            SchemaBody::FloatingPoint(FloatWidth::W64),
            payload_body.clone(),
        ]
        .into_boxed_slice(),
    ));
    let build = schemas.finish().unwrap();
    let payload_data = D::Tuple(
        vec![D::Dynamic(Some(Box::new(ValueDraft {
            schema: build.resolve(child).unwrap(),
            shape_values: Box::new([]),
            data: D::Tuple(vec![D::F64(F64Bits::from_f64(1.0)), D::Bool(true)].into_boxed_slice()),
        })))]
        .into_boxed_slice(),
    );
    let value = ValueDraft {
        schema: build.resolve(payload).unwrap(),
        shape_values: Box::new([]),
        data: payload_data.clone(),
    }
    .finalize(&SnapshotValidationContext::new(&build.table))
    .unwrap();
    let expected = ValueDraft {
        schema: build.resolve(root).unwrap(),
        shape_values: Box::new([]),
        data: D::Tuple(vec![D::F64(F64Bits::from_f64(0.5)), payload_data].into_boxed_slice()),
    }
    .finalize(&SnapshotValidationContext::new(&build.table))
    .unwrap();
    let document = SourceDocument::parse_resolved(
        "nested-dynamic.mec",
        Revision(0),
        "answer := (signal<f64>, payload)\nanswer\n",
        ParseConfig::default(),
    )
    .unwrap();
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog_and_input_schemas(
            &document.document(),
            mech_stdlib::source_catalog(),
            BTreeMap::from([("payload".into(), payload_body)]),
        )
        .unwrap();
    let program = bind_named(
        bind_named(program, &[("payload", value)]),
        &[("signal", number(0.5))],
    );
    let direct = program.compile_artifact().unwrap();
    for artifact in [direct.clone(), decoded(&direct)] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 6),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance.turn(&[]).unwrap();
            let output = instance.copied_output(0).unwrap();
            assert_eq!(output.schema_key(), expected.schema_key());
            // Dynamic descendants contain table-local IDs. Canonical identity
            // compares their schema keys and payloads across independent tables.
            assert_eq!(
                output.value_hash(&output.schemas().unwrap()).unwrap(),
                expected.value_hash(&build.table).unwrap()
            );
        }
    }
}

#[test]
fn exact_id_input_and_binding() {
    check(case(
        "Q30-Id",
        Some("id"),
        SchemaBody::Id,
        [D::Id(7), D::Id(9)],
    ));
    let fixture = case(
        "Q30-Id-rejection",
        Some("id"),
        SchemaBody::Id,
        [D::Id(7), D::Id(9)],
    );
    assert_eq!(
        canonical(&fixture, mech_stdlib::source_catalog())
            .bind_input_constants(&[(0, fixture.incompatible.unwrap())])
            .err()
            .unwrap()
            .code,
        "source-semantics/import-value-schema-mismatch"
    );
}

#[test]
fn exact_index_input_and_binding() {
    assert!(
        mech_runtime::RuntimeHostInputValue::Index(0)
            .into_value()
            .is_err()
    );
    check(case(
        "Q31-Index",
        Some("ix"),
        SchemaBody::Index,
        [D::Index(1), D::Index(2)],
    ));
    let valid = snapshot(SchemaBody::Index, D::Index(1));
    assert!(
        ValueDraft {
            schema: valid.schema(),
            shape_values: Box::new([]),
            data: D::Index(0)
        }
        .finalize(&SnapshotValidationContext::new(&valid.schemas().unwrap()))
        .is_err()
    );
}

#[test]
fn exact_generic_enum_input_and_binding() {
    use mech_core::{EnumVariantSchema, snapshot::EnumDraft};
    let schema = SchemaBody::Enum {
        key: NominalKey::from_path(
            NominalKind::Enum,
            &CanonicalNominalPath::new(vec!["test".into(), "enum".into()]).unwrap(),
        ),
        variants: vec![EnumVariantSchema {
            name: "value".into(),
            payload: Some(SchemaBody::FloatingPoint(FloatWidth::W64)),
        }]
        .into_boxed_slice(),
    };
    let data = |ordinal, payload| {
        D::Enum(EnumDraft {
            ordinal,
            payload: Some(Box::new(payload)),
        })
    };
    check(case(
        "Q32-Enum",
        None,
        schema.clone(),
        [
            data(0, D::F64(F64Bits::from_f64(9.0))),
            data(0, D::F64(F64Bits::from_f64(10.0))),
        ],
    ));
    let value = snapshot(schema, data(0, D::F64(F64Bits::from_f64(9.0))));
    for invalid in [
        data(1, D::F64(F64Bits::from_f64(9.0))),
        data(0, D::Bool(true)),
    ] {
        assert!(
            ValueDraft {
                schema: value.schema(),
                shape_values: Box::new([]),
                data: invalid
            }
            .finalize(&SnapshotValidationContext::new(&value.schemas().unwrap()))
            .is_err()
        );
    }
}

#[test]
fn binding_errors_retain_the_input_anchor() {
    let program = compile_source("answer := left<f64> + right<f64>\nanswer\n");
    let input = program
        .program()
        .inputs
        .iter()
        .position(|input| input.name == "right")
        .unwrap();
    let expected_anchor = program.source_map().inputs[input];
    let repeated = [(input as u32, number(1.0)), (input as u32, number(2.0))];
    let error = program
        .clone()
        .bind_input_constants(&repeated)
        .err()
        .unwrap();
    assert_eq!(error.code, "source-semantics/duplicate-input-binding");
    assert_eq!(error.anchor, expected_anchor);
    let error = program
        .clone()
        .bind_input_constants(&[(
            input as u32,
            snapshot(SchemaBody::String, D::String("wrong".into())),
        )])
        .err()
        .unwrap();
    assert_eq!(error.code, "source-semantics/import-value-schema-mismatch");
    assert_eq!(error.anchor, expected_anchor);
    assert_eq!(
        program
            .bind_input_constants(&[(2, number(1.0))])
            .err()
            .unwrap()
            .code,
        "source-semantics/input-binding-out-of-range"
    );
}

#[test]
fn successive_bindings_relocate_state_nodes_outputs_constraints_and_source_map() {
    let source = "~counter := a<f64>\ncounter += b<f64>\nanswer := counter + c<f64>\nsafe! := c > 0\nanswer\n";
    let program = compile_source(source);
    let c = program
        .program()
        .inputs
        .iter()
        .position(|input| input.name == "c")
        .unwrap();
    let c_anchor = program.source_map().inputs[c];
    let program = bind_named(
        bind_named(program, &[("b", number(2.0))]),
        &[("a", number(11.0))],
    );
    assert_eq!(program.program().inputs.len(), 1);
    assert_eq!(program.program().inputs[0].name, "c");
    assert_eq!(program.source_map().inputs.as_ref(), &[c_anchor]);
    assert_eq!(program.program().constraints.len(), 1);
    let direct = program.compile_artifact().unwrap();
    for artifact in [direct.clone(), decoded(&direct)] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x58c, 7),
            &artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
        )
        .unwrap();
        let input = instance.plan.inputs[0].clone();
        for (c, expected) in [(1.0, 14.0), (3.0, 18.0)] {
            let value = number(c)
                .rebind(input.schema, &input.shape, artifact.schemas())
                .unwrap();
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: input.slot,
                    value: &value,
                }])
                .and_then(|prepared| prepared.publish())
                .unwrap();
            assert_exact(
                &instance.copied_output(0).unwrap(),
                &number(expected),
                "all binding owners",
            );
        }
        let invalid = number(-1.0)
            .rebind(input.schema, &input.shape, artifact.schemas())
            .unwrap();
        assert!(
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: input.slot,
                    value: &invalid
                }])
                .and_then(|prepared| prepared.publish())
                .is_err()
        );
        assert_exact(
            &instance.copied_output(0).unwrap(),
            &number(18.0),
            "constraint rollback",
        );
    }
}
