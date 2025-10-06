use mech_core::*;
use mech_core::value::ToUsize;
#[cfg(feature = "matrix")]
use mech_core::structures::matrix::Matrix;

use std::fmt::Debug;
use std::ops::{Add, AddAssign, Sub, Div};
use num_traits::{Zero, One};
use itertools::Itertools;
use paste::paste;

// Combinatorics N Choose K----------------------------------------------------

#[derive(Debug)]
pub struct NChooseK<T> {
  n: Ref<T>,
  k: Ref<T>,
  out: Ref<T>,
}
impl<T> MechFunctionFactory for NChooseK<T> 
where
  T: Copy + Debug + Clone + Sync + Send + 'static +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + Div<Output = T> +
      Zero + One +
      ConstElem + CompileConst + AsValueKind +
      PartialEq + PartialOrd,
  Ref<T>: ToValue,
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let n: Ref<T> = unsafe{ arg1.as_unchecked().clone() };
        let k: Ref<T> = unsafe{ arg2.as_unchecked().clone() };
        let out: Ref<T> = unsafe{ out.as_unchecked().clone() };
        Ok(Box::new(Self{n, k, out}))
      }
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments}),
    }
  }
}
impl<T> MechFunctionImpl for NChooseK<T>
where
  T: Copy + Debug + Clone + Sync + Send + 'static +
      Add<Output = T> + AddAssign +
      Sub<Output = T> + Div<Output = T> +
      Zero + One +
      PartialEq + PartialOrd,
  Ref<T>: ToValue,
{
  fn solve(&self) {
    let n_ptr = self.n.as_ptr();
    let k_ptr = self.k.as_ptr();
    let out_ptr = self.out.as_mut_ptr();
    unsafe {
      let n = *n_ptr;
      let k = *k_ptr;
      if k > n {
        *out_ptr = T::zero(); // undefined for k > n
        return;
      }
      let mut result = T::one();
      let mut i = T::zero();
      while i < k {
        let numerator = n - i;
        let denominator = i + T::one();
        result = result * numerator / denominator;
        i = i + T::one();
      }
      *out_ptr = result;
    }
  }
  fn out(&self) -> Value {self.out.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for NChooseK<T> 
where
    T: ConstElem + CompileConst + AsValueKind
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("NChooseK<{}>", T::as_value_kind());
    compile_binop!(name, self.out, self.n, self.k, ctx, FeatureFlag::Custom(hash_str("combinatorics/n-choose-k")));
  }
}
register_fxn_descriptor!(NChooseK, u8, "u8", i8, "i8", u16, "u16", i16, "i16", u32, "u32", i32, "i32", u64, "u64", i64, "i64", u128, "u128", i128, "i128", F32, "f32", F64, "f64", R64, "r64", C64, "c64");

#[cfg(feature = "matrix")]
#[derive(Debug)]
pub struct NChooseKMatrix<T> {
  n: Ref<Matrix<T>>,
  k: Ref<T>,
  out: Ref<Matrix<T>>,
}
#[cfg(feature = "matrix")]
impl<T> MechFunctionFactory for NChooseKMatrix<T>
where
    T: Copy + Debug + Clone + Sync + Send + 'static +
       ToUsize + std::fmt::Display +
       Add<Output = T> + AddAssign +
       Sub<Output = T> + Div<Output = T> +
       Zero + One +
      ConstElem + CompileConst + AsValueKind +
       PartialEq + PartialOrd + ToMatrix,
    Ref<T>: ToValue,
    Matrix<T>: ToValue,
{
  fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
    match args {
      FunctionArgs::Binary(out, arg1, arg2) => {
        let n: Ref<Matrix<T>> = unsafe{ arg1.as_unchecked().clone() };
        let k: Ref<T> = unsafe{ arg2.as_unchecked().clone() };
        let out: Ref<Matrix<T>> = unsafe{ out.as_unchecked().clone() };
        Ok(Box::new(Self{n, k, out}))
      }
      _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments}),
    }
  }
}
#[cfg(feature = "matrix")]
impl<T> MechFunctionImpl for NChooseKMatrix<T>
where
    T: Copy + Debug + Clone + Sync + Send + 'static +
       ToUsize + std::fmt::Display +
       Add<Output = T> + AddAssign +
       Sub<Output = T> + Div<Output = T> +
       Zero + One +
       PartialEq + PartialOrd + ToMatrix,
    Ref<T>: ToValue,
    Matrix<T>: ToValue,
{
  fn solve(&self) {
      let n_matrix = self.n.borrow();
      let k_scalar = *self.k.borrow();
      let elements: Vec<T> = n_matrix.as_vec();
      let k_usize: usize = k_scalar.to_usize();
      // Check if k is greater than the number of elements. If it is, return an empty matrix.
      if k_usize > elements.len() {
          let empty = T::to_matrix(vec![], 0, k_usize);
          *self.out.borrow_mut() = empty;
          return;
      }
      // Generate combinations
      let combinations: Vec<Vec<T>> = elements.iter().copied().combinations(k_usize).collect();
      
      // Reshape into output matrix
      let rows = combinations.len();
      let cols = k_usize;
      let flat_data: Vec<T> = combinations.into_iter().flatten().collect();
      let result = T::to_matrix(flat_data, cols, rows);
      *self.out.borrow_mut() = result;
  }
  fn out(&self) -> Value { (*self.out.borrow()).to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(all(feature = "matrix", feature = "compiler"))]
impl<T> MechFunctionCompiler for NChooseKMatrix<T> 
where
    T: ConstElem + CompileConst + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("NChooseKMatrix<{}>", T::as_value_kind());
    compile_binop!(name, self.out, self.n, self.k, ctx, FeatureFlag::Custom(hash_str("combinatorics/n-choose-k")));
  }
}
register_fxn_descriptor!(NChooseKMatrix, u8, "u8", i8, "i8", u16, "u16", i16, "i16", u32, "u32", i32, "i32", u64, "u64", i64, "i64", u128, "u128", i128, "i128", F32, "f32", F64, "f64", R64, "r64", C64, "c64");


