#[macro_use]
use crate::*;
use num_traits::*;
#[cfg(feature = "matrix")]
use mech_core::matrix::Matrix;

// Mod ------------------------------------------------------------------------

#[macro_export]
macro_rules! impl_binop2 {
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
      PartialEq + PartialOrd + CompileConst + ConstElem +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + SubAssign +
      Mul<Output = T> + MulAssign +
      Div<Output = T> + DivAssign +
      Rem<Output = T> + RemAssign +
      Zero + One + AsValueKind,
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
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{} requires 2 arguments, got {:?}", stringify!($struct_name), args), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments})
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
      Rem<Output = T> + RemAssign +
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

macro_rules! mod_op {
    ($lhs:expr, $rhs:expr, $out:expr) => {
      unsafe { *$out = *$lhs % *$rhs; }};}
  
macro_rules! mod_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (o,(l,r)) in out_deref.iter_mut().zip(lhs_deref.iter().zip(rhs_deref.iter())) {
        *o = *l % *r;
      }
    }};}

macro_rules! mod_scalar_lhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe { 
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = (*$rhs);
      for (o,l) in out_deref.iter_mut().zip(lhs_deref.iter()) {
        *o = *l % rhs_deref;
      }
    }};}

macro_rules! mod_scalar_rhs_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = (*$lhs);
      let rhs_deref = &(*$rhs);
      for (o,r) in out_deref.iter_mut().zip(rhs_deref.iter()) {
        *o = lhs_deref % *r;
      }
    }};}

macro_rules! mod_mat_vec_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, lhs_col) in out_deref.column_iter_mut().zip(lhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_col[i] % rhs_deref[i];
        }
      }
    }
  };}

macro_rules! mod_vec_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut col, rhs_col) in out_deref.column_iter_mut().zip(rhs_deref.column_iter()) {
        for i in 0..col.len() {
          col[i] = lhs_deref[i] % rhs_col[i];
        }
      }
    }
  };}

macro_rules! mod_mat_row_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, lhs_row) in out_deref.row_iter_mut().zip(lhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_row[i] % rhs_deref[i];
        }
      }
    }
  };}

macro_rules! mod_row_mat_op {
  ($lhs:expr, $rhs:expr, $out:expr) => {
    unsafe {
      let mut out_deref = &mut (*$out);
      let lhs_deref = &(*$lhs);
      let rhs_deref = &(*$rhs);
      for (mut row, rhs_row) in out_deref.row_iter_mut().zip(rhs_deref.row_iter()) {
        for i in 0..row.len() {
          row[i] = lhs_deref[i] % rhs_row[i];
        }
      }
    }
  };}  

macro_rules! register_mod_fxns {
  ($lib:ident, $($suffix:ident),* $(,)?) => {
    paste::paste! {
      $(
        register_fxn_descriptor!([<$lib $suffix>],
          i8, "i8",
          i16, "i16",
          i32, "i32",
          i64, "i64",
          i128, "i128",
          u8, "u8",
          u16, "u16",
          u32, "u32",
          u64, "u64",
          u128, "u128",
          F32, "f32",
          F64, "f64"
        );
      )*
    }
  };
}

macro_rules! impl_math_fxns2 {
  ($lib:ident) => {
    impl_fxns!($lib,T,T,impl_binop2);
    register_mod_fxns!($lib,
      SS, SM1, SM2, SM3, SM4, SM2x3, SM3x2, SMD, SR2, SR3, SR4, SRD,
      SV2, SV3, SV4, SVD, M1S, M2S, M3S, M4S, M2x3S, M3x2S, MDS,
      R2S, R3S, R4S, RDS, V2S, V3S, V4S, VDS, M1M1, M2M2, M3M3, M4M4,
      M2x3M2x3, M3x2M3x2, MDMD, M2V2, M3V3, M4V4, M2x3V2, M3x2V3, MDVD,
      MDV2, MDV3, MDV4, V2M2, V3M3, V4M4, V2M2x3, V3M3x2, VDMD, V2MD,
      V3MD, V4MD, M2R2, M3R3, M4R4, M2x3R3, M3x2R2, MDRD, MDR2, MDR3,
      MDR4, R2M2, R3M3, R4M4, R3M2x3, R2M3x2, RDMD, R2MD, R3MD, R4MD,
      R2R2, R3R3, R4R4, RDRD, V2V2, V3V3, V4V4, VDVD
    );
  }}

impl_math_fxns2!(Mod);

fn impl_mod_fxn(lhs_value: Value, rhs_value: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_binop_match_arms!(
    Mod,
    (lhs_value, rhs_value),
    I8,   i8,   "i8";
    I16,  i16,  "i16";
    I32,  i32,  "i32";
    I64,  i64,  "i64";
    I128, i128, "i128";
    U8,   u8,   "u8";
    U16,  u16,  "u16";
    U32,  u32,  "u32";
    U64,  u64,  "u64";
    U128, u128, "u128";
    F32,  F32,  "f32";
    F64,  F64,  "f64";
  )
}

impl_mech_binop_fxn!(MathMod,impl_mod_fxn);