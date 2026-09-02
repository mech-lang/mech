use crate::*;

// MatMul ---------------------------------------------------------------------

macro_rules! checked_mul_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let next = checked_matrix_mul(*$lhs, *$rhs, "scalar product")?;
            *$out = next;
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! checked_dot_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
        unsafe {
            let lhs = &*$lhs;
            let rhs = &*$rhs;
            if lhs.nrows() != rhs.nrows() || lhs.ncols() != rhs.ncols() {
                return Err(MechError::new(
                    DimensionMismatch {
                        dims: vec![lhs.nrows(), lhs.ncols(), rhs.nrows(), rhs.ncols()],
                    },
                    None,
                )
                .with_compiler_loc());
            }
            let mut next = Zero::zero();
            for (lhs, rhs) in lhs.iter().zip(rhs.iter()) {
                let product = checked_matrix_mul(*lhs, *rhs, "dot-product multiplication")?;
                next = checked_matrix_add(next, product, "dot-product accumulation")?;
            }
            *$out = next;
        }
    };
}

#[cfg(feature = "matrix")]
macro_rules! impl_dot {
    ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
        impl_checked_matrix_binop!(
            $name,
            $type1,
            $type2,
            $out_type,
            checked_dot_op,
            product_contract
        );
    };
}

impl_checked_matrix_binop!(DotScalar, T, T, T, checked_mul_op, product_contract);
#[cfg(all(feature = "row_vector2", feature = "row_vector2"))]
impl_dot!(DotR2R2, RowVector2<T>, RowVector2<T>, T);
#[cfg(all(feature = "vector2", feature = "vector2"))]
impl_dot!(DotV2V2, Vector2<T>, Vector2<T>, T);

#[cfg(all(feature = "row_vector3", feature = "row_vector3"))]
impl_dot!(DotR3R3, RowVector3<T>, RowVector3<T>, T);
#[cfg(all(feature = "vector3", feature = "vector3"))]
impl_dot!(DotV3V3, Vector3<T>, Vector3<T>, T);

#[cfg(all(feature = "row_vector4", feature = "row_vector4"))]
impl_dot!(DotR4R4, RowVector4<T>, RowVector4<T>, T);
#[cfg(all(feature = "vector4", feature = "vector4"))]
impl_dot!(DotV4V4, Vector4<T>, Vector4<T>, T);

#[cfg(all(feature = "matrix1", feature = "matrix1"))]
impl_dot!(DotM1M1, Matrix1<T>, Matrix1<T>, T);
#[cfg(all(feature = "matrix2", feature = "matrix2"))]
impl_dot!(DotM2M2, Matrix2<T>, Matrix2<T>, T);
#[cfg(all(feature = "matrix3", feature = "matrix3"))]
impl_dot!(DotM3M3, Matrix3<T>, Matrix3<T>, T);
#[cfg(all(feature = "matrix4", feature = "matrix4"))]
impl_dot!(DotM4M4, Matrix4<T>, Matrix4<T>, T);

#[cfg(all(feature = "matrixd", feature = "matrixd"))]
impl_dot!(DotMDMD, DMatrix<T>, DMatrix<T>, T);
#[cfg(all(feature = "vectord", feature = "vectord"))]
impl_dot!(DotVDVD, DVector<T>, DVector<T>, T);
#[cfg(all(feature = "row_vectord", feature = "row_vectord"))]
impl_dot!(DotRDRD, RowDVector<T>, RowDVector<T>, T);

#[cfg(feature = "source")]
pub struct MatrixDot;

#[cfg(feature = "source")]
impl CanonicalFunctionSpecializer for MatrixDot {
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
        let lhs = invocation.input(0).expect("validated dot lhs");
        let rhs = invocation.input(1).expect("validated dot rhs");
        context.bind_runtime_factory_derived_output("Dot", None, &[lhs, rhs])
    }
}

#[cfg(all(test, feature = "u8", feature = "vector2"))]
mod checked_dot_tests {
    use super::*;