fn impl_combinatorics_n_choose_k_fxn(n: Value, k: Value) -> Result<Box<dyn MechFunction>, MechError> {
  match (n,k) {
    #[cfg(feature = "u8")]
    (Value::U8(n), Value::U8(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(u8::default())})),
    #[cfg(feature = "u16")]
    (Value::U16(n), Value::U16(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(u16::default())})),
    #[cfg(feature = "u32")]
    (Value::U32(n), Value::U32(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(u32::default())})),
    #[cfg(feature = "u64")]
    (Value::U64(n), Value::U64(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(u64::default())})),
    #[cfg(feature = "u128")]
    (Value::U128(n), Value::U128(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(u128::default())})),
    #[cfg(feature = "i8")]
    (Value::I8(n), Value::I8(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(i8::default())})),
    #[cfg(feature = "i16")]
    (Value::I16(n), Value::I16(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(i16::default())})),
    #[cfg(feature = "i32")]
    (Value::I32(n), Value::I32(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(i32::default())})),
    #[cfg(feature = "i64")]
    (Value::I64(n), Value::I64(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(i64::default())})),
    #[cfg(feature = "i128")]
    (Value::I128(n), Value::I128(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(i128::default())})),
    #[cfg(feature = "f32")]
    (Value::F32(n), Value::F32(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(F32::default())})),
    #[cfg(feature = "f64")]
    (Value::F64(n), Value::F64(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(F64::default())})),
    #[cfg(feature = "rational")]
    (Value::R64(n), Value::R64(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(R64::default())})),
    #[cfg(feature = "complex")]
    (Value::C64(n), Value::C64(k)) => Ok(Box::new(NChooseK{n: n, k: k, out: Ref::new(C64::default())})),
    #[cfg(all(feature = "matrix", feature = "u8"))]
    (Value::MatrixU8(n), Value::U8(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(u8::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "u16"))]
    (Value::MatrixU16(n), Value::U16(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(u16::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "u32"))]
    (Value::MatrixU32(n), Value::U32(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(u32::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "u64"))]
    (Value::MatrixU64(n), Value::U64(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(u64::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "u128"))]
    (Value::MatrixU128(n), Value::U128(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(u128::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "i8"))]
    (Value::MatrixI8(n), Value::I8(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(i8::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "i16"))]
    (Value::MatrixI16(n), Value::I16(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(i16::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "i32"))]
    (Value::MatrixI32(n), Value::I32(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(i32::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "i64"))]
    (Value::MatrixI64(n), Value::I64(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(i64::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "i128"))]
    (Value::MatrixI128(n), Value::I128(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(i128::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "f32"))]
    (Value::MatrixF32(n), Value::F32(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(F32::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "f64"))]
    (Value::MatrixF64(n), Value::F64(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(F64::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "rational"))]
    (Value::MatrixR64(n), Value::R64(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(R64::to_matrix(vec![], 0, 0))})),
    #[cfg(all(feature = "matrix", feature = "complex"))]
    (Value::MatrixC64(n), Value::C64(k)) => Ok(Box::new(NChooseKMatrix{n: Ref::new(n), k, out: Ref::new(C64::to_matrix(vec![], 0, 0))})),
    x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
  }
}
 
pub struct CombinatoricsNChooseK {}
impl NativeFunctionCompiler for CombinatoricsNChooseK {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let n = arguments[0].clone();
    let k = arguments[1].clone();

    match impl_combinatorics_n_choose_k_fxn(n.clone(),k.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (n,k) {
          (Value::MutableReference(n),Value::MutableReference(k)) => {let n_brrw = n.borrow();let k_brrw = k.borrow();impl_combinatorics_n_choose_k_fxn(n_brrw.clone(),k_brrw.clone())}
          (n,Value::MutableReference(k)) => {let k_brrw = k.borrow(); impl_combinatorics_n_choose_k_fxn(n.clone(),k_brrw.clone())}
          (Value::MutableReference(n),k) => {let n_brrw = n.borrow();impl_combinatorics_n_choose_k_fxn(n_brrw.clone(),k.clone())}
          x => Err(MechError{file: file!().to_string(), tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

inventory::submit! {
  FunctionCompiler {
    name: "combinatorics/n-choose-k",
    ptr: &CombinatoricsNChooseK{},
  }
}