use crate::{DimensionExpr, Interpreter, SchemaBody, ValueData};

fn evaluate_selection(selector: &str) -> (Vec<usize>, Vec<f64>) {
    let source = format!("x := [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0]; x{selector}");
    let tree = mech_syntax::parser::parse(&source).unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap().unwrap();
    let SchemaBody::Matrix { dimensions, .. } = output.closed_schema_body().unwrap() else {
        panic!("expected a canonical matrix selection");
    };
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        panic!("expected closed matrix dimensions");
    };
    let rows = *rows as usize;
    let columns = *columns as usize;
    let row_major = output
        .matrix_elements()
        .unwrap()
        .unwrap()
        .into_iter()
        .map(|cell| match cell.snapshot().unwrap().data() {
            ValueData::F64(value) => value.to_f64(),
            _ => panic!("expected f64 matrix elements"),
        })
        .collect::<Vec<_>>();
    let column_major = (0..columns)
        .flat_map(|column| {
            let values = &row_major;
            (0..rows).map(move |row| values[row * columns + column])
        })
        .collect();
    (vec![rows, columns], column_major)
}

#[test]
fn explicit_all_selector_preserves_linear_and_scalar_axis_access() {
    assert_eq!(
        evaluate_selection("[:]"),
        (
            vec![9, 1],
            vec![1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]
        )
    );
    assert_eq!(
        evaluate_selection("[:,2]"),
        (vec![3, 1], vec![2.0, 5.0, 8.0])
    );
    assert_eq!(
        evaluate_selection("[2,:]"),
        (vec![1, 3], vec![4.0, 5.0, 6.0])
    );
}

#[test]
fn explicit_all_selector_preserves_range_access_on_both_axes() {
    assert_eq!(
        evaluate_selection("[:,1..=2]"),
        (vec![3, 2], vec![1.0, 4.0, 7.0, 2.0, 5.0, 8.0])
    );
    assert_eq!(
        evaluate_selection("[1..=2,:]"),
        (vec![2, 3], vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0])
    );
}

#[test]
fn explicit_all_selector_preserves_index_vector_access_on_both_axes() {
    assert_eq!(
        evaluate_selection("[:,[1 3]]"),
        (vec![3, 2], vec![1.0, 4.0, 7.0, 3.0, 6.0, 9.0])
    );
    assert_eq!(
        evaluate_selection("[[1 3],:]"),
        (vec![2, 3], vec![1.0, 7.0, 2.0, 8.0, 3.0, 9.0])
    );
}

#[test]
fn explicit_all_selector_preserves_logical_mask_access_on_both_axes() {
    assert_eq!(
        evaluate_selection("[:,[true false true]]"),
        (vec![3, 2], vec![1.0, 4.0, 7.0, 3.0, 6.0, 9.0])
    );
    assert_eq!(
        evaluate_selection("[[true false true],:]"),
        (vec![2, 3], vec![1.0, 7.0, 2.0, 8.0, 3.0, 9.0])
    );
}
