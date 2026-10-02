//! Exact source/decoded acceptance for recovery finding G02's canonical
//! numeric target families.
#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use std::collections::BTreeMap;

use mech_core::snapshot::{
    Complex32Bits, Complex64Bits, F32Bits, F64Bits, SnapshotValidationContext,
};
use mech_core::{
    DimensionExpr, FloatWidth, IntegerWidth, ReactiveInstanceId, SchemaBody, SchemaDraft,
    SchemaTableBuilder, Value, ValueDataDraft as D, ValueDraft,
};
use mech_engine::resident::{ActivationFacts, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact};
use mech_runtime::{
    ResidentDurabilityPolicy, RuntimeBuilder, RuntimeValueSnapshot, SourceDocument,
};
use mech_syntax::document::{ParseConfig, Revision};

fn compile(source: &str) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(
        "s8-recovery-numeric-targets.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(document.is_strictly_clean());
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap()
        .compile_document(&document)
        .unwrap()
        .artifact()
        .clone()
}

fn exact_outputs(artifact: &ProgramArtifact, expected: &[&str]) {
    let catalog = mech_stdlib::source_catalog();
    let mut instance = activate(
        ReactiveInstanceId::new(0x58c, 2),
        artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    for expected in expected {
        instance.turn(&[]).unwrap();
        let output = RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap())
            .unwrap()
            .format_canonical_inline();
        assert_eq!(&output, expected);
    }
}

fn source_and_decoded(source: &str, expected: &[&str]) {
    let artifact = compile(source);
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    exact_outputs(&artifact, expected);
    exact_outputs(&decoded, expected);
}

#[test]
fn canonical_power_family_executes_source_and_decoded() {
    for (kind, body, data) in [
        (
            "u64",
            SchemaBody::UnsignedInteger(IntegerWidth::W64),
            D::U64(4),
        ),
        (
            "u128",
            SchemaBody::UnsignedInteger(IntegerWidth::W128),
            D::U128(4),
        ),
        ("i8", SchemaBody::SignedInteger(IntegerWidth::W8), D::I8(4)),
        (
            "i16",
            SchemaBody::SignedInteger(IntegerWidth::W16),
            D::I16(4),
        ),
        (
            "i32",
            SchemaBody::SignedInteger(IntegerWidth::W32),
            D::I32(4),
        ),
        (
            "i64",
            SchemaBody::SignedInteger(IntegerWidth::W64),
            D::I64(4),
        ),
        (
            "i128",
            SchemaBody::SignedInteger(IntegerWidth::W128),
            D::I128(4),
        ),
        (
            "c32",
            SchemaBody::Complex(FloatWidth::W32),
            complex(FloatWidth::W32, 4.0, 0.0),
        ),
        (
            "c64",
            SchemaBody::Complex(FloatWidth::W64),
            complex(FloatWidth::W64, 4.0, 0.0),
        ),
        (
            "r64",
            SchemaBody::Rational64,
            D::Rational64 {
                numerator: 4,
                denominator: 1,
            },
        ),
    ] {
        let expected = snapshot(body, data);
        public_source(
            &format!("answer := 2<{kind}> ^ 2<{kind}>\nanswer\n"),
            &[expected.clone(), expected],
        );
    }
}

#[test]
fn canonical_c32_arithmetic_and_updates_execute_source_and_decoded() {
    for (source, expected) in [
        (
            "~a := 1<c32>\na += 1<c32>\na == 2<c32>\n",
            &["true", "false"][..],
        ),
        (
            "answer := 6<c32> + 2<c32>\nanswer == 8<c32>\n",
            &["true", "true"],
        ),
        (
            "answer := 6<c32> - 2<c32>\nanswer == 4<c32>\n",
            &["true", "true"],
        ),
        (
            "answer := 6<c32> * 2<c32>\nanswer == 12<c32>\n",
            &["true", "true"],
        ),
        (
            "answer := 6<c32> / 2<c32>\nanswer == 3<c32>\n",
            &["true", "true"],
        ),
        (
            "~a := [10<c32> 20<c32>; 30<c32> 40<c32>]\na[[1 1],:] += 2<c32>\na[1,1] == 14<c32>\n",
            &["true", "false"],
        ),
    ] {
        source_and_decoded(source, expected);
    }

    for (operation, expected) in [("+", 6), ("-", 2), ("*", 8), ("/", 2)] {
        source_and_decoded(
            &format!(
                "a := [1<c32> 2<c32>; 3<c32> 4<c32>]\nanswer := a {operation} 2<c32>\nanswer[2,2] == {expected}<c32>\n"
            ),
            &["true", "true"],
        );
    }
}

