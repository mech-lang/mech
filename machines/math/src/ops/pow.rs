#[macro_use]
use crate::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Pow ------------------------------------------------------------------------

macro_rules! pow_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      *$out = (&*$lhs).pow(*$rhs);
    }
  };
}

macro_rules! pow_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$lhs).len() {
        (&mut *$out)[i] = (&*$lhs)[i].pow((&*$rhs)[i]);
      }
    }
  };
}

macro_rules! pow_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$lhs).len() {
        (&mut *$out)[i] = (&*$lhs)[i].pow(*$rhs);
      }
    }
  };
}

macro_rules! pow_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      for i in 0..(&*$rhs).len() {
        (&mut *$out)[i] = (*$lhs).pow((&*$rhs)[i]);
      }
    }
  };
}

macro_rules! pow_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_col[i].pow(rhs_deref[i]);
        }
      }
    }
  };}

macro_rules! pow_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_deref[i].pow(rhs_col[i]);
        }
      }
    }
  };}

macro_rules! pow_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_row[i].pow(rhs_deref[i]);
        }
      }
    }
  };}

macro_rules! pow_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_deref[i].pow(rhs_row[i]);
        }
      }
    }
  };} 
  
#[macro_export]
macro_rules! impl_powop {
($struct_name:ident, $arg1_type:ty, $arg2_type:ty, $out_type:ty, $op:ident, $feature_flag:expr) => {
  #[derive(Debug)]
  struct $struct_name<T> {
    lhs: Ref<$arg1_type>,
    rhs: Ref<$arg2_type>,
    out: Ref<$out_type>,
  }
  impl<T> MechFunctionFactory for $struct_name<T>
  where
    T: Copy + Debug + Clone + Sync + Send + 'static + 
    PartialEq + PartialOrd +
    Add<Output = T> + AddAssign +
    Sub<Output = T> + SubAssign +
    Mul<Output = T> + MulAssign +
    Div<Output = T> + DivAssign +
    Pow<T, Output = T> +
    CompileConst + ConstElem + AsValueKind +
    Zero + One,
    Ref<$out_type>: ToValue
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
            IncorrectNumberOfArguments { expected: 2, found: 0 },
            None
          ).with_compiler_loc()
        ),
      }
    }
  }
  impl<T> MechFunctionImpl for $struct_name<T>
  where
    T: Copy + Debug + Clone + Sync + Send + 'static + 
    PartialEq + PartialOrd +
    Add<Output = T> + AddAssign +
    Sub<Output = T> + SubAssign +
    Mul<Output = T> + MulAssign +
    Div<Output = T> + DivAssign +
    Pow<T, Output = T> +
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
    T: CompileConst + ConstElem + AsValueKind
  {
    fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
      let name = format!("{}<{}>", stringify!($struct_name), T::as_value_kind());
      compile_binop!(name, self.out, self.lhs, self.rhs, ctx, $feature_flag);
    }
  }};}

#[macro_export]
macro_rules! impl_math_fxns_pow {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_powop);
  }}

impl_math_fxns_pow!(Pow);

#[cfg(all(feature = "rational", feature = "i32"))]
#[derive(Debug)]
pub struct PowRational {
  pub lhs: Ref<R64>,
  pub rhs: Ref<i32>,
  pub out: Ref<R64>,
}
#[cfg(all(feature = "rational", feature = "i32"))]
impl MechFunctionFactory for PowRational {
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let lhs: Ref<R64> = unsafe { arg1.as_unchecked() }.clone();
        let rhs: Ref<i32> = unsafe { arg2.as_unchecked() }.clone();
        let out: Ref<R64> = unsafe { out.as_unchecked() }.clone();
        Ok(Box::new(Self {lhs, rhs, out }))
      },
      _ => Err(MechError2::new(
          IncorrectNumberOfArguments { expected: 2, found: 0 },
          None
        ).with_compiler_loc()
      ),
    }
  }
}
#[cfg(all(feature = "rational", feature = "i32"))]
impl MechFunctionImpl for PowRational {
  fn solve(&self) {
    let lhs_ptr = self.lhs.as_ptr();
    let rhs_ptr = self.rhs.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe {
      (*out_ptr).0 = (*lhs_ptr).0.pow((*rhs_ptr));
    }
  }
  fn out(&self) -> Value { self.out.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "rational", feature = "i32", feature = "compiler"))]
impl MechFunctionCompiler for PowRational 
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("PowRational<{}>", R64::as_value_kind());
    compile_binop!(name, self.out, self.lhs, self.rhs, ctx, FeatureFlag::Builtin(FeatureKind::Pow) );
  }
}

fn impl_pow_fxn(lhs_value: Value, rhs_value: Value) -> MResult<Box<dyn MechFunction>> {
  match (&lhs_value, &rhs_value) {
    #[cfg(all(feature = "rational", feature = "i32"))]
    (Value::R64(lhs), Value::I32(rhs)) => {
      return Ok(Box::new(PowRational {
        lhs: lhs.clone(),
        rhs: rhs.clone(),
        out: Ref::new(R64::default()),
      }));
    },
    _ => (),
  }
  impl_binop_match_arms!(
    Pow,
    register_fxn_descriptor_inner,
    (lhs_value, rhs_value),
    U8,   u8,   "u8";
    U16,  u16,  "u16";
    U32,  u32,  "u32";
    F32,  f32,  "f32";
    F64,  f64,  "f64";
  )
}

impl_mech_binop_fxn!(MathPow,impl_pow_fxn,"math/pow");