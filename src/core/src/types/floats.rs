use crate::*;
use super::*;

// F64
// ----------------------------------------------------------------------------

// This is a wrapper around f64 to implement traits like Hash and Eq, etc.

#[cfg(feature = "f64")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(PartialEq, Clone, Copy, PartialOrd)]
pub struct F64(pub f64);

#[cfg(feature = "f64")]
impl F64 {
  pub fn new(val: f64) -> F64 {
    F64(val)
  }
}

#[cfg(feature = "f64")]
impl fmt::Debug for F64 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

#[cfg(feature = "f64")]
impl fmt::Display for F64 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

#[cfg(feature = "f64")]
impl From<F64> for String {
  fn from(f: F64) -> Self {
      f.to_string()
  }
}

#[cfg(feature = "f64")]
impl From<F64> for usize {
  fn from(value: F64) -> Self {
    value.0 as usize
  }
}

#[cfg(feature = "f64")]
impl Eq for F64 {}

#[cfg(feature = "f64")]
impl Hash for F64 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}

#[cfg(all(feature = "f64", feature = "math_exp"))]
impl Pow<F64> for F64 {
  type Output = F64;
  fn pow(self, rhs: F64) -> Self::Output {
    F64(self.0.powf(rhs.0))
  }
}

#[cfg(feature = "f64")]
impl Add for F64 {
  type Output = F64;
  fn add(self, other: F64) -> F64 {
    F64(self.0 + other.0)
  }
}

#[cfg(feature = "f64")]
impl AddAssign for F64 {
  fn add_assign(&mut self, other: F64) {
    self.0 += other.0;
  }
}
#[cfg(feature = "f64")]
impl Sub for F64 {
  type Output = F64;
  fn sub(self, other: F64) -> F64 {
    F64(self.0 - other.0)
  }
}
#[cfg(feature = "f64")]
impl SubAssign for F64 {
  fn sub_assign(&mut self, other: F64) {
    self.0 -= other.0;
  }
}

#[cfg(feature = "f64")]
impl Mul for F64 {
  type Output = F64;
  fn mul(self, other: F64) -> F64 {
    F64(self.0 * other.0)
  }
}

#[cfg(feature = "f64")]
impl MulAssign for F64 {
  fn mul_assign(&mut self, other: F64) {
    self.0 *= other.0;
  }
}

#[cfg(feature = "f64")]
impl Div for F64 {
  type Output = F64;
  fn div(self, other: F64) -> F64 {
    F64(self.0 / other.0)
  }
}

#[cfg(feature = "f64")]
impl DivAssign for F64 {
  fn div_assign(&mut self, other: F64) {
    self.0 /= other.0;
  }
}

#[cfg(feature = "f64")]
impl Rem for F64 {
  type Output = F64;
  fn rem(self, other: F64) -> F64 {
    F64(self.0 % other.0)
  }
}

#[cfg(feature = "f64")]
impl RemAssign for F64 {
  fn rem_assign(&mut self, other: F64) {
    self.0 = self.0 % other.0;
  }
}

#[cfg(feature = "f64")]
impl Default for F64 {
  fn default() -> Self {
    F64(0.0)
  }
}

#[cfg(feature = "f64")]
impl Zero for F64 {
  fn zero() -> Self {
    F64(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}

#[cfg(feature = "f64")]
impl One for F64 {
  fn one() -> Self {
    F64(1.0)
  }
  fn is_one(&self) -> bool {
    self.0 == 1.0
  }
}

#[cfg(feature = "f64")]
impl Neg for F64 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    F64(-self.0)
  }
}

#[cfg(all(feature = "f64", feature = "range"))]
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

#[cfg(feature = "f64")]
impl From<F64> for Value {
  fn from(val: F64) -> Self {
    Value::F64(Ref::new(val))
  }
}

#[cfg(all(feature = "f64", feature = "f32"))]
impl From<F32> for F64 {
  fn from(value: F32) -> Self {
    F64::new(value.0 as f64)
  }
}

#[cfg(all(feature = "f64", feature = "f32"))]
impl From<F64> for F32 {
  fn from(value: F64) -> Self {
    F32::new(value.0 as f32)
  }
}

#[cfg(feature = "f64")]
impl ToUsize for F64 {
  fn to_usize(&self) -> usize {
    self.0 as usize
  }
}

// F32
// ----------------------------------------------------------------------------

// This is a wrapper around f32 to implement traits like Hash and Eq, etc.

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg(feature = "f32")]
#[derive(PartialEq, Clone, Copy, PartialOrd)]
pub struct F32(pub f32);

#[cfg(feature = "f32")]
impl F32 {
  pub fn new(val: f32) -> F32 {
    F32(val)
  }
}

#[cfg(feature = "f32")]
impl fmt::Display for F32 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

#[cfg(feature = "f32")]
impl fmt::Debug for F32 {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "{}", self.0)
  }
}

#[cfg(feature = "f32")]
impl From<F32> for String {
  fn from(f: F32) -> Self {
      f.to_string()
  }
}

#[cfg(feature = "f32")]
impl From<F32> for usize {
  fn from(value: F32) -> Self {
    value.0 as usize
  }
}

#[cfg(all(feature = "f32", feature = "math_exp"))]
impl Pow<F32> for F32 {
  type Output = F32;
  fn pow(self, rhs: F32) -> Self::Output {
    F32(self.0.pow(rhs.0))
  }
}

