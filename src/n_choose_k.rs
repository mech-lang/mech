use crate::*;
use mech_core::*;

use std::fmt::Debug;
use std::ops::{Add, AddAssign, Sub, Div, Mul};
use num_traits::{Zero, One};
use itertools::Itertools;

// Combinatorics N Choose K----------------------------------------------------

#[derive(Debug)]
pub struct NChooseK<T> {
    n: Ref<T>,
    k: Ref<T>,
    out: Ref<T>,
}

impl<T> MechFunction for NChooseK<T>
where
    T: Copy + Debug + Clone + Sync + Send + 'static +
       Add<Output = T> + AddAssign +
       Sub<Output = T> + Mul<Output = T> + Div<Output = T> +
       Zero + One +
       PartialEq + PartialOrd,
    Ref<T>: ToValue,
{
  fn solve(&self) {
    let n_ptr = self.n.as_ptr();
    let k_ptr = self.k.as_ptr();
    let out_ptr = self.out.as_ptr();
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

#[derive(Debug)]
pub struct NChooseKMatrix<T> {
  n: Ref<Matrix<T>>,
  k: Ref<T>,
  out: Ref<Matrix<T>>,
}

impl<T> MechFunction for NChooseKMatrix<T>
where
    T: Copy + Debug + Clone + Sync + Send + 'static +
       Into<usize> + std::fmt::Display + PrettyPrint +
       Add<Output = T> + AddAssign +
       Sub<Output = T> + Mul<Output = T> + Div<Output = T> +
       Zero + One +
       PartialEq + PartialOrd + ToMatrix,
    Ref<T>: ToValue,
    Matrix<T>: ToValue,
{
  fn solve(&self) {
      let n_matrix = self.n.borrow();
      let k_scalar = *self.k.borrow();
      let elements: Vec<T> = n_matrix.as_vec();
      let k_usize: usize = k_scalar.into();
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
      let result = T::to_matrix(flat_data, rows, cols);
      *self.out.borrow_mut() = result;
  }
  fn out(&self) -> Value { (*self.out.borrow()).to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}

fn impl_combinatorics_n_choose_k_fxn(n: Value, k: Value) -> Result<Box<dyn MechFunction>, MechError> {
  match (n,k) {
    (Value::U8(n), Value::U8(k)) => Ok(Box::new(NChooseK::<u8>{n: n, k: k, out: new_ref(u8::zero())})),
    (Value::U16(n), Value::U16(k)) => Ok(Box::new(NChooseK::<u16>{n: n, k: k, out: new_ref(u16::zero())})),
    (Value::U32(n), Value::U32(k)) => Ok(Box::new(NChooseK::<u32>{n: n, k: k, out: new_ref(u32::zero())})),
    (Value::U64(n), Value::U64(k)) => Ok(Box::new(NChooseK::<u64>{n: n, k: k, out: new_ref(u64::zero())})),
    (Value::U128(n), Value::U128(k)) => Ok(Box::new(NChooseK::<u128>{n: n, k: k, out: new_ref(u128::zero())})),
    (Value::I8(n), Value::I8(k)) => Ok(Box::new(NChooseK::<i8>{n: n, k: k, out: new_ref(i8::zero())})),
    (Value::I16(n), Value::I16(k)) => Ok(Box::new(NChooseK::<i16>{n: n, k: k, out: new_ref(i16::zero())})),
    (Value::I32(n), Value::I32(k)) => Ok(Box::new(NChooseK::<i32>{n: n, k: k, out: new_ref(i32::zero())})),
    (Value::I64(n), Value::I64(k)) => Ok(Box::new(NChooseK::<i64>{n: n, k: k, out: new_ref(i64::zero())})),
    (Value::I128(n), Value::I128(k)) => Ok(Box::new(NChooseK::<i128>{n: n, k: k, out: new_ref(i128::zero())})),
    (Value::F32(n), Value::F32(k)) => Ok(Box::new(NChooseK::<F32>{n: n, k: k, out: new_ref(F32::zero())})),
    (Value::F64(n), Value::F64(k)) => Ok(Box::new(NChooseK::<F64>{n: n, k: k, out: new_ref(F64::zero())})),
    (Value::MatrixF64(n), Value::F64(k)) => {
      let out = new_ref(F64::to_matrix(vec![], 0, 0));
      let n = new_ref(n);
      Ok(Box::new(NChooseKMatrix::<F64>{n, k, out}))
    },
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