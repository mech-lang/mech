use crate::value::*;
use crate::functions::*;

use mech_core::*;
use mech_core::nodes::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::ops::*;
use num_traits::*;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

pub type FunctionsRef = Ref<Functions>;
pub type Plan = Ref<Vec<Box<dyn MechFunction>>>;
pub type MutableReference = Ref<Value>;
pub type SymbolTableRef= Ref<SymbolTable>;
pub type ValRef = Ref<Value>;

pub type Ref<T> = Rc<RefCell<T>>;
pub fn new_ref<T>(item: T) -> Rc<RefCell<T>> {
  Rc::new(RefCell::new(item))
}

pub type MResult<T> = Result<T,MechError>;

#[derive(PartialEq, Debug, Clone, Copy, PartialOrd)]
pub struct F64(pub f64);
impl F64 {
  pub fn new(val: f64) -> F64 {
    F64(val)
  }
}
impl Eq for F64 {}
impl Hash for F64 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
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

#[derive(PartialEq, Debug, Clone, Copy, PartialOrd)]
pub struct F32(pub f32);
impl F32 {
  pub fn new(val: f32) -> F32 {
    F32(val)
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