#[cfg(feature = "f32")]
impl Eq for F32 {}

#[cfg(feature = "f32")]
impl Hash for F32 {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.to_bits().hash(state);
  }
}

#[cfg(feature = "f32")]
impl Add for F32 {
  type Output = F32;
  fn add(self, other: F32) -> F32 {
    F32(self.0 + other.0)
  }
}

#[cfg(feature = "f32")]
impl AddAssign for F32 {
  fn add_assign(&mut self, other: F32) {
    self.0 += other.0;
  }
}

#[cfg(feature = "f32")]
impl Rem for F32 {
  type Output = F32;
  fn rem(self, other: F32) -> F32 {
    F32(self.0 % other.0)
  }
}

#[cfg(feature = "f32")]
impl RemAssign for F32 {
  fn rem_assign(&mut self, other: F32) {
    self.0 = self.0 % other.0;
  }
}

#[cfg(feature = "f32")]
impl Zero for F32 {
  fn zero() -> Self {
    F32(0.0)
  }
  fn is_zero(&self) -> bool {
    self.0 == 0.0
  }
}

#[cfg(feature = "f32")]
impl One for F32 {
  fn one() -> Self {
    F32(1.0)
  }
  fn is_one(&self) -> bool {
    self.0 == 1.0
  }
}

#[cfg(feature = "f32")]
impl Sub for F32 {
  type Output = F32;
  fn sub(self, other: F32) -> F32 {
    F32(self.0 - other.0)
  }
}

#[cfg(feature = "f32")]
impl SubAssign for F32 {
  fn sub_assign(&mut self, other: F32) {
    self.0 -= other.0;
  }
}

#[cfg(feature = "f32")]
impl Mul for F32 {
  type Output = F32;
  fn mul(self, other: F32) -> F32 {
    F32(self.0 * other.0)
  }
}

#[cfg(feature = "f32")]
impl MulAssign for F32 {
  fn mul_assign(&mut self, other: F32) {
    self.0 *= other.0;
  }
}

#[cfg(feature = "f32")]
impl Div for F32 {
  type Output = F32;
  fn div(self, other: F32) -> F32 {
    F32(self.0 / other.0)
  }
}

#[cfg(feature = "f32")]
impl DivAssign for F32 {
  fn div_assign(&mut self, other: F32) {
    self.0 /= other.0;
  }
}

#[cfg(feature = "f32")]
impl Neg for F32 {
  type Output = Self;
  fn neg(self) -> Self::Output {
    F32(-self.0)
  }
}

#[cfg(all(feature = "f32", feature = "range"))]
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

#[cfg(feature = "f32")]
impl From<F32> for Value {
  fn from(val: F32) -> Self {
    Value::F32(Ref::new(val))
  }
}

#[cfg(feature = "f32")]
impl Default for F32 {
  fn default() -> Self {
    F32(0.0)
  }
}

#[cfg(feature = "f32")]
impl ToUsize for F32 {
  fn to_usize(&self) -> usize {
    self.0 as usize
  }
}

// Conversion Macros
// ----------------------------------------------------------------------------

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

#[cfg(all(feature = "f32", feature = "u8"))]
impl_into_float!(u8 => f32);
#[cfg(all(feature = "f64", feature = "u8"))]
impl_into_float!(u8 => f64);
#[cfg(all(feature = "f32", feature = "u16"))]
impl_into_float!(u16 => f32);
#[cfg(all(feature = "f64", feature = "u16"))]
impl_into_float!(u16 => f64);
#[cfg(all(feature = "f32", feature = "u32"))]
impl_into_float!(u32 => f32);
#[cfg(all(feature = "f64", feature = "u32"))]
impl_into_float!(u32 => f64);
#[cfg(all(feature = "f32", feature = "u64"))]
impl_into_float!(u64 => f32);
#[cfg(all(feature = "f64", feature = "u64"))]
impl_into_float!(u64 => f64);
#[cfg(all(feature = "f32", feature = "u128"))]
impl_into_float!(u128 => f32);
#[cfg(all(feature = "f64", feature = "u128"))]
impl_into_float!(u128 => f64);
#[cfg(all(feature = "f32", feature = "i8"))]
impl_into_float!(i8 => f32);
#[cfg(all(feature = "f64", feature = "i8"))]
impl_into_float!(i8 => f64);
#[cfg(all(feature = "f32", feature = "i16"))]
impl_into_float!(i16 => f32);
#[cfg(all(feature = "f64", feature = "i16"))]
impl_into_float!(i16 => f64);
#[cfg(all(feature = "f32", feature = "i32"))]
impl_into_float!(i32 => f32);
#[cfg(all(feature = "f64", feature = "i32"))]
impl_into_float!(i32 => f64);
#[cfg(all(feature = "f32", feature = "i64"))]
impl_into_float!(i64 => f32);
#[cfg(all(feature = "f64", feature = "i64"))]
impl_into_float!(i64 => f64);
#[cfg(all(feature = "f32", feature = "i128"))]
impl_into_float!(i128 => f32);
#[cfg(all(feature = "f64", feature = "i128"))]
impl_into_float!(i128 => f64);

#[cfg(feature = "f64")]
impl_into!(F64 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
#[cfg(feature = "f32")]
impl_into!(F32 => u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);