use crate::*;

// Greater Than ---------------------------------------------------------------

#[cfg(feature = "matrix")]
macro_rules! concat_scalar_lhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i].concat(&(*$rhs));
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! concat_scalar_rhs_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$rhs).len() {
                (&mut (*$out))[i] = (*$lhs).concat(&(&(*$rhs))[i]);
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! concat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            for i in 0..(*$lhs).len() {
                (&mut (*$out))[i] = (&(*$lhs))[i].concat(&(&(*$rhs))[i]);
            }
        }
    };
}

macro_rules! concat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            (*$out) = (*$lhs).concat(&(*$rhs));
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! concat_mat_vec_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_col[i].concat(&rhs_deref[i]);
                }
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! concat_vec_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
                for i in 0..col.len() {
                    col[i] = lhs_deref[i].concat(&rhs_col[i]);
                }
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! concat_mat_row_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_row[i].concat(&rhs_deref[i]);
                }
            }
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! concat_row_mat_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let out_deref = &mut (*$out);
            let lhs_deref = &(*$lhs);
            let rhs_deref = &(*$rhs);
            for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
                for i in 0..row.len() {
                    row[i] = lhs_deref[i].concat(&rhs_row[i]);
                }
            }
        }
    };
}

impl_string_fxns!(Concat);

#[cfg(feature = "source")]
pub struct StringConcat;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for StringConcat {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let lhs = invocation.input(0).expect("validated concat lhs");
        let rhs = invocation.input(1).expect("validated concat rhs");
        let extents = [lhs, rhs]
            .into_iter()
            .map(|input| {
                input
                    .cell()?
                    .resolved_descriptor()?
                    .current_extents()
                    .map_err(MechError::from)
            })
            .collect::<MResult<Vec<_>>>()?;
        let output_extents = extents
            .iter()
            .find(|extent| !extent.is_empty())
            .cloned()
            .unwrap_or_else(|| Box::new([]));
        context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation(context.resolved_call()?.operation.id),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![output_extents].into_boxed_slice(),
            &[lhs, rhs],
        )
    }
}

#[cfg(all(test, feature = "string"))]
mod scalar_port_tests {
    use super::*;

    fn string_value(cell: &ValueCell) -> String {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::String(value) = snapshot.data() else {
            panic!("expected canonical string output")
        };
        value.to_string()
    }

