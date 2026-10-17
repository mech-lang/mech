#![cfg(feature = "full_compiler")]

use mech_core::{
    DimensionExpr, ExecutionTarget, FunctionInvocation, IntegerWidth, OperationId,
    ResolvedOperationDescriptor, RuntimeFunctionId, SchemaBody, SpecializedFunction, ValueCell,
    ValueData,
    snapshot::{SequenceView, ValueDataDraft},
};

fn canonical(body: SchemaBody, data: ValueDataDraft) -> ValueCell {
    ValueCell::from_schema_data(body, data).unwrap()
}

fn count(value: u64) -> ValueCell {
    canonical(
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        ValueDataDraft::U64(value),
    )
}

fn bind(
    operation: &str,
    factory: &str,
    output: &ValueCell,
    inputs: &[ValueCell],
) -> SpecializedFunction {
    let catalog = mech_stdlib::source_catalog();
    let operation_id = OperationId::from_name(operation);
    let entry = catalog
        .runtime_entry(RuntimeFunctionId::from_name(factory))
        .unwrap();
    let invocation = match inputs {
        [input] => FunctionInvocation::unary(output.clone(), input.clone()),
        [first, second] => {
            FunctionInvocation::binary(output.clone(), first.clone(), second.clone())
        }
        _ => unreachable!(),
    };
    let parts = entry
        .bind_resolved_invocation(operation_id, ExecutionTarget::DirectRuntime, invocation)
        .unwrap();
    SpecializedFunction::syntax_directed(
        parts,
        ResolvedOperationDescriptor::from_name(
            operation,
            entry.operation_contract(operation_id).unwrap().clone(),
        )
        .unwrap(),
        entry.id,
        ExecutionTarget::DirectRuntime,
        entry.implementation_memory_class(),
    )
    .unwrap()
}

#[test]
fn maintained_managed_parser_and_formatter_publish_atomically() {
    let text = canonical(SchemaBody::String, ValueDataDraft::String("42".into()));
    let parsed = count(0);
    let parser = bind(
        "number/parse-u64",
        "number/parse-u64",
        &parsed,
        &[text.clone()],
    );
    parser.instance().solve_result().unwrap();
    assert!(matches!(
        parsed.snapshot().unwrap().data(),
        ValueData::U64(42)
    ));
    let invalid = canonical(
        SchemaBody::String,
        ValueDataDraft::String("18446744073709551616".into()),
    );
    text.replace(&invalid.snapshot().unwrap()).unwrap();
    assert!(parser.instance().solve_result().is_err());
    assert!(matches!(
        parsed.snapshot().unwrap().data(),
        ValueData::U64(42)
    ));

    let value = ValueCell::from_exact(1.25_f64).unwrap();
    let precision = ValueCell::from_exact(9_u64).unwrap();
    let formatted = canonical(SchemaBody::String, ValueDataDraft::String(String::new()));
    let formatter = bind(
        "number/format",
        "NumberFormatScalarCanonical",
        &formatted,
        &[value, precision.clone()],
    );
    formatter.instance().solve_result().unwrap();
    assert!(
        matches!(formatted.snapshot().unwrap().data(), ValueData::String(text) if text.as_ref() == "1.250000000")
    );
    precision.replace(&count(13).snapshot().unwrap()).unwrap();
    assert!(formatter.instance().solve_result().is_err());
    assert!(
        matches!(formatted.snapshot().unwrap().data(), ValueData::String(text) if text.as_ref() == "1.250000000")
    );
}

#[test]
fn maintained_managed_dense_matrix_format_preserves_geometry_and_maximum_float_payloads() {
    let values = [1.25, 2.5, 5.0, -3.75, 0.0, 6.25];
    let source = ValueCell::from_exact(nalgebra::DMatrix::from_row_slice(2, 3, &values)).unwrap();
    let precision = ValueCell::from_exact(12_u64).unwrap();
    let output = canonical(
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: Box::new([DimensionExpr::Constant(2), DimensionExpr::Constant(3)]),
        },
        ValueDataDraft::Matrix(
            (0..6)
                .map(|_| ValueDataDraft::String(String::new()))
                .collect(),
        ),
    );
    let formatter = bind(
        "number/format",
        "NumberFormatMatrixCanonical",
        &output,
        &[source.clone(), precision],
    );
    formatter.instance().solve_result().unwrap();
    let rendered = output.snapshot().unwrap();
    let ValueData::Matrix(matrix) = rendered.data() else {
        panic!()
    };
    let SequenceView::String(elements) = matrix.elements() else {
        panic!()
    };
    assert_eq!(
        elements
            .iter()
            .map(|text| text.as_ref())
            .collect::<Vec<_>>(),
        values.map(|value| format!("{value:.12}"))
    );

    let maximum = ValueCell::from_exact(nalgebra::DMatrix::from_element(2, 3, -f64::MAX)).unwrap();
    source.replace(&maximum.snapshot().unwrap()).unwrap();
    formatter.instance().solve_result().unwrap();
    let rendered = output.snapshot().unwrap();
    let ValueData::Matrix(matrix) = rendered.data() else {
        panic!()
    };
    let SequenceView::String(elements) = matrix.elements() else {
        panic!()
    };
    assert!(
        elements
            .iter()
            .all(|text| text.as_ref() == format!("{:.12}", -f64::MAX))
    );
}

#[test]
fn maintained_managed_matrix_length_does_not_need_payload_construction_workspace() {
    let source = ValueCell::from_exact(nalgebra::DMatrix::from_element(128, 64, 7.0_f64)).unwrap();
    let output = count(0);
    let length = bind(
        "sequence/length",
        "sequence/length",
        &output,
        &[source.clone()],
    );
    length.instance().solve_result().unwrap();
    assert!(matches!(
        output.snapshot().unwrap().data(),
        ValueData::U64(8192)
    ));
    for (rows, columns) in [(3, 2), (0, 7)] {
        let replacement =
            ValueCell::from_exact(nalgebra::DMatrix::from_element(rows, columns, 9.0_f64)).unwrap();
        source.replace(&replacement.snapshot().unwrap()).unwrap();
        length.instance().solve_result().unwrap();
        assert!(matches!(
            output.snapshot().unwrap().data(),
            ValueData::U64(value) if *value == (rows * columns) as u64
        ));
    }

    let text = canonical(
        SchemaBody::String,
        ValueDataDraft::String("e\u{301}👩🏽‍💻🇺🇸".into()),
    );
    let text_length = bind("sequence/length", "sequence/length", &output, &[text]);
    text_length.instance().solve_result().unwrap();
    assert!(matches!(
        output.snapshot().unwrap().data(),
        ValueData::U64(3)
    ));
}

#[test]
fn maintained_managed_dense_numbers_keep_full_integer_width_and_float_kind() {
    for (source, precision, expected) in [
        (
            ValueCell::from_exact(u128::MAX).unwrap(),
            0,
            u128::MAX.to_string(),
        ),
        (
            ValueCell::from_exact(i128::MIN).unwrap(),
            0,
            i128::MIN.to_string(),
        ),
        (
            ValueCell::from_exact(1.25_f32).unwrap(),
            9,
            "1.250000000".into(),
        ),
    ] {
        let output = canonical(SchemaBody::String, ValueDataDraft::String(String::new()));
        let formatter = bind(
            "number/format",
            "NumberFormatScalarCanonical",
            &output,
            &[source, count(precision)],
        );
        formatter.instance().solve_result().unwrap();
        assert!(
            matches!(output.snapshot().unwrap().data(), ValueData::String(text) if text.as_ref() == expected)
        );
    }
}
