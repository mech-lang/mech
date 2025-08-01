use crate::value::*;
use crate::functions::*;
use crate::*;
use crate::nodes::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::ops::*;
use std::iter::Step;
use num_traits::*;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use libm::{pow,powf};
use paste::paste;

pub type FunctionsRef = Ref<Functions>;
pub type Plan = Ref<Vec<Box<dyn MechFunction>>>;
pub type MutableReference = Ref<Value>;
pub type SymbolTableRef= Ref<SymbolTable>;
pub type ValRef = Ref<Value>;
use std::num::FpCategory;

pub type Ref<T> = Rc<RefCell<T>>;
pub fn new_ref<T>(item: T) -> Rc<RefCell<T>> {
  Rc::new(RefCell::new(item))
}

pub type MResult<T> = Result<T,MechError>;

#[derive(PartialEq, Clone, Copy, PartialOrd, Serialize, Deserialize)]
pub struct F64(pub f64);
impl F64 {
  pub fn new(val: f64) -> F64 {
    F64(val)
  }
}

impl fmt::Debug for F64 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

impl fmt::Display for F64 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

impl From<F64> for String {
  fn from(f: F64) -> Self {
      f.to_string()
  }
}

impl From<F64> for usize {
  fn from(value: F64) -> Self {
    value.0 as usize
  }
}

impl Eq for F64 {}
impl Hash for F64 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}

impl Pow<F64> for F64 {
  type Output = F64;
  fn pow(self, rhs: F64) -> Self::Output {
    F64(self.0.powf(rhs.0))
  }
}

impl Add for F64 {
  type Output = F64;
  fn add(self, other: F64) -> F64 {
    F64(self.0 + other.0)
  }
}
impl AddAssign for F64 {
  fn add_assign(&mut self, other: F64) {
    self.0 += other.0;
  }
}
impl Sub for F64 {
  type Output = F64;
  fn sub(self, other: F64) -> F64 {
    F64(self.0 - other.0)
  }
}
impl SubAssign for F64 {
  fn sub_assign(&mut self, other: F64) {
    self.0 -= other.0;
  }
}
impl Mul for F64 {
  type Output = F64;
  fn mul(self, other: F64) -> F64 {
    F64(self.0 * other.0)
  }
}
impl MulAssign for F64 {
  fn mul_assign(&mut self, other: F64) {
    self.0 *= other.0;
  }
}
impl Div for F64 {
  type Output = F64;
  fn div(self, other: F64) -> F64 {
    F64(self.0 / other.0)
  }
}
impl DivAssign for F64 {
  fn div_assign(&mut self, other: F64) {
    self.0 /= other.0;
  }
}
impl Rem for F64 {
  type Output = F64;
  fn rem(self, other: F64) -> F64 {
    F64(self.0 % other.0)
  }
}
impl RemAssign for F64 {
  fn rem_assign(&mut self, other: F64) {
    self.0 = self.0 % other.0;
  }
}

impl Default for F64 {
  fn default() -> Self {
    F64(0.0)
  }
}

impl Zero for F64 {
  fn zero() -> Self {
    F64(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}

impl One for F64 {
  fn one() -> Self {
    F64(1.0)
  }
  fn is_one(&self) -> bool {
    self.0 == 1.0
  }
}
impl Neg for F64 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    F64(-self.0)
  }
}
impl Step for F64 {
  fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
    if start.0 > end.0 {
      return (0, None);
    }
    let diff = end.0 - start.0;
    // Handle special floating-point cases
    match diff.classify() {
      FpCategory::Normal | FpCategory::Zero => {
        if diff.fract() == 0.0 {
          let steps = diff as usize;
          (steps, Some(steps))
        } else {
          (usize::MAX, None)
        }
      }
      _ => (usize::MAX, None),
    }
  }

  fn forward_checked(start: Self, count: usize) -> Option<Self> {
    Some(F64(start.0 + count as f64)) 
  }

  fn backward_checked(start: Self, count: usize) -> Option<Self> {
    Some(F64(start.0 - count as f64)) 
  }

  fn forward(start: Self, count: usize) -> Self {
    F64(start.0 + count as f64) 
  }

  fn backward(start: Self, count: usize) -> Self {
    F64(start.0 - count as f64)
  }
}

impl From<F64> for Value {
  fn from(val: F64) -> Self {
    Value::F64(new_ref(val))
  }
}

#[derive(PartialEq, Clone, Copy, PartialOrd, Serialize, Deserialize)]
pub struct F32(pub f32);
impl F32 {
  pub fn new(val: f32) -> F32 {
    F32(val)
  }
}

