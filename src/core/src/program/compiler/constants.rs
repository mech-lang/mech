use crate::*;
use super::*;

// CompileConst Trait
// ----------------------------------------------------------------------------

pub trait CompileConst {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32>;
}

#[cfg(feature = "compiler")]
impl CompileConst for Value {

  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();

    match self {
      #[cfg(feature = "bool")]
      Value::Bool(x) => payload.write_u8(if *x.borrow() { 1 } else { 0 })?,
      #[cfg(feature = "string")]
      Value::String(x) => {
        let string_brrw = x.borrow();
        let bytes = string_brrw.as_bytes();
        payload.write_u32::<LittleEndian>(bytes.len() as u32)?;
        payload.extend_from_slice(bytes);
      },
      #[cfg(feature = "u8")]
      Value::U8(x) => payload.write_u8(*x.borrow())?,
      #[cfg(feature = "u16")]
      Value::U16(x) => payload.write_u16::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "u32")]
      Value::U32(x) => payload.write_u32::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "u64")]
      Value::U64(x) => payload.write_u64::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "u128")]
      Value::U128(x) => payload.write_u128::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "i8")]
      Value::I8(x) => payload.write_i8(*x.borrow())?,
      #[cfg(feature = "i16")]
      Value::I16(x) => payload.write_i16::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "i32")]
      Value::I32(x) => payload.write_i32::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "i64")]
      Value::I64(x) => payload.write_i64::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "i128")]
      Value::I128(x) => payload.write_i128::<LittleEndian>(*x.borrow())?,
      #[cfg(feature = "f32")]
      Value::F32(x) => payload.write_f32::<LittleEndian>(x.borrow().0)?,
      #[cfg(feature = "f64")]
      Value::F64(x) => payload.write_f64::<LittleEndian>(x.borrow().0)?,
      #[cfg(feature = "atom")]
      Value::Atom(x) => payload.write_u64::<LittleEndian>(*x)?,
      #[cfg(feature = "index")]
      Value::Index(x) => payload.write_u64::<LittleEndian>(*x.borrow() as u64)?,
      #[cfg(feature = "complex")]
      Value::ComplexNumber(x) => {
        let c = x.borrow();
        payload.write_f64::<LittleEndian>(c.0.re)?;
        payload.write_f64::<LittleEndian>(c.0.im)?;
      },
      #[cfg(feature = "rational")]
      Value::RationalNumber(x) => {
        let r = x.borrow();
        payload.write_i64::<LittleEndian>(*r.numer())?;
        payload.write_i64::<LittleEndian>(*r.denom())?;
      },
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x) => todo!(), //{return x.compile_const(ctx);}
      _ => todo!(),
    }
    ctx.compile_const(&payload, self.kind())
  }
}

#[cfg(feature = "compiler")]
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

macro_rules! impl_compile_const_matrix {
  ($matrix_type:ty) => {
    impl<T> CompileConst for $matrix_type
    where
      T: ConstElem,
    {
      fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
        let rows = self.nrows() as u32;
        let cols = self.ncols() as u32;
        let mut payload = Vec::<u8>::with_capacity((rows * cols) as usize * 8);

        // write header: rows, cols
        payload.write_u32::<LittleEndian>(rows)?;
        payload.write_u32::<LittleEndian>(cols)?;

        // write elements column-major
        for c in 0..cols as usize {
          for r in 0..rows as usize {
            self[(r, c)].write_le(&mut payload);
          }
        }
        let elem_vk = T::value_kind();
        let mat_vk = ValueKind::Matrix(Box::new(elem_vk), vec![rows as usize, cols as usize]);
        ctx.compile_const(&payload, mat_vk)
      }
    }
  };
}

#[cfg(feature = "matrix1")]
impl_compile_const_matrix!(na::Matrix1<T>);
#[cfg(feature = "matrix2")]
impl_compile_const_matrix!(na::Matrix2<T>);
#[cfg(feature = "matrix3")]
impl_compile_const_matrix!(na::Matrix3<T>);
#[cfg(feature = "matrix4")]
impl_compile_const_matrix!(na::Matrix4<T>);
#[cfg(feature = "matrix2x3")]
impl_compile_const_matrix!(na::Matrix2x3<T>);
#[cfg(feature = "matrix3x2")]
impl_compile_const_matrix!(na::Matrix3x2<T>);
#[cfg(feature = "row_vector2")]
impl_compile_const_matrix!(na::RowVector2<T>);
#[cfg(feature = "row_vector3")]
impl_compile_const_matrix!(na::RowVector3<T>);
#[cfg(feature = "row_vector4")]
impl_compile_const_matrix!(na::RowVector4<T>);
#[cfg(feature = "vector2")]
impl_compile_const_matrix!(na::Vector2<T>);
#[cfg(feature = "vector3")]
impl_compile_const_matrix!(na::Vector3<T>);
#[cfg(feature = "vector4")]
impl_compile_const_matrix!(na::Vector4<T>);
#[cfg(feature = "matrixd")]
impl_compile_const_matrix!(na::DMatrix<T>);
#[cfg(feature = "vectord")]
impl_compile_const_matrix!(na::DVector<T>);
#[cfg(feature = "row_vectord")]
impl_compile_const_matrix!(na::RowDVector<T>);

// ConstElem Trait
// ----------------------------------------------------------------------------

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
