use crate::*;

// MatMul ---------------------------------------------------------------------

macro_rules! checked_mul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_matrix_mul(*$lhs, *$rhs, "scalar matrix product")?;
            *$out = next;
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! checked_matmul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let lhs = &*$lhs;
            let rhs = &*$rhs;
            let current = &*$out;
            if lhs.ncols() != rhs.nrows()
                || current.nrows() != lhs.nrows()
                || current.ncols() != rhs.ncols()
            {
                return Err(MechError::new(
                    DimensionMismatch {
                        dims: vec![
                            lhs.nrows(),
                            lhs.ncols(),
                            rhs.nrows(),
                            rhs.ncols(),
                            current.nrows(),
                            current.ncols(),
                        ],
                    },
                    None,
                )
                .with_compiler_loc());
            }

            let mut next = current.clone();
            for row in 0..lhs.nrows() {
                for column in 0..rhs.ncols() {
                    let mut sum = Zero::zero();
                    for inner in 0..lhs.ncols() {
                        let product = checked_matrix_mul(
                            lhs[(row, inner)],
                            rhs[(inner, column)],
                            "matrix-product multiplication",
                        )?;
                        sum = checked_matrix_add(sum, product, "matrix-product accumulation")?;
                    }
                    next[(row, column)] = sum;
                }
            }
            *$out = next;
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_matmul {
    ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
        impl_checked_matrix_binop!(
            $name,
            $type1,
            $type2,
            $out_type,
            checked_matmul_op,
            crate::product_contract
        );
    };
}

impl_checked_matrix_binop!(
    MatMulScalar,
    T,
    T,
    T,
    checked_mul_op,
    crate::product_contract
);
#[cfg(all(feature = "row_vector4", feature = "vector4", feature = "matrix1"))]
impl_matmul!(MatMulR4V4, RowVector4<T>, Vector4<T>, Matrix1<T>);
#[cfg(all(feature = "row_vector4", feature = "matrix4"))]
impl_matmul!(MatMulR4M4, RowVector4<T>, Matrix4<T>, RowVector4<T>);
#[cfg(all(feature = "row_vector4", feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulR4MD, RowVector4<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "row_vector3", feature = "vector3", feature = "matrix1"))]
impl_matmul!(MatMulR3V3, RowVector3<T>, Vector3<T>, Matrix1<T>);
#[cfg(all(feature = "row_vector3", feature = "matrix3"))]
impl_matmul!(MatMulR3M3, RowVector3<T>, Matrix3<T>, RowVector3<T>);
#[cfg(all(
    feature = "row_vector3",
    feature = "matrix3x2",
    feature = "row_vector2"
))]
impl_matmul!(MatMulR3M3x2, RowVector3<T>, Matrix3x2<T>, RowVector2<T>);
#[cfg(all(feature = "row_vector3", feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulR3MD, RowVector3<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "row_vector2", feature = "vector2", feature = "matrix1"))]
impl_matmul!(MatMulR2V2, RowVector2<T>, Vector2<T>, Matrix1<T>);
#[cfg(all(feature = "row_vector2", feature = "matrix2", feature = "row_vector2"))]
impl_matmul!(MatMulR2M2, RowVector2<T>, Matrix2<T>, RowVector2<T>);
#[cfg(all(
    feature = "row_vector2",
    feature = "matrix2x3",
    feature = "row_vector3"
))]
impl_matmul!(MatMulR2M2x3, RowVector2<T>, Matrix2x3<T>, RowVector3<T>);
#[cfg(all(feature = "row_vector2", feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulR2MD, RowVector2<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "row_vectord", feature = "vectord", feature = "matrix1"))]
impl_matmul!(MatMulRDVD, RowDVector<T>, DVector<T>, Matrix1<T>);
#[cfg(all(
    feature = "row_vectord",
    feature = "vectord",
    feature = "matrixd",
    not(feature = "matrix1")
))]
impl_matmul!(MatMulRDVDMD, RowDVector<T>, DVector<T>, DMatrix<T>);
#[cfg(all(feature = "row_vectord", feature = "matrixd"))]
impl_matmul!(MatMulRDMD, RowDVector<T>, DMatrix<T>, RowDVector<T>);

#[cfg(all(feature = "vector4", feature = "row_vector4", feature = "matrix4"))]
impl_matmul!(MatMulV4R4, Vector4<T>, RowVector4<T>, Matrix4<T>);
#[cfg(all(feature = "vector3", feature = "row_vector3", feature = "matrix3"))]
impl_matmul!(MatMulV3R3, Vector3<T>, RowVector3<T>, Matrix3<T>);
#[cfg(all(feature = "vector2", feature = "row_vector2", feature = "matrix2"))]
impl_matmul!(MatMulV2R2, Vector2<T>, RowVector2<T>, Matrix2<T>);

#[cfg(all(feature = "vectord", feature = "row_vectord", feature = "matrixd"))]
impl_matmul!(MatMulVDRD, DVector<T>, RowDVector<T>, DMatrix<T>);