    #[test]
    fn scalar_concat_uses_exact_canonical_ports_and_state() {
        let output = ValueCell::from_exact(String::new()).unwrap();
        let alias = output.clone();
        let function = ConcatSS::<String>::new_invocation(FunctionInvocation::binary(
            output.clone(),
            ValueCell::from_exact("left".to_string()).unwrap(),
            ValueCell::from_exact("-right".to_string()).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(string_value(&output), "left-right");
        assert!(output.same_cell(&alias));
        assert_eq!(
            function.reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            output.replace(&ValueCell::from_exact("changed".to_string())?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(string_value(&output), "left-right");

        assert!(
            ConcatSS::<String>::new_invocation(FunctionInvocation::unary(
                ValueCell::from_exact(String::new()).unwrap(),
                ValueCell::from_exact("wrong-layout".to_string()).unwrap(),
            ))
            .is_err()
        );
        assert!(
            ConcatSS::<String>::new_invocation(FunctionInvocation::binary(
                ValueCell::from_exact(String::new()).unwrap(),
                ValueCell::from_exact("left".to_string()).unwrap(),
                ValueCell::from_exact(1_usize).unwrap(),
            ))
            .is_err()
        );
    }

    #[cfg(feature = "source")]
    #[test]
    fn source_specialization_keeps_concat_behavior() {
        let invocation = SpecializationInvocation::from_cells(
            vec![
                ValueCell::from_exact("source".to_string()).unwrap(),
                ValueCell::from_exact("-path".to_string()).unwrap(),
            ]
            .into_boxed_slice(),
        );
        let mut context = SpecializationContext::for_invocation(&invocation, None).unwrap();
        let function = StringConcat {}
            .specialize_invocation(&invocation, &mut context)
            .unwrap()
            .into_parts()
            .0;
        function.solve_result().unwrap();
        assert!(matches!(
            function.output().snapshot().unwrap().data(),
            ValueData::String(value) if value.as_ref() == "source-path"
        ));
    }
}

#[cfg(all(test, feature = "string", feature = "matrix2", feature = "matrixd"))]
mod fixed_matrix_port_tests {
    use super::*;

    #[test]
    fn fixed_concat_preserves_storage_and_rejects_dynamic_inputs() {
        let lhs = Ref::new(Matrix2::new(
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ));
        let rhs = Ref::new(Matrix2::from_element("!".to_string()));
        let out = Ref::new(Matrix2::from_element(String::new()));
        let alias = out.clone();
        let function = ConcatM2M2::<String>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(lhs, 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(rhs, 2, 2).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        assert!(out.same_handle(&alias));
        assert_eq!(
            *out.borrow(),
            Matrix2::new(
                "a!".to_string(),
                "b!".to_string(),
                "c!".to_string(),
                "d!".to_string(),
            )
        );

        let wrong = Ref::new(DMatrix::from_element(2, 2, "x".to_string()));
        assert!(
            ConcatM2M2::<String>::new_invocation(FunctionInvocation::binary(
                ValueCell::from_exact_matrix_ref(
                    Ref::new(Matrix2::from_element(String::new())),
                    2,
                    2,
                )
                .unwrap(),
                ValueCell::from_exact_matrix_ref(wrong, 2, 2).unwrap(),
                ValueCell::from_exact_matrix_ref(
                    Ref::new(Matrix2::from_element("y".to_string())),
                    2,
                    2,
                )
                .unwrap(),
            ))
            .is_err()
        );
    }
}

#[cfg(all(
    test,
    feature = "string",
    feature = "matrixd",
    feature = "vectord",
    feature = "row_vectord"
))]
mod dynamic_matrix_port_tests {
    use super::*;

    fn matrix(values: &[&str]) -> DMatrix<String> {
        DMatrix::from_row_slice(
            2,
            2,
            &values
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn dynamic_broadcast_orientation_and_shape_rollback_are_canonical() {
        let matrix_ref = Ref::new(matrix(&["a", "b", "c", "d"]));
        let vector = Ref::new(DVector::from_vec(vec!["v1".to_string(), "v2".to_string()]));
        let row = Ref::new(RowDVector::from_vec(vec![
            "r1".to_string(),
            "r2".to_string(),
        ]));

        let vector_out = Ref::new(DMatrix::from_element(2, 2, String::new()));
        let vector_cell = ValueCell::from_exact_matrix_ref(vector_out.clone(), 2, 2).unwrap();
        let vector_function = ConcatMDVD::<String>::new_invocation(FunctionInvocation::binary(
            vector_cell.clone(),
            ValueCell::from_exact_matrix_ref(matrix_ref.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(vector, 2, 1).unwrap(),
        ))
        .unwrap();
        vector_function.solve_result().unwrap();
        assert_eq!(*vector_out.borrow(), matrix(&["av1", "bv1", "cv2", "dv2"]));

        let row_out = Ref::new(DMatrix::from_element(2, 2, String::new()));
        ConcatMDRD::<String>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(row_out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(matrix_ref, 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(row, 1, 2).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*row_out.borrow(), matrix(&["ar1", "br2", "cr1", "dr2"]));

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(vector_function.as_ref())?;
            *vector_out.borrow_mut() = DMatrix::from_element(1, 3, "changed".to_string());
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(vector_out.borrow().shape(), (2, 2));
        assert_eq!(*vector_out.borrow(), matrix(&["av1", "bv1", "cv2", "dv2"]));
    }
}
