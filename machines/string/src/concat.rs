use crate::*;
#[cfg(all(feature = "matrix", feature = "source"))]
use mech_core::matrix::Matrix;

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
fn impl_concat_fxn(lhs_value: LegacyValue, rhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_binop_match_arms!(
      Concat,
      (lhs_value, rhs_value),
      String, String, "string";
    )
}

#[cfg(feature = "source")]
impl_mech_binop_fxn!(StringConcat, impl_concat_fxn, "string/concat");

#[cfg(all(test, feature = "string"))]
mod scalar_port_tests {
    use super::*;

    fn binary_args<L, R, O>(out: &Ref<O>, lhs: &Ref<L>, rhs: &Ref<R>) -> FunctionArgs
    where
        Ref<L>: ToValue,
        Ref<R>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value())
    }

    #[test]
    fn scalar_legacy_and_invocation_factories_are_equivalent() {
        let lhs = Ref::new("left".to_string());
        let rhs = Ref::new("-right".to_string());
        let legacy_out = Ref::new(String::new());
        let invocation_out = Ref::new(String::new());
        let legacy = ConcatSS::<String>::new(binary_args(&legacy_out, &lhs, &rhs)).unwrap();
        let invocation = ConcatSS::<String>::new_invocation(
            binary_args(&invocation_out, &lhs, &rhs).into(),
        )
        .unwrap();

        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(&*legacy_out.borrow(), "left-right");
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
    }

    #[test]
    fn scalar_state_restores_the_same_output_cell_and_identity() {
        let lhs = Ref::new("a".to_string());
        let rhs = Ref::new("b".to_string());
        let out = Ref::new(String::new());
        let out_alias = out.clone();
        let function =
            ConcatSS::<String>::new_invocation(binary_args(&out, &lhs, &rhs).into()).unwrap();
        function.solve_result().unwrap();
        assert_eq!(
            function.reactive_output_cell_ids(),
            function.out().reactive_root_cell_ids()
        );

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*function)?;
            *out.borrow_mut() = "mutated".to_string();
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();

        assert!(out.same_handle(&out_alias));
        assert_eq!(&*out.borrow(), "ab");
    }

    #[test]
    fn distinct_equal_scalar_outputs_keep_distinct_identity() {
        let lhs = Ref::new("a".to_string());
        let rhs = Ref::new("b".to_string());
        let first_out = Ref::new(String::new());
        let second_out = Ref::new(String::new());
        let first = ConcatSS::<String>::new_invocation(
            binary_args(&first_out, &lhs, &rhs).into(),
        )
        .unwrap();
        let second = ConcatSS::<String>::new_invocation(
            binary_args(&second_out, &lhs, &rhs).into(),
        )
        .unwrap();
        first.solve_result().unwrap();
        second.solve_result().unwrap();

        assert_eq!(*first_out.borrow(), *second_out.borrow());
        assert!(!first_out.same_handle(&second_out));
        assert_ne!(
            first.reactive_output_cell_ids(),
            second.reactive_output_cell_ids()
        );
    }

    #[test]
    fn scalar_factory_rejects_the_wrong_argument_layout() {
        let lhs = Ref::new("a".to_string());
        let out = Ref::new(String::new());
        let error = ConcatSS::<String>::new_invocation(
            FunctionArgs::Unary(out.to_value(), lhs.to_value()).into(),
        )
        .err()
        .expect("unary layout must be rejected");

        assert_eq!(error.kind_name(), "IncorrectNumberOfArguments");
        let error = error.kind_as::<IncorrectNumberOfArguments>().unwrap();
        assert_eq!((error.expected, error.found), (2, 1));
    }

    #[cfg(feature = "source")]
    #[test]
    fn source_specialization_keeps_existing_concat_behavior() {
        let lhs = Ref::new("source".to_string());
        let rhs = Ref::new("-path".to_string());
        let function = StringConcat {}
            .specialize(&[lhs.to_value(), rhs.to_value()])
            .unwrap();
        function.solve_result().unwrap();
        let LegacyValue::String(out) = function.out() else {
            panic!("expected string output")
        };
        assert_eq!(&*out.borrow(), "source-path");
    }
}

#[cfg(all(test, feature = "string", feature = "matrix2", feature = "matrixd"))]
mod fixed_matrix_port_tests {
    use super::*;