#[cfg(all(feature = "matrix4", feature = "vector4"))]
impl_matmul!(MatMulM4V4, Matrix4<T>, Vector4<T>, Vector4<T>);
#[cfg(all(feature = "matrix4"))]
impl_matmul!(MatMulM4M4, Matrix4<T>, Matrix4<T>, Matrix4<T>);
#[cfg(all(feature = "matrix4", feature = "matrixd"))]
impl_matmul!(MatMulM4MD, Matrix4<T>, DMatrix<T>, DMatrix<T>);

#[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
impl_matmul!(MatMulM2M2x3, Matrix2<T>, Matrix2x3<T>, Matrix2x3<T>);
#[cfg(all(feature = "matrix2", feature = "matrix2"))]
impl_matmul!(MatMulM2M2, Matrix2<T>, Matrix2<T>, Matrix2<T>);
#[cfg(all(feature = "matrix2", feature = "vector2"))]
impl_matmul!(MatMulM2V2, Matrix2<T>, Vector2<T>, Vector2<T>);
#[cfg(all(feature = "matrix2", feature = "matrixd"))]
impl_matmul!(MatMulM2MD, Matrix2<T>, DMatrix<T>, DMatrix<T>);

#[cfg(feature = "matrix3")]
impl_matmul!(MatMulM3M3, Matrix3<T>, Matrix3<T>, Matrix3<T>);
#[cfg(all(feature = "matrix3", feature = "matrix3x2"))]
impl_matmul!(MatMulM2M3x2, Matrix3<T>, Matrix3x2<T>, Matrix3x2<T>);
#[cfg(all(feature = "matrix3", feature = "vector3"))]
impl_matmul!(MatMulM3V3, Matrix3<T>, Vector3<T>, Vector3<T>);
#[cfg(all(feature = "matrix3", feature = "matrixd"))]
impl_matmul!(MatMulM3MD, Matrix3<T>, DMatrix<T>, DMatrix<T>);

#[cfg(all(feature = "matrix1"))]
impl_matmul!(MatMulM1M1, Matrix1<T>, Matrix1<T>, Matrix1<T>);

#[cfg(all(feature = "matrix2x3", feature = "vector3", feature = "vector2"))]
impl_matmul!(MatMulM2x3V2, Matrix2x3<T>, Vector3<T>, Vector2<T>);
#[cfg(all(feature = "matrix2x3", feature = "matrix3"))]
impl_matmul!(MatMulM2x3M3, Matrix2x3<T>, Matrix3<T>, Matrix2x3<T>);
#[cfg(all(feature = "matrix2x3", feature = "matrix3x2", feature = "matrix2"))]
impl_matmul!(MatMulM2x3M3x2, Matrix2x3<T>, Matrix3x2<T>, Matrix2<T>);
#[cfg(all(feature = "matrix2x3", feature = "matrixd"))]
impl_matmul!(MatMulM2x3MD, Matrix2x3<T>, DMatrix<T>, DMatrix<T>);

#[cfg(all(feature = "matrix3x2", feature = "vector2", feature = "vector3"))]
impl_matmul!(MatMulM3x2V2, Matrix3x2<T>, Vector2<T>, Vector3<T>);
#[cfg(all(feature = "matrix3x2", feature = "matrix2"))]
impl_matmul!(MatMulM3x2M2, Matrix3x2<T>, Matrix2<T>, Matrix3x2<T>);
#[cfg(all(feature = "matrix3x2", feature = "matrix2x3", feature = "matrix3"))]
impl_matmul!(MatMulM3x2M2x3, Matrix3x2<T>, Matrix2x3<T>, Matrix3<T>);
#[cfg(all(feature = "matrix3x2", feature = "matrixd"))]
impl_matmul!(MatMulM3x2MD, Matrix3x2<T>, DMatrix<T>, DMatrix<T>);

#[cfg(feature = "matrixd")]
impl_matmul!(MatMulMDMD, DMatrix<T>, DMatrix<T>, DMatrix<T>);
#[cfg(all(feature = "matrixd", feature = "matrix3x2"))]
impl_matmul!(MatMulMDM3x2, DMatrix<T>, Matrix3x2<T>, DMatrix<T>);
#[cfg(all(feature = "matrixd", feature = "vectord"))]
impl_matmul!(MatMulMDVD, DMatrix<T>, DVector<T>, DVector<T>);
#[cfg(all(feature = "matrixd", feature = "row_vectord"))]
impl_matmul!(MatMulMDRD, DMatrix<T>, RowDVector<T>, DMatrix<T>);

