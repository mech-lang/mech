#[macro_use]
use crate::stdlib::*;

// ----------------------------------------------------------------------------
// Type Conversion Library
// ----------------------------------------------------------------------------

pub mod scalar;
pub mod scalar_to_mat;
pub mod mat_to_mat;

pub use self::scalar::*;
pub use self::scalar_to_mat::*;
pub use self::mat_to_mat::*;

pub trait LosslessInto<T> {
  fn lossless_into(self) -> T;
}

macro_rules! lossless_into {
  ($from_type:ty) => {
    impl LosslessInto<String> for $from_type {
      fn lossless_into(self) -> String {
        self.to_string()
      }
    }
  };
  ($from_type:ty, $to_type:ty) => {
    impl LosslessInto<$to_type> for $from_type {
      fn lossless_into(self) -> $to_type {
        self as $to_type
      }
    }
  };
}

lossless_into!(i8);
lossless_into!(i16);
lossless_into!(i32);
lossless_into!(i64);
lossless_into!(i128);
lossless_into!(u8);
lossless_into!(u16);
lossless_into!(u32);
lossless_into!(u64);
lossless_into!(u128);
lossless_into!(F32);
lossless_into!(F64);
lossless_into!(bool);

lossless_into!(u8,u8);
lossless_into!(u8,u16);
lossless_into!(u8,u32);
lossless_into!(u8,u64);
lossless_into!(u8,u128);
lossless_into!(u8,i8);
lossless_into!(u8,i16);
lossless_into!(u8,i32);
lossless_into!(u8,i64);
lossless_into!(u8,i128);

lossless_into!(u16,u8);
lossless_into!(u16,u16);
lossless_into!(u16,u32);
lossless_into!(u16,u64);
lossless_into!(u16,u128);
lossless_into!(u16,i8);
lossless_into!(u16,i16);
lossless_into!(u16,i32);
lossless_into!(u16,i64);
lossless_into!(u16,i128);

lossless_into!(u32,u8);
lossless_into!(u32,u16);
lossless_into!(u32,u32);
lossless_into!(u32,u64);
lossless_into!(u32,u128);
lossless_into!(u32,i8);
lossless_into!(u32,i16);
lossless_into!(u32,i32);
lossless_into!(u32,i64);
lossless_into!(u32,i128);

lossless_into!(u64,u8);
lossless_into!(u64,u16);
lossless_into!(u64,u32);
lossless_into!(u64,u64);
lossless_into!(u64,u128);
lossless_into!(u64,i8);
lossless_into!(u64,i16);
lossless_into!(u64,i32);
lossless_into!(u64,i64);
lossless_into!(u64,i128);

lossless_into!(u128,u8);
lossless_into!(u128,u16);
lossless_into!(u128,u32);
lossless_into!(u128,u64);
lossless_into!(u128,u128);
lossless_into!(u128,i8);
lossless_into!(u128,i16);
lossless_into!(u128,i32);
lossless_into!(u128,i64);
lossless_into!(u128,i128);

lossless_into!(i8,i8);
lossless_into!(i8,i16);
lossless_into!(i8,i32);
lossless_into!(i8,i64);
lossless_into!(i8,i128);
lossless_into!(i8,u8);
lossless_into!(i8,u16);
lossless_into!(i8,u32);
lossless_into!(i8,u64);
lossless_into!(i8,u128);

lossless_into!(i16,i8);
lossless_into!(i16,i16);
lossless_into!(i16,i32);
lossless_into!(i16,i64);
lossless_into!(i16,i128);
lossless_into!(i16,u8);
lossless_into!(i16,u16);
lossless_into!(i16,u32);
lossless_into!(i16,u64);
lossless_into!(i16,u128);

lossless_into!(i32,i8);
lossless_into!(i32,i16);
lossless_into!(i32,i32);
lossless_into!(i32,i64);
lossless_into!(i32,i128);
lossless_into!(i32,u8);
lossless_into!(i32,u16);
lossless_into!(i32,u32);
lossless_into!(i32,u64);
lossless_into!(i32,u128);

lossless_into!(i64,i8);
lossless_into!(i64,i16);
lossless_into!(i64,i32);
lossless_into!(i64,i64);
lossless_into!(i64,i128);
lossless_into!(i64,u8);
lossless_into!(i64,u16);
lossless_into!(i64,u32);
lossless_into!(i64,u64);
lossless_into!(i64,u128);

lossless_into!(i128,i8);
lossless_into!(i128,i16);
lossless_into!(i128,i32);
lossless_into!(i128,i64);
lossless_into!(i128,i128);
lossless_into!(i128,u8);
lossless_into!(i128,u16);
lossless_into!(i128,u32);
lossless_into!(i128,u64);
lossless_into!(i128,u128);

macro_rules! lossless_into_float_to_int {
  ($float_type:ty, $int_type:ty) => {
    impl LosslessInto<$int_type> for $float_type {
      fn lossless_into(self) -> $int_type {
        self.0 as $int_type
      }
    }
  };
}

lossless_into_float_to_int!(F64, u8);
lossless_into_float_to_int!(F64, u16);
lossless_into_float_to_int!(F64, u32);
lossless_into_float_to_int!(F64, u64);
lossless_into_float_to_int!(F64, u128);
lossless_into_float_to_int!(F64, i8);
lossless_into_float_to_int!(F64, i16);
lossless_into_float_to_int!(F64, i32);
lossless_into_float_to_int!(F64, i64);
lossless_into_float_to_int!(F64, i128);

lossless_into_float_to_int!(F32, u8);
lossless_into_float_to_int!(F32, u16);
lossless_into_float_to_int!(F32, u32);
lossless_into_float_to_int!(F32, u64);
lossless_into_float_to_int!(F32, u128);
lossless_into_float_to_int!(F32, i8);
lossless_into_float_to_int!(F32, i16);
lossless_into_float_to_int!(F32, i32);
lossless_into_float_to_int!(F32, i64);
lossless_into_float_to_int!(F32, i128);

macro_rules! lossless_into_int_to_float {
  ($int_type:ty) => {
    paste!{
      impl LosslessInto<F32> for $int_type {
        fn lossless_into(self) -> F32 {
          F32::new(self as f32)
        }
      }
      impl LosslessInto<F64> for $int_type {
        fn lossless_into(self) -> F64 {
          F64::new(self as f64)
        }
      }
    }
  };
}

lossless_into_int_to_float!(u8);
lossless_into_int_to_float!(u16);
lossless_into_int_to_float!(u32);
lossless_into_int_to_float!(u64);
lossless_into_int_to_float!(u128);
lossless_into_int_to_float!(i8);
lossless_into_int_to_float!(i16);
lossless_into_int_to_float!(i32);
lossless_into_int_to_float!(i64);
lossless_into_int_to_float!(i128);

impl LosslessInto<F32> for F64 {
  fn lossless_into(self) -> F32 {
    F32::new(self.0 as f32)
  }
}

impl LosslessInto<F64> for F32 {
  fn lossless_into(self) -> F64 {
    F64::new(self.0 as f64)
  }
}

impl LosslessInto<F64> for F64 {
  fn lossless_into(self) -> F64 {
    self
  }
}

impl LosslessInto<F32> for F32 {
  fn lossless_into(self) -> F32 {
    self
  }
}