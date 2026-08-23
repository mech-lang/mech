use crate::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use mech_core::*;
use nalgebra::ComplexField;
use num_traits::{One, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSolveSingular;

impl MechErrorKind for MatrixSolveSingular {
    fn name(&self) -> &str {
        "MatrixSolveSingular"
    }

    fn message(&self) -> String {
        "Matrix solve requires a nonsingular coefficient matrix".to_string()
    }
}

// Solve  ------------------------------------------------------------------

#[macro_export]
macro_rules! impl_binop_solve {
    ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident) => {
        #[derive(Debug)]
        pub struct $struct_name<T> {
            pub lhs: Ref<$arg1_type>,
            pub rhs: Ref<$arg2_type>,
            pub out: Ref<$out_type>,
        }
        impl<T> MechFunctionFactory for $struct_name<T>
        where
            #[cfg(feature = "semantic-compiler")]
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ComplexField
                + AsValueKind
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One
                + ConstElem
                + CompileConst
                + AsValueKind,
            #[cfg(not(feature = "semantic-compiler"))]
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ComplexField
                + AsValueKind
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One,
            Ref<$out_type>: ToValue,
            $arg1_type: FunctionRuntimeType,
            $arg2_type: FunctionRuntimeType,
            $out_type: FunctionRuntimeType,
        {
            const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
                <$out_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg1_type as FunctionRuntimeType>::REPRESENTATION,
                <$arg2_type as FunctionRuntimeType>::REPRESENTATION,
            );

            fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
                match args {
                    FunctionArgs::Binary(out, arg1, arg2) => {
                        let lhs: Ref<$arg1_type> =
                            arg1.try_function_ref(FunctionArgumentRole::Input(0))?;
                        let rhs: Ref<$arg2_type> =
                            arg2.try_function_ref(FunctionArgumentRole::Input(1))?;
                        let out: Ref<$out_type> =
                            out.try_function_ref(FunctionArgumentRole::Output)?;
                        Ok(Box::new(Self { lhs, rhs, out }))
                    }
                    _ => Err(MechError::new(
                        IncorrectNumberOfArguments {
                            expected: 2,
                            found: args.len(),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                }
            }
        }
        impl<T> MechFunctionImpl for $struct_name<T>
        where
            T: Copy
                + Debug
                + Display
                + Clone
                + Sync
                + Send
                + 'static
                + PartialEq
                + PartialOrd
                + ComplexField
                + Add<Output = T>
                + AddAssign
                + Sub<Output = T>
                + SubAssign
                + Mul<Output = T>
                + MulAssign
                + Div<Output = T>
                + DivAssign
                + Zero
                + One,
            Ref<$out_type>: ToValue,
        {
            fn solve_result(&self) -> MResult<()> {
                let lhs_ptr = self.lhs.as_ptr();
                let rhs_ptr = self.rhs.as_ptr();
                let out_ptr = self.out.as_mut_ptr();
                $op!(lhs_ptr, rhs_ptr, out_ptr);
                Ok(())
            }
            fn out(&self) -> LegacyValue {
                self.out.to_value()
            }
            fn to_string(&self) -> String {
                format!("{:#?}", self)
            }

            fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
                Ok(self.reactive_output_values())
            }
        }
        #[cfg(feature = "semantic-compiler")]
        impl<T> MechFunctionCompiler for $struct_name<T>
        where
            T: ConstElem + CompileConst + AsValueKind,
        {
            fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
                let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
                compile_binop!(name, self.out, self.lhs, self.rhs, ctx);
            }
        }
    };
}

macro_rules! solve_op {
    ($a:expr, $b:expr, $out:expr) => {
        unsafe {
            let solution = (*$a).clone().lu().solve(&*$b).ok_or_else(|| {
                MechError::new(MatrixSolveSingular, None).with_compiler_loc()
            })?;
            *$out = solution;
        }
    };
}

macro_rules! impl_solve {
    ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
        impl_binop_solve!($name, $type1, $type2, $out_type, solve_op);
    };
}

#[cfg(all(feature = "matrixd", feature = "vectord"))]
impl_solve!(MatrixSolveMDVD, DMatrix<T>, DVector<T>, DVector<T>);

#[cfg(feature = "matrixd")]
impl_solve!(MatrixSolveMDMD, DMatrix<T>, DMatrix<T>, DMatrix<T>);

// Keep fixed-shape source mathematical. The semantic compiler sees the
// ordinary solve operation and compute backends can scalarize it without the
// program spelling out an inverse or splitting a matrix right-hand side into
// columns.
#[cfg(all(feature = "matrix2", feature = "matrix2x3"))]
impl_solve!(MatrixSolveM2M2x3, Matrix2<T>, Matrix2x3<T>, Matrix2x3<T>);

#[cfg(all(test, feature = "f64", feature = "matrixd", feature = "vectord"))]
mod tests {
    use super::*;