#[cfg(feature = "source")]
pub struct MatrixMatMul;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for MatrixMatMul {
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
        let lhs = invocation.input(0).expect("validated matrix-product lhs");
        let rhs = invocation.input(1).expect("validated matrix-product rhs");
        let dimensions = match (lhs.matrix_descriptor()?, rhs.matrix_descriptor()?) {
            (None, None) => None,
            (Some(lhs), Some(rhs)) => {
                if lhs.cols != rhs.rows {
                    return Err(MechError::new(
                        DimensionMismatch {
                            dims: vec![lhs.rows, lhs.cols, rhs.rows, rhs.cols],
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                Some((lhs.rows, rhs.cols))
            }
            _ => {
                return Err(MechError::new(
                    FunctionArgumentTypeMismatch {
                        role: FunctionArgumentRole::Input(0),
                        expected: "two scalars or two compatible matrices".into(),
                        found: format!("{:?} and {:?}", lhs.representation(), rhs.representation()),
                    },
                    None,
                )
                .with_compiler_loc());
            }
        };
        let output_extents: Box<[u64]> = dimensions.map_or_else(
            || Vec::<u64>::new().into_boxed_slice(),
            |(rows, columns)| vec![rows as u64, columns as u64].into_boxed_slice(),
        );
        context.bind_resolved_runtime(
            mech_core::RuntimeBindingSelector::Operation(context.resolved_call()?.operation),
            mech_core::ExecutionTarget::DirectRuntime,
            vec![output_extents].into_boxed_slice(),
            &[lhs, rhs],
        )
    }
}

#[cfg(all(test, feature = "u8", feature = "matrixd"))]
mod checked_matmul_tests {
    use super::*;

    #[test]
    fn integer_matrix_product_rejects_overflow_and_retains_output() {
        let lhs = Ref::new(DMatrix::from_row_slice(1, 2, &[200_u8, 200]));
        let rhs = Ref::new(DMatrix::from_column_slice(2, 1, &[1_u8, 0]));
        let out = Ref::new(DMatrix::from_element(1, 1, 17_u8));
        let function = MatMulMDMD {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(out.borrow()[(0, 0)], 200);
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&function)?;
            *rhs.borrow_mut() = DMatrix::from_column_slice(2, 1, &[2, 2]);

            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MatrixArithmeticOverflow");
            assert_eq!(out.borrow()[(0, 0)], 200);
            *out.borrow_mut() = DMatrix::from_element(2, 2, 19);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(out.borrow().shape(), (1, 1));
        assert_eq!(out.borrow()[(0, 0)], 200);
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "matrix2",
    feature = "matrixd"
))]
mod canonical_port_tests {
    use super::*;

    #[test]
    fn fixed_and_dynamic_products_preserve_identity_shape_and_state() {
        let fixed_lhs = Ref::new(Matrix2::new(1.0_f64, 2.0, 3.0, 4.0));
        let fixed_rhs = Ref::new(Matrix2::new(5.0_f64, 6.0, 7.0, 8.0));
        let fixed_out = Ref::new(Matrix2::zeros());
        MatMulM2M2::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(fixed_out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(fixed_lhs, 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(fixed_rhs, 2, 2).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(*fixed_out.borrow(), Matrix2::new(19.0, 22.0, 43.0, 50.0));

        let lhs = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[1.0_f64, 2.0, 3.0, 4.0, 5.0, 6.0],
        ));
        let rhs = Ref::new(DMatrix::from_row_slice(
            3,
            2,
            &[7.0_f64, 8.0, 9.0, 10.0, 11.0, 12.0],
        ));
        let out = Ref::new(DMatrix::zeros(2, 2));
        let alias = out.clone();
        let output = ValueCell::from_exact_matrix_ref(out.clone(), 2, 2).unwrap();
        let function = MatMulMDMD::<f64>::new_invocation(FunctionInvocation::binary(
            output.clone(),
            ValueCell::from_exact_matrix_ref(lhs, 2, 3).unwrap(),
            ValueCell::from_exact_matrix_ref(rhs, 3, 2).unwrap(),
        ))
        .unwrap();
        function.solve_result().unwrap();
        assert_eq!(
            *out.borrow(),
            DMatrix::from_row_slice(2, 2, &[58.0, 64.0, 139.0, 154.0])
        );
        assert_eq!(
            function.reactive_output_cell_ids(),
            vec![output.reactive_cell_id()]
        );

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(function.as_ref())?;
            *out.borrow_mut() = DMatrix::from_element(1, 3, -1.0);
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert!(out.same_handle(&alias));
        assert_eq!(out.borrow().shape(), (2, 2));
    }

    #[test]
    fn dimension_failure_is_atomic() {
        let original = DMatrix::from_element(2, 2, 17.0_f64);
        let out = Ref::new(original.clone());
        let function = MatMulMDMD::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact_matrix_ref(out.clone(), 2, 2).unwrap(),
            ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::from_element(2, 3, 2.0_f64)), 2, 3)
                .unwrap(),
            ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::from_element(2, 2, 3.0_f64)), 2, 2)
                .unwrap(),
        ))
        .unwrap();
        assert_eq!(
            function.solve_result().unwrap_err().kind_name(),
            "DimensionMismatch"
        );
        assert_eq!(*out.borrow(), original);
    }
}
