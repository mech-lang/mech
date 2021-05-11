// |DDDDDDDD|RRRRRRR|SMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM|
// |DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD|S|EEEEEEEE|FFFFFFFFFFFFFFFFFFFFFFF|
// D: Domain [0, 2^32]
// S: Sign bit
// E: Exponent [2^8]
// F: Fraction [2^23] 
// Credit: Chris Granger, who implemented this for Eve v0.4
// Also credit to Josh Cole for coming up with the spec
// Adapted and extended for Mech by Corey Montella

use errors::{ErrorType};
#[cfg(feature = "no-std")] use alloc::string::String;
//#[cfg(feature = "no-std")] use num::traits::float::FloatCore;
#[cfg(feature = "no-std")] use libm::F64Ext;
use num_traits::Float;
use std::mem;

const MANTISSA_MASK:u64 = ((1 as u64) << 49) as u64 - 1; // 49 bits at the end
const META_MASK:u64 = ((1 << 15) as u64 - 1) << 49; // 15 1s at the front
const OVERFLOW_MASK:u64 = ((1 << 16) as u64 - 1) << 48; // 15 1s at the front
const RANGE_MASK:u64 = ((1 << 7) as u64 - 1) << 49;
const SHIFTED_RANGE_DOMAIN_MASK:u64 = (1 << 7) as u64 - 1;
const SHIFTED_FILL:u64 = (((1 as u64) << 57) as u64 - 1) << 7;
const SIGN_MASK:u64 = 1 << 48;

pub type Quantity = u64;

pub trait ToQuantity {
  fn to_quantity(&self) -> u64;
}

pub trait FromQuantity<T> {
  fn get_value(self) -> T;
}

impl ToQuantity for f32 {
  #[inline(always)]
  fn to_quantity(&self) -> u64 {
    unsafe{mem::transmute::<f32, u32>(*self) as u64}
  }
}

impl ToQuantity for u32 {
  #[inline(always)]
  fn to_quantity(&self) -> u64 {
    unsafe{mem::transmute::<f32, u32>(*self as f32) as u64}
  }
}

impl ToQuantity for i32 {
  #[inline(always)]
  fn to_quantity(&self) -> u64 {
    unsafe{mem::transmute::<f32, i32>(*self as f32) as u64}
  }
}

impl ToQuantity for u64 {
  #[inline(always)]
  fn to_quantity(&self) -> u64 {
    unsafe{mem::transmute::<f32, u32>(*self as f32) as u64}
  }
}

pub trait QuantityMath {
  fn negate(self) -> Quantity;
  fn add(self, Quantity) -> Quantity;
  fn sub(self, Quantity) -> Quantity;
  fn multiply(self, Quantity) -> Quantity;
  fn divide(self, Quantity) -> Quantity;
  fn power(self, Quantity) -> Quantity;
  fn less_than(self, Quantity) -> bool;
  fn greater_than(self, Quantity) -> bool;
  fn less_than_equal(self, Quantity) -> bool;
  fn greater_than_equal(self, Quantity) -> bool;
  fn equal(self, Quantity) -> bool;
  fn not_equal(self, Quantity) -> bool;
  fn to_string(self) -> String;
  fn format(self) -> String;
  fn to_f32(self) -> f32;
  fn to_u64(self) -> u64;
}

impl QuantityMath for Quantity {

  fn negate(self) -> Quantity {
    self
  }

  fn to_string(self) -> String {
    self.format()
  }

  fn format(self) -> String {
    format!("{:?}",self.to_f32())
  }

  fn to_f32(self) -> f32 {
    unsafe{mem::transmute::<u32, f32>(self as u32)}
  }

  fn to_u64(self) -> u64 {
    unsafe{mem::transmute::<u32, f32>(self as u32) as u64}
  }

  #[inline(always)]
  fn add(self, other:Quantity) -> Quantity {
    unsafe{mem::transmute::<f32, u32>(self.to_f32() + other.to_f32()) as u64}
  }

  fn sub(self, other:Quantity) -> Quantity {
    unsafe{mem::transmute::<f32, u32>(self.to_f32() - other.to_f32()) as u64}
  }

  fn multiply(self, other:Quantity) -> Quantity {
    unsafe{mem::transmute::<f32, u32>(self.to_f32() * other.to_f32()) as u64}
  }

  fn divide(self, other:Quantity) -> Quantity {
    unsafe{mem::transmute::<f32, u32>(self.to_f32() / other.to_f32()) as u64}
  }

  fn power(self, other:Quantity) -> Quantity {
    unsafe{mem::transmute::<f32, u32>(self.to_f32().powf(other.to_f32())) as u64}
  }

  fn less_than(self, other: Quantity) -> bool {
    self.to_f32() < other.to_f32()
  }

  fn less_than_equal(self, other: Quantity) -> bool {
    self.to_f32() <= other.to_f32()
  }

  fn greater_than_equal(self, other: Quantity) -> bool {
    self.to_f32() >= other.to_f32()
  }

  fn greater_than(self, other: Quantity) -> bool {
    self.to_f32() > other.to_f32()
  }

  fn equal(self, other: Quantity) -> bool {
    self.to_f32() == other.to_f32()
  }

  fn not_equal(self, other: Quantity) -> bool {
    self.to_f32() != other.to_f32()
  }
}