    #[test]
    fn singular_matrix_is_a_structured_error_on_reactive_resolve() {
        let lhs = Ref::new(DMatrix::identity(2, 2));
        let rhs = Ref::new(DVector::from_vec(vec![3.0, 4.0]));
        let out = Ref::new(DVector::from_element(2, -1.0));
        let function = MatrixSolveMDVD {
            lhs: lhs.clone(),
            rhs,
            out: out.clone(),
        };

        function.solve_result().unwrap();
        let previous = out.borrow().clone();
        *lhs.borrow_mut() = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 4.0]);

        let error = function.solve_result().unwrap_err();
        assert_eq!(error.kind_name(), "MatrixSolveSingular");
        assert_eq!(*out.borrow(), previous);
    }

    #[test]
    fn matrix_right_hand_side_is_solved_in_one_operation() {
        let lhs = Ref::new(DMatrix::from_row_slice(2, 2, &[4.0, 1.0, 2.0, 3.0]));
        let rhs = Ref::new(DMatrix::from_row_slice(
            2,
            3,
            &[9.0, 1.0, 5.0, 8.0, 7.0, 2.0],
        ));
        let out = Ref::new(DMatrix::zeros(2, 3));
        let function = MatrixSolveMDMD {
            lhs: lhs.clone(),
            rhs: rhs.clone(),
            out: out.clone(),
        };

        function.solve_result().unwrap();

        let residual = lhs.borrow().clone() * out.borrow().clone() - rhs.borrow().clone();
        assert!(residual.norm() < 1.0e-12);
    }
}

#[cfg(feature = "source")]
macro_rules! impl_solve_match_arms {
  ($arg:expr, $($($matrix_kind:tt, $target_type:tt, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          #[cfg(all(feature = $value_string, feature = "matrixd"))]
          (LegacyValue::$matrix_kind(Matrix::DMatrix(lhs)), LegacyValue::$matrix_kind(Matrix::DMatrix(rhs))) => {
            let (a_rows, a_cols) = lhs.borrow().shape();
            let (b_rows, b_cols) = rhs.borrow().shape();
            if a_rows != a_cols || a_rows != b_rows {
              return Err(MechError::new(
                DimensionMismatch { dims: vec![a_rows, a_cols, b_rows, b_cols] },
                Some("Matrix solve requires a square coefficient matrix whose rows match the right-hand side".to_string())
              ).with_compiler_loc());
            }
            Ok(Box::new(MatrixSolveMDMD {
              lhs: lhs.clone(),
              rhs: rhs.clone(),
              out: Ref::new(DMatrix::from_element(a_rows, b_cols, $target_type::zero())),
            }))
          },
          #[cfg(all(feature = $value_string, feature = "matrix2", feature = "matrix2x3"))]
          (LegacyValue::$matrix_kind(Matrix::Matrix2(lhs)), LegacyValue::$matrix_kind(Matrix::Matrix2x3(rhs))) => {
            Ok(Box::new(MatrixSolveM2M2x3 {
              lhs: lhs.clone(),
              rhs: rhs.clone(),
              out: Ref::new(Matrix2x3::from_element($target_type::zero())),
            }))
          },
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
          (LegacyValue::$matrix_kind(Matrix::DMatrix(lhs)), LegacyValue::$matrix_kind(Matrix::DVector(rhs))) => {
            let (a_rows, a_cols) = lhs.borrow().shape();
            let (b_rows, b_cols) = rhs.borrow().shape();
            if b_cols != 1 {
              return Err(MechError::new(
                DimensionMismatch { dims: vec![a_rows, a_cols, b_rows, b_cols] },
                Some("Right-hand side must be a vector (1 column)".to_string())
              ).with_compiler_loc());
            }
            if a_rows != a_cols || a_rows != b_rows {
              return Err(MechError::new(
                DimensionMismatch { dims: vec![a_rows, a_cols, b_rows, b_cols] },
                Some("Matrix solve requires a square coefficient matrix whose rows match the right-hand side".to_string())
              ).with_compiler_loc());
            }
            Ok(Box::new(MatrixSolveMDVD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DVector::from_element(a_rows, $target_type::zero())) }))
          },
          #[cfg(feature = $value_string)]
          (LegacyValue::$matrix_kind(lhs), LegacyValue::$matrix_kind(rhs)) => {
            let lhs_shape = lhs.shape();
            let rhs_shape = rhs.shape();
            return Err(MechError::new(
              DimensionMismatch { dims: vec![lhs_shape[0], lhs_shape[1], rhs_shape[0], rhs_shape[1]] },
              Some("Matrix solve is not implemented for this pair of matrix representations".to_string())
            ).with_compiler_loc());
          }
        )+
      )+
      (arg1,arg2) => Err(MechError::new(
        UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: stringify!($fxn).to_string() },
        Some("Unsupported types for matrix solve".to_string())
      ).with_compiler_loc()),
    }
  }
}

#[cfg(feature = "source")]
fn impl_solve_fxn(lhs_value: LegacyValue, rhs_value: LegacyValue) -> MResult<Box<dyn MechFunction>> {
    impl_solve_match_arms!(
      (lhs_value, rhs_value),
      MatrixF32,  f32,  "f32";
      MatrixF64,  f64,  "f64";
      //R64, MatrixR64, R64, "rational";
      //C64, MatrixC64, C64, "complex";
    )
}

#[cfg(feature = "source")]
impl_mech_binop_fxn!(MatrixSolve, impl_solve_fxn, "matrix/solve");