#[test]
fn canonical_complex_and_rational_matrix_ops_execute_source_and_decoded() {
    source_and_decoded(
        "+> stats\na := [1<c32> 2<c32>; 3<c32> 4<c32>]\nanswer := stats/sum/row(a)\nanswer[2] == 6<c32>\n",
        &["true", "true"],
    );

    for kind in ["c32", "c64", "r64"] {
        source_and_decoded(
            &format!(
                "a := [1<{kind}> 2<{kind}>; 3<{kind}> 4<{kind}>]\nanswer := matrix/matmul(a,a)\nanswer[2,2] == 22<{kind}>\n"
            ),
            &["true", "true"],
        );
    }
}

#[test]
fn canonical_f32_special_binary_broadcast_remains_executable() {
    source_and_decoded(
        "+> math\na := [1f32 2f32;3f32 4f32]\nanswer := math/copysign(a,-1f32)\nanswer\n",
        &["[-1 -2; -3 -4]", "[-1 -2; -3 -4]"],
    );
}

fn snapshot(body: SchemaBody, data: D) -> Value {
    let mut schemas = SchemaTableBuilder::new();
    let handle = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = schemas.finish().unwrap();
    ValueDraft {
        schema: built.resolve(handle).unwrap(),
        shape_values: Box::new([]),
        data,
    }
    .finalize(&SnapshotValidationContext::new(&built.table))
    .unwrap()
}

fn complex(width: FloatWidth, real: f64, imaginary: f64) -> D {
    match width {
        FloatWidth::W32 => D::Complex32(Complex32Bits::new(
            F32Bits::from_f32(real as f32),
            F32Bits::from_f32(imaginary as f32),
        )),
        FloatWidth::W64 => D::Complex64(Complex64Bits::new(
            F64Bits::from_f64(real),
            F64Bits::from_f64(imaginary),
        )),
    }
}

fn scalar(width: FloatWidth, real: f64, imaginary: f64) -> Value {
    snapshot(
        SchemaBody::Complex(width.clone()),
        complex(width, real, imaginary),
    )
}

// Canonical drafts are written in row-major order, independently of either executor.
fn matrix(element: SchemaBody, rows: u64, columns: u64, values: Vec<D>) -> Value {
    snapshot(
        SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![
                DimensionExpr::Constant(rows),
                DimensionExpr::Constant(columns),
            ]
            .into_boxed_slice(),
        },
        D::Matrix(values.into_boxed_slice()),
    )
}

fn complex_matrix(width: FloatWidth, rows: u64, columns: u64, values: &[(f64, f64)]) -> Value {
    matrix(
        SchemaBody::Complex(width.clone()),
        rows,
        columns,
        values
            .iter()
            .map(|&(r, i)| complex(width.clone(), r, i))
            .collect(),
    )
}