    fn binary_args<L, R, O>(out: &Ref<O>, lhs: &Ref<L>, rhs: &Ref<R>) -> FunctionArgs
    where
        Ref<L>: ToValue,
        Ref<R>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value())
    }

    #[test]
    fn fixed_matrix_factories_match_and_restore_exact_contents() {
        let lhs = Ref::new(Matrix2::new(
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ));
        let rhs = Ref::new(Matrix2::new(
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ));
        let empty = || Matrix2::from_element(String::new());
        let legacy_out = Ref::new(empty());
        let invocation_out = Ref::new(empty());
        let legacy =
            ConcatM2M2::<String>::new(binary_args(&legacy_out, &lhs, &rhs)).unwrap();
        let invocation = ConcatM2M2::<String>::new_invocation(
            binary_args(&invocation_out, &lhs, &rhs).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        let expected = invocation_out.borrow().clone();

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*invocation)?;
            *invocation_out.borrow_mut() = Matrix2::from_element("changed".to_string());
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*invocation_out.borrow(), expected);
    }

    #[test]
    fn fixed_factory_rejects_dynamic_matrix_storage() {
        let wrong = Ref::new(DMatrix::from_element(2, 2, "x".to_string()));
        let rhs = Ref::new(Matrix2::from_element("y".to_string()));
        let out = Ref::new(Matrix2::from_element(String::new()));
        let error = ConcatM2M2::<String>::new_invocation(
            binary_args(&out, &wrong, &rhs).into(),
        )
        .err()
        .expect("dynamic storage must not satisfy a Matrix2 port");
        assert_eq!(error.kind_name(), "FunctionArgumentTypeMismatch");
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

    fn binary_args<L, R, O>(out: &Ref<O>, lhs: &Ref<L>, rhs: &Ref<R>) -> FunctionArgs
    where
        Ref<L>: ToValue,
        Ref<R>: ToValue,
        Ref<O>: ToValue,
    {
        FunctionArgs::Binary(out.to_value(), lhs.to_value(), rhs.to_value())
    }

    fn matrix(values: &[&str]) -> DMatrix<String> {
        DMatrix::from_row_slice(
            2,
            2,
            &values.iter().map(|value| (*value).to_string()).collect::<Vec<_>>(),
        )
    }

    #[test]
    fn scalar_and_dynamic_matrix_combinations_are_exact() {
        let scalar = Ref::new("s".to_string());
        let matrix_ref = Ref::new(matrix(&["a", "b", "c", "d"]));

        let scalar_lhs_out = Ref::new(DMatrix::from_element(2, 2, String::new()));
        ConcatSMD::<String>::new_invocation(
            binary_args(&scalar_lhs_out, &scalar, &matrix_ref).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*scalar_lhs_out.borrow(), matrix(&["sa", "sb", "sc", "sd"]));

        let scalar_rhs_out = Ref::new(DMatrix::from_element(2, 2, String::new()));
        ConcatMDS::<String>::new_invocation(
            binary_args(&scalar_rhs_out, &matrix_ref, &scalar).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*scalar_rhs_out.borrow(), matrix(&["as", "bs", "cs", "ds"]));
    }

    #[test]
    fn dynamic_matrix_factories_match_and_restore_shape() {
        let lhs = Ref::new(matrix(&["a", "b", "c", "d"]));
        let rhs = Ref::new(matrix(&["1", "2", "3", "4"]));
        let legacy_out = Ref::new(DMatrix::from_element(2, 2, String::new()));
        let invocation_out = Ref::new(DMatrix::from_element(2, 2, String::new()));
        let legacy = ConcatMDMD::<String>::new(binary_args(&legacy_out, &lhs, &rhs)).unwrap();
        let invocation = ConcatMDMD::<String>::new_invocation(
            binary_args(&invocation_out, &lhs, &rhs).into(),
        )
        .unwrap();
        legacy.solve_result().unwrap();
        invocation.solve_result().unwrap();
        assert_eq!(*legacy_out.borrow(), *invocation_out.borrow());
        let expected = invocation_out.borrow().clone();
        assert_eq!(
            invocation.reactive_output_cell_ids(),
            invocation.out().reactive_root_cell_ids()
        );

        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&*invocation)?;
            *invocation_out.borrow_mut() = DMatrix::from_element(1, 3, "changed".to_string());
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(invocation_out.borrow().shape(), (2, 2));
        assert_eq!(*invocation_out.borrow(), expected);
    }

    #[test]
    fn dynamic_vector_and_row_broadcasts_preserve_orientation() {
        let matrix_ref = Ref::new(matrix(&["a", "b", "c", "d"]));
        let vector = Ref::new(DVector::from_vec(vec!["v1".to_string(), "v2".to_string()]));
        let row = Ref::new(RowDVector::from_vec(vec!["r1".to_string(), "r2".to_string()]));
        let out = || Ref::new(DMatrix::from_element(2, 2, String::new()));

        let matrix_vector = out();
        ConcatMDVD::<String>::new_invocation(
            binary_args(&matrix_vector, &matrix_ref, &vector).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(
            *matrix_vector.borrow(),
            matrix(&["av1", "bv1", "cv2", "dv2"])
        );

        let vector_matrix = out();
        ConcatVDMD::<String>::new_invocation(
            binary_args(&vector_matrix, &vector, &matrix_ref).into(),
        )
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(
            *vector_matrix.borrow(),
            matrix(&["v1a", "v1b", "v2c", "v2d"])
        );

        let matrix_row = out();
        ConcatMDRD::<String>::new_invocation(binary_args(&matrix_row, &matrix_ref, &row).into())
            .unwrap()
            .solve_result()
            .unwrap();
        assert_eq!(*matrix_row.borrow(), matrix(&["ar1", "br2", "cr1", "dr2"]));

        let row_matrix = out();
        ConcatRDMD::<String>::new_invocation(binary_args(&row_matrix, &row, &matrix_ref).into())
            .unwrap()
            .solve_result()
            .unwrap();
        assert_eq!(*row_matrix.borrow(), matrix(&["r1a", "r2b", "r1c", "r2d"]));
    }
}