macro_rules! impl_into {
  ($from:ty => $($to:ty),*) => {
    $(impl Into<$to> for $from {
      fn into(self) -> $to {
        self.0 as $to
      }
    })*
  };
}

macro_rules! impl_into_float {
  ($from:ty => $($to:ty),*) => {
    paste!{
      $(impl Into<[<$to:upper>]> for $from {
        fn into(self) -> [<$to:upper>] {
          [<$to:upper>]::new(self as $to)
        }
      })*
    }
  };
}

impl_into_float!(u8 => f32, f64);
impl_into_float!(u16 => f32, f64);
impl_into_float!(u32 => f32, f64);
impl_into_float!(u64 => f32, f64);
impl_into_float!(u128 => f32, f64);

impl_into_float!(i8 => f32, f64);
impl_into_float!(i16 => f32, f64);
impl_into_float!(i32 => f32, f64);
impl_into_float!(i64 => f32, f64);
impl_into_float!(i128 => f32, f64);

impl_into!(F64 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
impl_into!(F32 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl Into<F32> for F64 {
  fn into(self) -> F32 {
    F32::new(self.0 as f32)
  }
}

impl Into<F64> for F32 {
  fn into(self) -> F64 {
    F64::new(self.0 as f64)
  }
}

impl fmt::Display for F32 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

impl fmt::Debug for F32 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

impl From<F32> for String {
  fn from(f: F32) -> Self {
      f.to_string()
  }
}

impl From<F32> for usize {
  fn from(value: F32) -> Self {
    value.0 as usize
  }
}

impl Pow<F32> for F32 {
  type Output = F32;
  fn pow(self, rhs: F32) -> Self::Output {
    F32(self.0.pow(rhs.0))
  }
}

impl Eq for F32 {}
impl Hash for F32 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}
impl Add for F32 {
  type Output = F32;
  fn add(self, other: F32) -> F32 {
    F32(self.0 + other.0)
  }
}
impl AddAssign for F32 {
  fn add_assign(&mut self, other: F32) {
    self.0 += other.0;
  }
}
impl Rem for F32 {
  type Output = F32;
  fn rem(self, other: F32) -> F32 {
    F32(self.0 % other.0)
  }
}
impl RemAssign for F32 {
  fn rem_assign(&mut self, other: F32) {
    self.0 = self.0 % other.0;
  }
}
impl Zero for F32 {
  fn zero() -> Self {
    F32(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}
impl One for F32 {
  fn one() -> Self {
    F32(1.0)
  }
  fn is_one(&self) -> bool {
    self.0 == 1.0
  }
}
impl Sub for F32 {
  type Output = F32;
  fn sub(self, other: F32) -> F32 {
    F32(self.0 - other.0)
  }
}
impl SubAssign for F32 {
  fn sub_assign(&mut self, other: F32) {
    self.0 -= other.0;
  }
}
impl Mul for F32 {
  type Output = F32;
  fn mul(self, other: F32) -> F32 {
    F32(self.0 * other.0)
  }
}
impl MulAssign for F32 {
  fn mul_assign(&mut self, other: F32) {
    self.0 *= other.0;
  }
}
impl Div for F32 {
  type Output = F32;
  fn div(self, other: F32) -> F32 {
    F32(self.0 / other.0)
  }
}
impl DivAssign for F32 {
  fn div_assign(&mut self, other: F32) {
    self.0 /= other.0;
  }
}
impl Neg for F32 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    F32(-self.0)
  }
}
impl Step for F32 {

  fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
    if start.0 > end.0 {
      return (0, None);
    }
    let diff = end.0 - start.0;
    // Handle special floating-point cases
    match diff.classify() {
      FpCategory::Normal | FpCategory::Zero => {
        if diff.fract() == 0.0 {
          let steps = diff as usize;
          (steps, Some(steps))
        } else {
          (usize::MAX, None)
        }
      }
      _ => (usize::MAX, None),
    }
  }
  
  fn forward_checked(start: Self, count: usize) -> Option<Self> {
    Some(F32(start.0 + count as f32)) 
  }

  fn backward_checked(start: Self, count: usize) -> Option<Self> {
    Some(F32(start.0 - count as f32)) 
  }

  fn forward(start: Self, count: usize) -> Self {
    F32(start.0 + count as f32) 
  }

  fn backward(start: Self, count: usize) -> Self {
    F32(start.0 - count as f32) 
  }
}

impl From<F32> for Value {
  fn from(val: F32) -> Self {
    Value::F32(new_ref(val))
  }
}

impl Default for F32 {
  fn default() -> Self {
    F32(0.0)
  }
}