fn assert_data(actual: D, expected: D, tolerance: Option<f64>) {
    fn component(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            actual.is_finite(),
            "expected finite {expected}, got {actual}"
        );
        if expected == 0.0 {
            assert_eq!(actual.to_bits(), expected.to_bits(), "exact zero sign");
        } else {
            assert!(
                (actual - expected).abs() <= expected.abs() * tolerance,
                "expected {expected}, got {actual}, relative tolerance {tolerance}"
            );
        }
    }
    match (actual, expected, tolerance) {
        (D::Complex32(a), D::Complex32(e), Some(t)) => {
            component(
                f64::from(a.real().to_f32()),
                f64::from(e.real().to_f32()),
                t,
            );
            component(
                f64::from(a.imaginary().to_f32()),
                f64::from(e.imaginary().to_f32()),
                t,
            );
        }
        (D::Complex64(a), D::Complex64(e), Some(t)) => {
            component(a.real().to_f64(), e.real().to_f64(), t);
            component(a.imaginary().to_f64(), e.imaginary().to_f64(), t);
        }
        (D::Matrix(a), D::Matrix(e), t) => {
            assert_eq!(a.len(), e.len());
            for (a, e) in a.into_iter().zip(e) {
                assert_data(a, e, t);
            }
        }
        (a, e, _) => assert_eq!(a, e, "exact canonical components"),
    }
}

fn assert_value(actual: &Value, expected: &Value, tolerance: Option<f64>) {
    assert_eq!(
        actual.schema_key(),
        expected.schema_key(),
        "exact canonical result kind and dimensions"
    );
    assert_eq!(
        actual.shape().parameter_values(),
        expected.shape().parameter_values()
    );
    let a = actual.schemas().unwrap();
    let e = expected.schemas().unwrap();
    assert_eq!(
        a.get(actual.schema())
            .unwrap()
            .closed_body(actual.shape())
            .unwrap(),
        e.get(expected.schema())
            .unwrap()
            .closed_body(expected.shape())
            .unwrap()
    );
    assert_data(
        actual.canonical_data_draft().unwrap(),
        expected.canonical_data_draft().unwrap(),
        tolerance,
    );
}

// The first publication happens during loading; later values come from public
// steps. Exercise both the public compiler/load route and bytecode decoding.
fn public_outputs(
    source: Option<&str>,
    artifact: &ProgramArtifact,
    expected: &[Value],
    tolerance: Option<f64>,
) {
    for bytecode in [false, true] {
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap();
        let outcome = if bytecode {
            runtime.load_bytecode_program(
                &mech_engine::encode_program_artifact_bytecode_v1(artifact).unwrap(),
                ResidentDurabilityPolicy::Volatile,
            )
        } else if let Some(source) = source {
            runtime.load_source_program(source, ResidentDurabilityPolicy::Volatile)
        } else {
            runtime.load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
        }
        .unwrap();
        assert_value(&outcome.initial_value.to_value(), &expected[0], tolerance);
        let output = artifact.outputs()[0].output;
        for (turn, expected) in expected.iter().enumerate() {
            if turn != 0 {
                runtime.step_active_program().unwrap();
            }
            assert_value(
                &runtime.output_value(output).unwrap().unwrap().to_value(),
                expected,
                tolerance,
            );
        }
    }
}

fn public_source(source: &str, expected: &[Value]) {
    public_outputs(Some(source), &compile(source), expected, None);
}

