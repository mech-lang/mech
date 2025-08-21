use crate::*;

#[cfg(feature = "f64")]
impl CompileConst for F64 {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_f64::<LittleEndian>(self.0)?;
    ctx.compile_const(&payload, ValueKind::F64)
  }
}

#[cfg(feature = "f32")]
impl CompileConst for F32 {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_f32::<LittleEndian>(self.0)?;
    ctx.compile_const(&payload, ValueKind::F32)
  }
}

#[cfg(feature = "u8")]
impl CompileConst for u8 {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_u8(*self)?;
    ctx.compile_const(&payload, ValueKind::U8)
  }
}

#[cfg(feature = "i8")]
impl CompileConst for i8 {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_i8(*self)?;
    ctx.compile_const(&payload, ValueKind::I8)
  }
}

macro_rules! impl_compile_const {
  ($feature:literal, $t:tt) => {
    paste! {
      #[cfg(feature = $feature)]
      impl CompileConst for $t {
        fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
          let mut payload = Vec::<u8>::new();
          payload.[<write_ $t>]::<LittleEndian>(*self)?;
          ctx.compile_const(&payload, ValueKind::[<$t:upper>])
        }
      }
    }
  };
}

impl_compile_const!("u16", u16);
impl_compile_const!("u32", u32);
impl_compile_const!("u64", u64);
impl_compile_const!("u128", u128);
impl_compile_const!("i16", i16);
impl_compile_const!("i32", i32);
impl_compile_const!("i64", i64);
impl_compile_const!("i128", i128);

#[cfg(feature = "bool")]
impl CompileConst for bool {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_u8(if *self { 1 } else { 0 })?;
    ctx.compile_const(&payload, ValueKind::Bool)
  }
}

#[cfg(feature = "string")]
impl CompileConst for String {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_u32::<LittleEndian>(self.len() as u32)?;
    payload.extend_from_slice(self.as_bytes());
    ctx.compile_const(&payload, ValueKind::String)
  }
}

#[cfg(feature = "rational")]
impl CompileConst for RationalNumber {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_i64::<LittleEndian>(*self.numer())?;
    payload.write_i64::<LittleEndian>(*self.denom())?;
    ctx.compile_const(&payload, ValueKind::RationalNumber)
  }
}

#[cfg(feature = "complex")]
impl CompileConst for ComplexNumber {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_f64::<LittleEndian>(self.0.re)?;
    payload.write_f64::<LittleEndian>(self.0.im)?;
    ctx.compile_const(&payload, ValueKind::ComplexNumber)
  }
}

pub trait ConstElem {
  fn write_le(&self, out: &mut Vec<u8>);
  fn value_kind() -> ValueKind;
  fn align() -> u8 { 1 }
}

#[cfg(feature = "f64")]
impl ConstElem for F64 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_f64::<LittleEndian>(self.0).expect("write f64");
  }
  fn value_kind() -> ValueKind { ValueKind::F64 }
  fn align() -> u8 { 8 }
}

#[cfg(feature = "f32")]
impl ConstElem for F32 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_f32::<LittleEndian>(self.0).expect("write f32");
  }
  fn value_kind() -> ValueKind { ValueKind::F32 }
  fn align() -> u8 { 4 }
} 

macro_rules! impl_const_elem {
  ($feature:literal, $t:ty, $align:expr) => {
    paste!{
      #[cfg(feature = $feature)]
      impl ConstElem for $t {
        fn write_le(&self, out: &mut Vec<u8>) {
          out.[<write_ $t>]::<LittleEndian>(*self).expect(concat!("write ", stringify!($t)));
        }
        fn value_kind() -> ValueKind { ValueKind::[<$t:upper>] }
        fn align() -> u8 { $align }
      }
    }
  };
}

impl_const_elem!("u16", u16, 2);
impl_const_elem!("u32", u32, 4);
impl_const_elem!("u64", u64, 8);
impl_const_elem!("u128", u128, 16);
impl_const_elem!("i16", i16, 2);
impl_const_elem!("i32", i32, 4);
impl_const_elem!("i64", i64, 8);
impl_const_elem!("i128", i128, 16);

#[cfg(feature = "u8")]
impl ConstElem for u8 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u8(*self).expect("write u8");
  }
  fn value_kind() -> ValueKind { ValueKind::U8 }
  fn align() -> u8 { 1 }
} 

#[cfg(feature = "i8")]
impl ConstElem for i8 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_i8(*self).expect("write i8");
  }
  fn value_kind() -> ValueKind { ValueKind::I8 }
  fn align() -> u8 { 1 }
}

#[cfg(feature = "rational")]
impl ConstElem for RationalNumber {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_i64::<LittleEndian>(*self.numer()).expect("write rational numer");
    out.write_i64::<LittleEndian>(*self.denom()).expect("write rational denom");
  }
  fn value_kind() -> ValueKind { ValueKind::RationalNumber }
  fn align() -> u8 { 16 }
}

#[cfg(feature = "complex")]
impl ConstElem for ComplexNumber {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_f64::<LittleEndian>(self.0.re).expect("write complex real");
    out.write_f64::<LittleEndian>(self.0.im).expect("write complex imag");
  }
  fn value_kind() -> ValueKind { ValueKind::ComplexNumber }
  fn align() -> u8 { 16 }
}