    #[test]
    fn integer_dot_rejects_overflow_and_retains_output() {
        let lhs = Ref::new(Vector2::new(200_u8, 200));
        let rhs = Ref::new(Vector2::new(1_u8, 0));
        let out = Ref::new(17_u8);
        let function = DotV2V2 {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();
        assert_eq!(*out.borrow(), 200);
        with_reactive_journal_participant(|mut participant| {
            participant.capture_function_state(&function)?;
            *rhs.borrow_mut() = Vector2::new(2, 2);

            let error = function.solve_result().unwrap_err();
            assert_eq!(error.kind_name(), "MatrixArithmeticOverflow");
            assert_eq!(*out.borrow(), 200);
            *out.borrow_mut() = 19;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(*out.borrow(), 200);
    }
}

#[cfg(all(
    test,
    feature = "runtime",
    feature = "f64",
    feature = "vector2",
    feature = "matrixd"
))]
mod canonical_port_tests {
    use super::*;

    fn f64_value(cell: &ValueCell) -> f64 {
        let snapshot = cell.snapshot().unwrap();
        let ValueData::F64(value) = snapshot.data() else {
            panic!("expected f64 dot output")
        };
        value.to_f64()
    }

    #[test]
    fn scalar_fixed_and_dynamic_dot_products_use_exact_ports() {
        let scalar_out = ValueCell::from_exact(0.0_f64).unwrap();
        let scalar_alias = scalar_out.clone();
        let scalar = DotScalar::<f64>::new_invocation(FunctionInvocation::binary(
            scalar_out.clone(),
            ValueCell::from_exact(2.5_f64).unwrap(),
            ValueCell::from_exact(4.0_f64).unwrap(),
        ))
        .unwrap();
        scalar.solve_result().unwrap();
        assert_eq!(f64_value(&scalar_out), 10.0);
        assert!(scalar_out.same_cell(&scalar_alias));

        let fixed_out = ValueCell::from_exact(0.0_f64).unwrap();
        DotV2V2::<f64>::new_invocation(FunctionInvocation::binary(
            fixed_out.clone(),
            ValueCell::from_exact_matrix_ref(Ref::new(Vector2::new(1.0, 2.0)), 2, 1).unwrap(),
            ValueCell::from_exact_matrix_ref(Ref::new(Vector2::new(3.0, 4.0)), 2, 1).unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(f64_value(&fixed_out), 11.0);

        let dynamic_out = ValueCell::from_exact(0.0_f64).unwrap();
        DotMDMD::<f64>::new_invocation(FunctionInvocation::binary(
            dynamic_out.clone(),
            ValueCell::from_exact_matrix_ref(
                Ref::new(DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 3.0, 4.0])),
                2,
                2,
            )
            .unwrap(),
            ValueCell::from_exact_matrix_ref(
                Ref::new(DMatrix::from_row_slice(2, 2, &[2.0, 0.0, 1.0, 2.0])),
                2,
                2,
            )
            .unwrap(),
        ))
        .unwrap()
        .solve_result()
        .unwrap();
        assert_eq!(f64_value(&dynamic_out), 13.0);

        with_reactive_journal_participant(|mut participant| -> MResult<()> {
            participant.capture_function_state(scalar.as_ref())?;
            scalar_out.replace(&ValueCell::from_exact(-1.0_f64)?.snapshot()?)?;
            participant.preflight_restore_before()?;
            participant.apply_restore_before();
            Ok(())
        })
        .unwrap();
        assert_eq!(f64_value(&scalar_out), 10.0);
    }

    #[test]
    fn dot_rejects_wrong_exact_type_and_layout() {
        let rhs = ValueCell::from_exact_matrix_ref(Ref::new(Vector2::new(1.0, 2.0)), 2, 1)
            .unwrap();
        assert!(DotV2V2::<f64>::new_invocation(FunctionInvocation::binary(
            ValueCell::from_exact(0.0_f64).unwrap(),
            ValueCell::from_exact(1.0_f64).unwrap(),
            rhs.clone(),
        ))
        .is_err());
        assert!(DotV2V2::<f64>::new_invocation(FunctionInvocation::unary(
            ValueCell::from_exact(0.0_f64).unwrap(),
            rhs,
        ))
        .is_err());
    }
}