fn bound(source: &str, inputs: &[(&str, Value)]) -> ProgramArtifact {
    let document = SourceDocument::parse_resolved(
        "numeric-edge-inputs.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(document.is_strictly_clean());
    let schemas = inputs
        .iter()
        .map(|(name, value)| {
            let owner = value.schemas().unwrap();
            (
                (*name).to_owned(),
                owner.get(value.schema()).unwrap().body().clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog_and_input_schemas(
            &document.document(),
            mech_stdlib::source_catalog(),
            schemas,
        )
        .unwrap();
    let bindings = program
        .program()
        .inputs
        .iter()
        .enumerate()
        .map(|(index, input)| {
            (
                index as u32,
                inputs
                    .iter()
                    .find(|(name, _)| *name == input.name)
                    .unwrap()
                    .1
                    .clone(),
            )
        })
        .collect::<Vec<_>>();
    program
        .bind_input_constants(&bindings)
        .unwrap()
        .compile_artifact()
        .unwrap()
}

#[test]
fn public_nonreal_arithmetic_has_exact_components_and_types() {
    for (kind, width) in [("c32", FloatWidth::W32), ("c64", FloatWidth::W64)] {
        for (expression, result) in [
            (format!("(1+2i<{kind}>) * (3-4i<{kind}>)"), (11.0, 2.0)),
            (format!("(1+3i<{kind}>) / (1+1i<{kind}>)"), (2.0, 1.0)),
        ] {
            let expected = scalar(width.clone(), result.0, result.1);
            public_source(
                &format!("answer := {expression}\nanswer\n"),
                &[expected.clone(), expected],
            );
        }
    }
}

#[test]
fn public_matrix_products_check_every_element_and_shape() {
    for (kind, width) in [("c32", FloatWidth::W32), ("c64", FloatWidth::W64)] {
        let source = format!(
            "a := [(1+2i<{kind}>) (3-1i<{kind}>); (2+1i<{kind}>) (1+2i<{kind}>)]\nb := [(2-1i<{kind}>) (1+1i<{kind}>); (2+3i<{kind}>) (4-2i<{kind}>)]\nanswer := matrix/matmul(a,b)\nanswer\n"
        );
        let expected = complex_matrix(
            width,
            2,
            2,
            &[(13.0, 10.0), (9.0, -7.0), (1.0, 7.0), (9.0, 9.0)],
        );
        public_source(&source, &[expected.clone(), expected]);
    }
    let expected = matrix(
        SchemaBody::Rational64,
        2,
        2,
        vec![
            D::Rational64 {
                numerator: 31,
                denominator: 72,
            },
            D::Rational64 {
                numerator: 17,
                denominator: 36,
            },
            D::Rational64 {
                numerator: 5,
                denominator: 12,
            },
            D::Rational64 {
                numerator: 0,
                denominator: 1,
            },
        ],
    );
    public_source(
        "a := [1/2 1/3; 2/3 -1/2]\nb := [3/4 1/2; 1/6 2/3]\nanswer := matrix/matmul(a,b)\nanswer\n",
        &[expected.clone(), expected],
    );
}

#[test]
fn public_state_and_selected_updates_assert_successive_values() {
    public_source(
        "~a := (1+2i<c32>)\na += (2-1i<c32>)\na\n",
        &[
            scalar(FloatWidth::W32, 3.0, 1.0),
            scalar(FloatWidth::W32, 5.0, 0.0),
        ],
    );
    public_source(
        "~a := [(1+2i<c32>) (3-1i<c32>); (2+1i<c32>) (1+2i<c32>)]\na[[1 1],:] += (2+1i<c32>)\na\n",
        &[
            complex_matrix(
                FloatWidth::W32,
                2,
                2,
                &[(5.0, 4.0), (7.0, 1.0), (2.0, 1.0), (1.0, 2.0)],
            ),
            complex_matrix(
                FloatWidth::W32,
                2,
                2,
                &[(9.0, 6.0), (11.0, 3.0), (2.0, 1.0), (1.0, 2.0)],
            ),
        ],
    );
}

#[test]
fn public_power_layouts_preserve_exact_complex_and_rational_kinds() {
    for (expression, expected) in [
        ("(1+1i<c32>) ^ 2<c32>", scalar(FloatWidth::W32, 0.0, 2.0)),
        (
            "(1+1i<c32>) ^ [2<c32> 3<c32>]",
            complex_matrix(FloatWidth::W32, 1, 2, &[(0.0, 2.0), (-2.0, 2.0)]),
        ),
        (
            "[(1+1i<c32>) (2-1i<c32>)] ^ 2<c32>",
            complex_matrix(FloatWidth::W32, 1, 2, &[(0.0, 2.0), (3.0, -4.0)]),
        ),
        (
            "[(1+1i<c32>) (2-1i<c32>)] ^ [2<c32> 3<c32>]",
            complex_matrix(FloatWidth::W32, 1, 2, &[(0.0, 2.0), (2.0, -11.0)]),
        ),
        (
            "1/2 ^ 2<i32>",
            snapshot(
                SchemaBody::Rational64,
                D::Rational64 {
                    numerator: 1,
                    denominator: 4,
                },
            ),
        ),
    ] {
        public_source(
            &format!("answer := {expression}\nanswer\n"),
            &[expected.clone(), expected],
        );
    }
}

#[test]
fn public_fractional_power_edges_reach_scalar_and_aggregate_helpers() {
    for (width, maximum, near_one, sqrt, near, tolerance) in [
        (
            FloatWidth::W32,
            f64::from(f32::MAX),
            f64::from(f32::from_bits(1.0_f32.to_bits() - 1)),
            (2.0267144054983168e19, 8.394925938143273e18),
            (3.4028055603080294e38, 3.4028052417143948e38),
            4e-7,
        ),
        (
            FloatWidth::W64,
            f64::MAX,
            f64::from_bits(1.0_f64.to_bits() - 1),
            (1.4730945569055654e154, 6.101757441282702e153),
            (1.7976931348621741e308, 1.7976931348621738e308),
            4e-14,
        ),
    ] {
        for (power, reference) in [(0.5, sqrt), (near_one, near)] {
            for aggregate in [false, true] {
                let base = if aggregate {
                    complex_matrix(
                        width.clone(),
                        1,
                        2,
                        &[(maximum, maximum), (maximum, -maximum)],
                    )
                } else {
                    scalar(width.clone(), maximum, maximum)
                };
                let expected = if aggregate {
                    complex_matrix(
                        width.clone(),
                        1,
                        2,
                        &[reference, (reference.0, -reference.1)],
                    )
                } else {
                    scalar(width.clone(), reference.0, reference.1)
                };
                let artifact = bound(
                    "answer := base ^ power\nanswer\n",
                    &[("base", base), ("power", scalar(width.clone(), power, 0.0))],
                );
                public_outputs(
                    None,
                    &artifact,
                    &[expected.clone(), expected],
                    Some(tolerance),
                );
            }
        }
    }
}

#[test]
fn public_infinite_divisors_preserve_exact_zero_signs() {
    for (width, maximum) in [
        (FloatWidth::W32, f64::from(f32::MAX)),
        (FloatWidth::W64, f64::MAX),
    ] {
        for (divisor, positive, negative) in [
            ((f64::INFINITY, f64::INFINITY), (0.0, 0.0), (-0.0, 0.0)),
            ((f64::NEG_INFINITY, 1.0), (-0.0, -0.0), (0.0, 0.0)),
            ((1.0, f64::NEG_INFINITY), (-0.0, 0.0), (0.0, -0.0)),
        ] {
            for aggregate in [false, true] {
                let left = if aggregate {
                    complex_matrix(
                        width.clone(),
                        1,
                        2,
                        &[(maximum, maximum), (-maximum, -maximum)],
                    )
                } else {
                    scalar(width.clone(), maximum, maximum)
                };
                let expected = if aggregate {
                    complex_matrix(width.clone(), 1, 2, &[positive, negative])
                } else {
                    scalar(width.clone(), positive.0, positive.1)
                };
                let artifact = bound(
                    "answer := left / right\nanswer\n",
                    &[
                        ("left", left),
                        ("right", scalar(width.clone(), divisor.0, divisor.1)),
                    ],
                );
                public_outputs(None, &artifact, &[expected.clone(), expected], None);
            }
        }
    }
}

#[test]
fn public_rejected_late_matrix_update_retains_state_and_recovers() {
    let accepted = matrix(
        SchemaBody::SignedInteger(IntegerWidth::W8),
        1,
        2,
        vec![D::I8(2), D::I8(124)],
    );
    let source = "~a := [1<i8> 120<i8>]\na[:,:] += [1<i8> 4<i8>]\na\n";
    let artifact = compile(source);
    for bytecode in [false, true] {
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap();
        let outcome = if bytecode {
            runtime.load_bytecode_program(
                &mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap(),
                ResidentDurabilityPolicy::Volatile,
            )
        } else {
            runtime.load_source_program(source, ResidentDurabilityPolicy::Volatile)
        }
        .unwrap();
        assert_value(&outcome.initial_value.to_value(), &accepted, None);
        let output = artifact.outputs()[0].output;
        // The first element's next value (3) fits; only the later element (128)
        // overflows. Neither the state cell nor the published result may change.
        let error = runtime.step_active_program().unwrap_err();
        assert_eq!(error.kind_name(), "ResidentRouteFailure");
        assert!(format!("{error:?}").contains("Arithmetic"), "{error:?}");
        assert_value(
            &runtime.output_value(output).unwrap().unwrap().to_value(),
            &accepted,
            None,
        );
        assert!(runtime.step_active_program().is_err());
        assert_value(
            &runtime.output_value(output).unwrap().unwrap().to_value(),
            &accepted,
            None,
        );
        runtime.unload_active_program().unwrap();
        let valid = runtime
            .load_source_program(
                "~b := (1+2i<c32>)\nb += (2-1i<c32>)\nb\n",
                ResidentDurabilityPolicy::Volatile,
            )
            .unwrap();
        assert_value(
            &valid.initial_value.to_value(),
            &scalar(FloatWidth::W32, 3.0, 1.0),
            None,
        );
        runtime.step_active_program().unwrap();
        assert_value(
            &runtime.program_output_value().unwrap().unwrap().to_value(),
            &scalar(FloatWidth::W32, 5.0, 0.0),
            None,
        );
    }
}

#[test]
fn public_large_complex_power_phases_remain_finite() {
    let expected = scalar(FloatWidth::W64, 1.0, 0.0);
    let artifact = bound(
        "answer := base ^ power\nanswer\n",
        &[
            ("base", scalar(FloatWidth::W64, -1.0, 0.0)),
            (
                "power",
                scalar(FloatWidth::W64, f64::MAX, f64::MIN_POSITIVE),
            ),
        ],
    );
    public_outputs(None, &artifact, &[expected.clone(), expected], Some(4e-14));
}

#[test]
fn public_near_unit_complex_powers_preserve_magnitude_and_direction() {
    let expected = complex_matrix(
        FloatWidth::W64,
        1,
        3,
        &[
            (-0.22495495699442813, 0.9743691637791269),
            (-0.22495495699442813, -0.9743691637791269),
            (-0.22495495699442813, -0.9743691637791269),
        ],
    );
    let artifact = bound(
        "answer := base ^ power\nanswer\n",
        &[
            (
                "base",
                complex_matrix(
                    FloatWidth::W64,
                    1,
                    3,
                    &[(1.0, 1e-308), (1.0, -1e-308), (-1.0, 1e-308)],
                ),
            ),
            (
                "power",
                scalar(FloatWidth::W64, f64::MAX, f64::MIN_POSITIVE),
            ),
        ],
    );
    public_outputs(None, &artifact, &[expected.clone(), expected], Some(4e-14));
}

#[test]
fn public_linear_power_duplicates_keep_exact_values_and_types() {
    let expected = |first| {
        matrix(
            SchemaBody::UnsignedInteger(IntegerWidth::W128),
            1,
            2,
            vec![D::U128(first), D::U128(3)],
        )
    };
    public_source(
        "~a := [2<u128> 3<u128>]\na[[1 1]] ^= 2<u128>\na\n",
        &[expected(16), expected(65_536)],
    );
}

#[test]
fn public_linear_power_budget_rejection_preserves_accepted_session() {
    struct Factory(Value);
    impl mech_runtime::ResidentReplRuntimeFactory for Factory {
        fn build(
            &self,
            _: mech_runtime::MechEventBuffer,
        ) -> mech_core::MResult<mech_runtime::MechRuntime> {
            RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .build()
        }
        fn activate_document(
            &self,
            events: mech_runtime::MechEventBuffer,
            document: &SourceDocument,
        ) -> mech_core::MResult<(
            mech_runtime::MechRuntime,
            mech_runtime::RuntimeProgramLoadOutcome,
        )> {
            let artifact = bound(
                &document.source().to_contiguous_string(),
                &[("positions", self.0.clone())],
            );
            let mut runtime = self.build(events)?;
            let outcome =
                runtime.load_compiled_program(artifact, ResidentDurabilityPolicy::Volatile)?;
            Ok((runtime, outcome))
        }
    }
    // This large repeated selection exceeds the resident budgets. The kernel
    // regression separately isolates compute admission from retained-node
    // admission; this public sequence checks rejection and continued usability.
    let positions = matrix(SchemaBody::Index, 1, 16_384, vec![D::Index(1); 16_384]);
    let mut session = mech_runtime::ResidentReplSession::from_source(
        Factory(positions),
        "~a := [(1+1i<c64>) (2-1i<c64>)]\na += 1<c64>\n<+ a\na\n".to_owned(),
    )
    .unwrap();
    let accepted = complex_matrix(FloatWidth::W64, 1, 2, &[(2.0, 1.0), (3.0, -1.0)]);
    assert_value(
        &session.symbol("a").unwrap().unwrap().to_value(),
        &accepted,
        None,
    );
    let rejected = SourceDocument::parse_resolved(
        "linear-power-budget.mec",
        Revision(1),
        "~a := [(1+1i<c64>) (2-1i<c64>)]\na[positions] ^= 1<c64>\n<+ a\na\n",
        ParseConfig::default(),
    )
    .unwrap();
    let error = session.replace_document(rejected).unwrap_err();
    assert_eq!(error.kind_name(), "ResidentRouteFailure");
    assert!(format!("{error:?}").contains("InvalidShape"), "{error:?}");
    assert_value(
        &session.symbol("a").unwrap().unwrap().to_value(),
        &accepted,
        None,
    );
    session.step(1).unwrap();
    let next = complex_matrix(FloatWidth::W64, 1, 2, &[(3.0, 1.0), (4.0, -1.0)]);
    assert_value(
        &session.symbol("a").unwrap().unwrap().to_value(),
        &next,
        None,
    );
}

#[test]
fn public_session_rejects_candidate_without_replacing_accepted_state() {
    struct Factory;
    impl mech_runtime::ResidentReplRuntimeFactory for Factory {
        fn build(
            &self,
            _: mech_runtime::MechEventBuffer,
        ) -> mech_core::MResult<mech_runtime::MechRuntime> {
            RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .build()
        }
    }
    let mut session = mech_runtime::ResidentReplSession::from_source(
        Factory,
        "~a := [1<i8> 120<i8>]\na[:,:] += [1<i8> 1<i8>]\na\n".to_owned(),
    )
    .unwrap();
    let accepted = matrix(
        SchemaBody::SignedInteger(IntegerWidth::W8),
        1,
        2,
        vec![D::I8(2), D::I8(121)],
    );
    assert_value(
        &session.symbol("a").unwrap().unwrap().to_value(),
        &accepted,
        None,
    );
    let rejected = SourceDocument::parse_resolved(
        "rejected-numeric.mec",
        Revision(1),
        "~a := [1<i8> 120<i8>]\na[:,:] += [1<i8> 8<i8>]\na\n",
        ParseConfig::default(),
    )
    .unwrap();
    let error = session.replace_document(rejected).unwrap_err();
    assert_eq!(error.kind_name(), "ResidentRouteFailure");
    assert!(format!("{error:?}").contains("Arithmetic"), "{error:?}");
    assert_value(
        &session.symbol("a").unwrap().unwrap().to_value(),
        &accepted,
        None,
    );
    // Continue the same accepted program, so restarting from its seed or
    // mutating an early element of the rejected candidate cannot pass.
    session.step(1).unwrap();
    let next = matrix(
        SchemaBody::SignedInteger(IntegerWidth::W8),
        1,
        2,
        vec![D::I8(3), D::I8(122)],
    );
    assert_value(
        &session.symbol("a").unwrap().unwrap().to_value(),
        &next,
        None,
    );
}
