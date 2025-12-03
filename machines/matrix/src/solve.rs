use crate::*;
use mech_core::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;
use nalgebra::ComplexField;

// Solve  ------------------------------------------------------------------

#[macro_export]
macro_rules! impl_binop_solve {
  ($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident, $feature_flag:expr) => {
    #[derive(Debug)]
    pub struct $struct_name<T> {
      pub lhs: Ref<$arg1_type>,
      pub rhs: Ref<$arg2_type>,
      pub out: Ref<$out_type>,
    }
    impl<T> MechFunctionFactory for $struct_name<T> 
    where
      #[cfg(feature = "compiler")]      T: Copy + Debug + Display + Clone + Sync + Send + 'static + PartialEq + PartialOrd + ComplexField + AsValueKind + Add<Output = T> + AddAssign +Sub<Output = T> + SubAssign +Mul<Output = T> + MulAssign +Div<Output = T> + DivAssign +Zero + One + ConstElem + CompileConst + AsValueKind,
      #[cfg(not(feature = "compiler"))] T: Copy + Debug + Display + Clone + Sync + Send + 'static + PartialEq + PartialOrd + ComplexField + AsValueKind + Add<Output = T> + AddAssign +Sub<Output = T> + SubAssign +Mul<Output = T> + MulAssign +Div<Output = T> + DivAssign +Zero + One,
      Ref<$out_type>: ToValue,
    {
      fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
          FunctionArgs::Binary(out, arg1, arg2) => {
            let lhs: Ref<$arg1_type> = unsafe { arg1.as_unchecked() }.clone();
            let rhs: Ref<$arg2_type> = unsafe { arg2.as_unchecked() }.clone();
            let out: Ref<$out_type> = unsafe { out.as_unchecked() }.clone();
            Ok(Box::new(Self {lhs, rhs, out }))
          },
          _ => Err(MechError2::new(
              IncorrectNumberOfArguments { expected: 2, found: args.len() }, 
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
    impl<T> MechFunctionImpl for $struct_name<T>
    where
      T: Copy + Debug + Display + Clone + Sync + Send + 'static + 
      PartialEq + PartialOrd + ComplexField +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + SubAssign +
      Mul<Output = T> + MulAssign +
      Div<Output = T> + DivAssign +
      Zero + One,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
          let lhs_ptr = self.lhs.as_ptr();
          let rhs_ptr = self.rhs.as_ptr();
          let out_ptr = self.out.as_mut_ptr();
          $op!(lhs_ptr,rhs_ptr,out_ptr);
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:#?}", self) }
    }   
    #[cfg(feature = "compiler")]
    impl<T> MechFunctionCompiler for $struct_name<T> 
    where
      T: ConstElem + CompileConst + AsValueKind
    {
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
        compile_binop!(name, self.out, self.lhs, self.rhs, ctx, $feature_flag);
      }
    }
  };
}

macro_rules! solve_op {
  ($a:expr, $b:expr, $out:expr) => {
    unsafe { *$out = (*$a).clone().lu().solve(&*$b).unwrap(); }
  };}

macro_rules! impl_solve {
  ($name:ident, $type1:ty, $type2:ty, $out_type:ty) => {
    impl_binop_solve!($name, $type1, $type2, $out_type, solve_op, FeatureFlag::Builtin(FeatureKind::Solve));
    register_fxn_descriptor!($name, f64, "f64");
  };
}

#[cfg(all(feature = "matrixd", feature = "vectord"))]
impl_solve!(MatrixSolveMDVD, DMatrix<T>, DVector<T>, DVector<T>);

macro_rules! impl_solve_match_arms {
  ($arg:expr, $($($matrix_kind:tt, $target_type:tt, $value_string:tt),+);+ $(;)?) => {
    match $arg {
      $(
        $(
          #[cfg(all(feature = $value_string, feature = "matrixd", feature = "vectord"))]
          (Value::$matrix_kind(Matrix::DMatrix(lhs)), Value::$matrix_kind(Matrix::DVector(rhs))) => {
            let (a_rows, a_cols) = lhs.borrow().shape();
            let (b_rows, b_cols) = rhs.borrow().shape();
            if b_cols != 1 {
              return Err(MechError2::new(
                DimensionMismatch { dims: vec![a_rows, a_cols, b_rows, b_cols] },
                Some("Right-hand side must be a vector (1 column)".to_string())
              ).with_compiler_loc());
            }
            if a_rows != b_rows {
              return Err(MechError2::new(
                DimensionMismatch { dims: vec![a_rows, a_cols, b_rows, b_cols] },
                Some("Matrix rows must match vector rows".to_string())
              ).with_compiler_loc());
            }
            Ok(Box::new(MatrixSolveMDVD { lhs: lhs.clone(), rhs: rhs.clone(), out: Ref::new(DVector::from_element(a_rows, $target_type::zero())) }))
          },
          #[cfg(feature = $value_string)]
          (Value::$matrix_kind(lhs), Value::$matrix_kind(rhs)) => {
            let lhs_shape = lhs.shape();
            let rhs_shape = rhs.shape();
            return Err(MechError2::new(
              DimensionMismatch { dims: vec![lhs_shape[0], lhs_shape[1], rhs_shape[0], rhs_shape[1]] },
              Some("Matrix multiplication is only implemented for `matrixd` and `vectord` types".to_string())
            ).with_compiler_loc());
          }
        )+
      )+
      (arg1,arg2) => Err(MechError2::new(
        UnhandledFunctionArgumentKind2 { arg: (arg1.kind(),arg2.kind()), fxn_name: stringify!($fxn).to_string() },
        Some("Unsupported types for matrix multiplication".to_string())
      ).with_compiler_loc()),
    }
  }
}

fn impl_solve_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  impl_solve_match_arms!(
    (lhs_value, rhs_value),
    MatrixF32,  f32,  "f32";
    MatrixF64,  f64,  "f64";
    //R64, MatrixR64, R64, "rational";
    //C64, MatrixC64, C64, "complex";
  )
}

impl_mech_binop_fxn!(MatrixSolve, impl_solve_fxn, "matrix/solve");
