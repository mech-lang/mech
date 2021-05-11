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
    let result:u64 = 0;
    result | unsafe{mem::transmute::<f32, u32>(*self)} as u64
  }
}


impl ToQuantity for u32 {
  #[inline(always)]
  fn to_quantity(&self) -> u64 {
    let result:u64 = 0;
    result | (*self as f32) as u64
  }
}

impl ToQuantity for i32 {
  #[inline(always)]
  fn to_quantity(&self) -> u64 {
    let result:u64 = 0;
    result | (*self as f32) as u64
  }
}

impl ToQuantity for u64 {
  #[inline(always)]
  fn to_quantity(&self) -> u64 {
    let result:u64 = 0;
    result | (*self as f32) as u64
  }
}


/*
#[inline(always)]
pub fn overflow_handler(me:u64) -> (u64, u64) {
  let hi = 64 - me.leading_zeros() - 48;
  let r = (2u64.pow(hi) as f64).log10().ceil() as u32;
  let result = me / 10u64.pow(r) as u64;
  (result, r as u64)
}

pub fn decrease_range(mantissa:i64, range_delta:u64) -> (i64, u64) {
  let remaining_space = mantissa.leading_zeros();
  let thing:u64 = (1 as u64) << remaining_space;
  let remaining_10 = (thing as f64).log10().floor() as u64;
  if range_delta <= remaining_10 {
    let new_mantissa = mantissa * 10u64.pow(range_delta as u32) as i64;
    (new_mantissa, range_delta)
  } else {
    let new_mantissa = mantissa * 10u64.pow(remaining_10 as u32) as i64;
    (new_mantissa, remaining_10)
  }
}

pub fn increase_range(mantissa:i64, range_delta:u64) -> (i64, bool) {
  let range = 10u64.pow(range_delta as u32) as i64;
  (mantissa / range, mantissa % range != 0)
}

#[inline(always)]
pub fn shifted_range(range:u64) -> u64 {
  range << 49
}

pub fn make_quantity(mantissa:i64, range:i64, domain:u64) -> Quantity {
  let value = mantissa.to_quantity();
  let cur_range = (value.range() + range) as u64;
  value & !RANGE_MASK | ((cur_range << 49) & RANGE_MASK) | (domain << 56)
}*/

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
    self as u64
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