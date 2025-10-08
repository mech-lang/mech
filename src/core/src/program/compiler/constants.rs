use crate::*;
use super::*;
#[cfg(feature = "matrix")]
use crate::structures::Matrix;

// CompileConst Trait
// ----------------------------------------------------------------------------

pub trait CompileConst {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32>;
}

#[cfg(feature = "compiler")]
impl CompileConst for Value {

  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let reg = match self {
      #[cfg(feature = "bool")]
      Value::Bool(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "string")]
      Value::String(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "u8")]
      Value::U8(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "u16")]
      Value::U16(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "u32")]
      Value::U32(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "u64")]
      Value::U64(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "u128")]
      Value::U128(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "i8")]
      Value::I8(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "i16")]
      Value::I16(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "i32")]
      Value::I32(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "i64")]
      Value::I64(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "i128")]
      Value::I128(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "f32")]
      Value::F32(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "f64")]
      Value::F64(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "atom")]
      Value::Atom(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "index")]
      Value::Index(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "complex")]
      Value::C64(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "rational")]
      Value::R64(x) => x.borrow().compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "f64"))]
      Value::MatrixF64(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "f32"))]
      Value::MatrixF32(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "u8"))]
      Value::MatrixU8(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "u16"))]
      Value::MatrixU16(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "u32"))]
      Value::MatrixU32(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "u64"))]
      Value::MatrixU64(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "u128"))]
      Value::MatrixU128(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "i8"))]
      Value::MatrixI8(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "i16"))]
      Value::MatrixI16(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "i32"))]
      Value::MatrixI32(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "i64"))]
      Value::MatrixI64(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "i128"))]
      Value::MatrixI128(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "bool"))]
      Value::MatrixBool(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "rational"))]
      Value::MatrixR64(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "complex"))]
      Value::MatrixC64(x) => x.compile_const(ctx)?,
      #[cfg(all(feature = "matrix", feature = "string"))]
      Value::MatrixString(x) => x.compile_const(ctx)?,
      #[cfg(feature = "matrix")]
      Value::MatrixIndex(x) => x.compile_const(ctx)?,
      #[cfg(feature = "matrix")]
      Value::MatrixValue(x) => x.compile_const(ctx)?,
      #[cfg(feature = "table")]
      Value::Table(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "record")]
      Value::Record(x) => x.borrow().compile_const(ctx)?,
      #[cfg(feature = "set")]
      Value::Set(x) => x.borrow().compile_const(ctx)?,
      x => todo!("CompileConst not implemented for {:?}", x),
    };
    Ok(reg)
  }
}

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

impl CompileConst for usize {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_u64::<LittleEndian>(*self as u64)?;
    ctx.compile_const(&payload, ValueKind::Index)
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

#[cfg(feature = "u16")]
impl_compile_const!("u16", u16);
#[cfg(feature = "u32")]
impl_compile_const!("u32", u32);
#[cfg(feature = "u64")]
impl_compile_const!("u64", u64);
#[cfg(feature = "u128")]
impl_compile_const!("u128", u128);
#[cfg(feature = "i16")]
impl_compile_const!("i16", i16);
#[cfg(feature = "i32")]
impl_compile_const!("i32", i32);
#[cfg(feature = "i64")]
impl_compile_const!("i64", i64);
#[cfg(feature = "i128")]
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
impl CompileConst for R64 {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_i64::<LittleEndian>(*self.numer())?;
    payload.write_i64::<LittleEndian>(*self.denom())?;
    ctx.compile_const(&payload, ValueKind::R64)
  }
}

#[cfg(feature = "complex")]
impl CompileConst for C64 {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_f64::<LittleEndian>(self.0.re)?;
    payload.write_f64::<LittleEndian>(self.0.im)?;
    ctx.compile_const(&payload, ValueKind::C64)
  }
}

macro_rules! impl_compile_const_matrix {
  ($matrix_type:ty) => {
    impl<T> CompileConst for $matrix_type
    where
      T: ConstElem + AsValueKind,
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
        let elem_vk = T::as_value_kind();
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

#[cfg(feature = "matrix")]
impl<T> CompileConst for Matrix<T> 
where
  T: CompileConst + ConstElem + AsValueKind
{
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    match self {
      #[cfg(feature = "matrixd")]
      Matrix::DMatrix(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "vectord")]
      Matrix::DVector(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "row_vectord")]
      Matrix::RowDVector(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "vector2")]
      Matrix::Vector2(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "vector3")]
      Matrix::Vector3(mat) => mat.borrow().compile_const(ctx),
      #[cfg(feature = "vector4")]
      Matrix::Vector4(mat) => mat.borrow().compile_const(ctx),
    }
  }
}

#[cfg(feature = "matrixd")]
impl<T> CompileConst for Ref<DMatrix<T>> 
where
  T: CompileConst + ConstElem + AsValueKind
{
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    self.borrow().compile_const(ctx)
  }
}

#[cfg(feature = "vectord")]
impl<T> CompileConst for Ref<DVector<T>> 
where
  T: CompileConst + ConstElem + AsValueKind
{
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    self.borrow().compile_const(ctx)
  }
}

#[cfg(feature = "row_vectord")]
impl<T> CompileConst for Ref<RowDVector<T>> 
where
  T: CompileConst + ConstElem + AsValueKind
{
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    self.borrow().compile_const(ctx)
  }
}

#[cfg(feature = "record")]
impl CompileConst for MechRecord {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();

    // write the number of columns
    payload.write_u32::<LittleEndian>(self.cols as u32)?;

    // write each column: (name hash, value kind, data)
    for (col_id, value) in self.data.iter() {
      // column name hash
      payload.write_u64::<LittleEndian>(*col_id)?;
      // value kind
      let value_kind = value.kind();
      value_kind.write_le(&mut payload);
      // value data
      value.write_le(&mut payload);
    }

    // Write the field name strings into the payload
    for (_col_id, col_name) in self.field_names.iter() {
      col_name.write_le(&mut payload);
    }
    ctx.compile_const(&payload, self.kind())
  }
}

#[cfg(feature = "enum")]
impl CompileConst for MechEnum {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();

    // write the enum id
    payload.write_u64::<LittleEndian>(self.id)?;

    // write the number of variants
    payload.write_u32::<LittleEndian>(self.variants.len() as u32)?;

    // write each variant: (variant id, has value, value data)
    for (variant_id, variant_value) in self.variants.iter() {
      // variant id
      payload.write_u64::<LittleEndian>(*variant_id)?;
      match variant_value {
        Some(v) => {
          // has value
          payload.write_u8(1)?;
          // value kind
          let value_kind = v.kind();
          value_kind.write_le(&mut payload);
          // value data
          v.write_le(&mut payload);
        },
        None => {
          // has no value
          payload.write_u8(0)?;
        }
      }
    }
    ctx.compile_const(&payload, ValueKind::Enum(self.id))
  }
}

#[cfg(feature = "atom")]
impl CompileConst for MechAtom {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    payload.write_u64::<LittleEndian>(self.0)?;
    ctx.compile_const(&payload, ValueKind::Atom(self.0))
  }
}

#[cfg(feature = "set")]
impl CompileConst for MechSet {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    // include the kind to match write_le/from_le
    self.kind.write_le(&mut payload);
    // write the number of elements
    payload.write_u32::<LittleEndian>(self.num_elements as u32)?;
    // write each element
    for element in &self.set {
      element.write_le(&mut payload);
    }
    ctx.compile_const(&payload, self.kind())
  }
}


// ConstElem Trait
// ----------------------------------------------------------------------------

pub trait ConstElem {
  fn write_le(&self, out: &mut Vec<u8>);
  fn from_le(bytes: &[u8]) -> Self;
  fn value_kind(&self) -> ValueKind;
  fn align() -> u8 { 1 }
}

#[cfg(feature = "f64")]
impl ConstElem for F64 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_f64::<LittleEndian>(self.0).expect("write f64");
  }
  fn from_le(bytes: &[u8]) -> Self {
    let mut rdr = std::io::Cursor::new(bytes);
    let val = rdr.read_f64::<LittleEndian>().expect("read f64");
    F64(val)
  }
  fn value_kind(&self) -> ValueKind { ValueKind::F64 }
  fn align() -> u8 { 8 }
}

#[cfg(feature = "f32")]
impl ConstElem for F32 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_f32::<LittleEndian>(self.0).expect("write f32");
  }
  fn from_le(bytes: &[u8]) -> Self {
    let mut rdr = std::io::Cursor::new(bytes);
    let val = rdr.read_f32::<LittleEndian>().expect("read f32");
    F32(val)
  }
  fn value_kind(&self) -> ValueKind { ValueKind::F32 }
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
        fn from_le(bytes: &[u8]) -> Self {
          let mut rdr = std::io::Cursor::new(bytes);
          rdr.[<read_ $t>]::<LittleEndian>().expect(concat!("read ", stringify!($t)))
        }
        fn value_kind(&self) -> ValueKind { ValueKind::[<$t:upper>] }
        fn align() -> u8 { $align }
      }
    }
  };
}

#[cfg(feature = "u16")]
impl_const_elem!("u16", u16, 2);
#[cfg(feature = "u32")]
impl_const_elem!("u32", u32, 4);
#[cfg(feature = "u64")]
impl_const_elem!("u64", u64, 8);
#[cfg(feature = "u128")]
impl_const_elem!("u128", u128, 16);
#[cfg(feature = "i16")]
impl_const_elem!("i16", i16, 2);
#[cfg(feature = "i32")]
impl_const_elem!("i32", i32, 4);
#[cfg(feature = "i64")]
impl_const_elem!("i64", i64, 8);
#[cfg(feature = "i128")]
impl_const_elem!("i128", i128, 16);

#[cfg(feature = "u8")]
impl ConstElem for u8 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u8(*self).expect("write u8");
  }
  fn from_le(bytes: &[u8]) -> Self {
    bytes[0]
  }
  fn value_kind(&self) -> ValueKind { ValueKind::U8 }
  fn align() -> u8 { 1 }
} 

#[cfg(feature = "i8")]
impl ConstElem for i8 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_i8(*self).expect("write i8");
  }
  fn from_le(bytes: &[u8]) -> Self {
    bytes[0] as i8
  }
  fn value_kind(&self) -> ValueKind { ValueKind::I8 }
  fn align() -> u8 { 1 }
}

#[cfg(feature = "rational")]
impl ConstElem for R64 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_i64::<LittleEndian>(*self.numer()).expect("write rational numer");
    out.write_i64::<LittleEndian>(*self.denom()).expect("write rational denom");
  }
  fn from_le(bytes: &[u8]) -> Self {
    let numer = match bytes[0..8].try_into() {
      Ok(arr) => i64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read numerator from bytes"),
    };
    let denom = match bytes[8..16].try_into() {
      Ok(arr) => i64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read denominator from bytes"),
    };
    if denom == 0 {
      panic!("Denominator cannot be zero");
    }
    R64::new(numer, denom)
  }
  fn value_kind(&self) -> ValueKind { ValueKind::R64 }
  fn align() -> u8 { 16 }
}

#[cfg(feature = "complex")]
impl ConstElem for C64 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_f64::<LittleEndian>(self.0.re).expect("write complex real");
    out.write_f64::<LittleEndian>(self.0.im).expect("write complex imag");
  }
  fn from_le(bytes: &[u8]) -> Self {
    let real = match bytes[0..8].try_into() {
      Ok(arr) => f64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read real part from bytes"),
    };
    let imag = match bytes[8..16].try_into() {
      Ok(arr) => f64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read imaginary part from bytes"),
    };
    C64::new(real, imag)
  }
  fn value_kind(&self) -> ValueKind { ValueKind::C64 }
  fn align() -> u8 { 16 }
}

#[cfg(feature = "string")]
impl ConstElem for String {
  fn write_le(&self, out: &mut Vec<u8>) {
    use byteorder::{LittleEndian, WriteBytesExt};
    out.write_u32::<LittleEndian>(self.len() as u32).expect("write string length");
    out.extend_from_slice(self.as_bytes());
  }
  fn from_le(bytes: &[u8]) -> Self {
    use byteorder::{LittleEndian, ReadBytesExt};
    use std::io::Cursor;
    let mut cursor = Cursor::new(bytes);
    // read length safely
    let len = match cursor.read_u32::<LittleEndian>() {
      Ok(n) => n as usize,
      Err(_) => panic!("Failed to read string length from bytes"),
    };
    let start = cursor.position() as usize;
    let end = start + len;
    if end > bytes.len() {
      panic!(
        "String::from_le: declared length {} exceeds available bytes ({})",
        len, bytes.len()
      );
    }
    let str_bytes = &bytes[start..end];
    match std::str::from_utf8(str_bytes) {
      Ok(s) => s.to_string(),
      Err(_) => panic!("Failed to convert bytes to UTF-8 string"),
    }
  }
  fn value_kind(&self) -> ValueKind { ValueKind::String }
  fn align() -> u8 { 1 }
}

#[cfg(feature = "bool")]
impl ConstElem for bool {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u8(if *self { 1 } else { 0 }).expect("write bool");
  }
  fn from_le(bytes: &[u8]) -> Self {
    bytes[0] != 0
  }
  fn value_kind(&self) -> ValueKind { ValueKind::Bool }
  fn align() -> u8 { 1 }
}

impl ConstElem for usize {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u64::<LittleEndian>(*self as u64).expect("write usize");
  }
  fn from_le(bytes: &[u8]) -> Self {
    let val = match bytes[0..8].try_into() {
      Ok(arr) => u64::from_le_bytes(arr),
      Err(_) => panic!("Failed to read usize from bytes"),
    };
    val as usize
  }
  fn value_kind(&self) -> ValueKind { ValueKind::Index }
  fn align() -> u8 { 8 }
}

macro_rules! impl_const_elem_matrix {
  ($matrix_type:ty) => {
    impl<T> ConstElem for $matrix_type
    where
      T: ConstElem + std::fmt::Debug + std::clone::Clone + PartialEq + 'static,
    {
      fn write_le(&self, out: &mut Vec<u8>) {
        out.write_u32::<LittleEndian>(self.nrows() as u32).unwrap();
        out.write_u32::<LittleEndian>(self.ncols() as u32).unwrap();
        for c in 0..self.ncols() {
          for r in 0..self.nrows() {
            self[(r, c)].write_le(out);
          }
        }
      }
      fn from_le(bytes: &[u8]) -> Self {
        let mut cursor = Cursor::new(bytes);
        let rows = cursor.read_u32::<LittleEndian>().unwrap() as usize;
        let cols = cursor.read_u32::<LittleEndian>().unwrap() as usize;
        let mut elements: Vec<T> = Vec::with_capacity(rows * cols);

        // Read in column-major order
        for _c in 0..cols {
          for _r in 0..rows {
            let elem = T::from_le(&bytes[cursor.position() as usize..]);
            let mut buf = Vec::new();
            elem.write_le(&mut buf);
            cursor.set_position(cursor.position() + buf.len() as u64);
            elements.push(elem);
          }
        }
        // Now construct the fixed-size matrix
        // All nalgebra fixed-size matrices implement `from_row_slice`
        <$matrix_type>::from_row_slice(&elements)
      }
      fn value_kind(&self) -> ValueKind { self.value_kind() }
      fn align() -> u8 { 8 }
    }
  };
}

impl<T> ConstElem for DMatrix<T>
where
  T: ConstElem + std::fmt::Debug + std::clone::Clone + PartialEq + 'static,
{
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u32::<LittleEndian>(self.nrows() as u32).unwrap();
    out.write_u32::<LittleEndian>(self.ncols() as u32).unwrap();
    for c in 0..self.ncols() {
      for r in 0..self.nrows() {
        self[(r, c)].write_le(out);
      }
    }
  }
  fn from_le(bytes: &[u8]) -> Self {
    let mut cursor = Cursor::new(bytes);
    let rows = cursor.read_u32::<LittleEndian>().unwrap() as usize;
    let cols = cursor.read_u32::<LittleEndian>().unwrap() as usize;
    let mut elements = Vec::with_capacity(rows * cols);
    // Read in column-major order
    for _c in 0..cols {
      for _r in 0..rows {
        let elem = T::from_le(&bytes[cursor.position() as usize..]);
        let mut buf = Vec::new();
        elem.write_le(&mut buf);
        cursor.set_position(cursor.position() + buf.len() as u64);
        elements.push(elem);
      }
    }
    DMatrix::from_vec(rows, cols, elements)
  }
  fn value_kind(&self) -> ValueKind { self.value_kind() }
  fn align() -> u8 { 8 }
}

impl<T> ConstElem for DVector<T>
where
  T: ConstElem + std::fmt::Debug + std::clone::Clone + PartialEq + 'static,
{
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u32::<LittleEndian>(self.nrows() as u32).unwrap();
    out.write_u32::<LittleEndian>(self.ncols() as u32).unwrap();
    for c in 0..self.ncols() {
      for r in 0..self.nrows() {
        self[(r, c)].write_le(out);
      }
    }
  }
  fn from_le(bytes: &[u8]) -> Self {
    let mut cursor = Cursor::new(bytes);
    let rows = cursor.read_u32::<LittleEndian>().unwrap() as usize;
    let cols = cursor.read_u32::<LittleEndian>().unwrap() as usize;
    let mut elements = Vec::with_capacity(rows * cols);
    // Read in column-major order
    for _c in 0..cols {
      for _r in 0..rows {
        let elem = T::from_le(&bytes[cursor.position() as usize..]);
        let mut buf = Vec::new();
        elem.write_le(&mut buf);
        cursor.set_position(cursor.position() + buf.len() as u64);
        elements.push(elem);
      }
    }
    DVector::from_vec(elements)
  }
  fn value_kind(&self) -> ValueKind { self.value_kind() }
  fn align() -> u8 { 8 }
}

impl<T> ConstElem for RowDVector<T>
where
  T: ConstElem + std::fmt::Debug + std::clone::Clone + PartialEq + 'static,
{
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u32::<LittleEndian>(self.nrows() as u32).unwrap();
    out.write_u32::<LittleEndian>(self.ncols() as u32).unwrap();
    for c in 0..self.ncols() {
      for r in 0..self.nrows() {
        self[(r, c)].write_le(out);
      }
    }
  }
  fn from_le(bytes: &[u8]) -> Self {
    let mut cursor = Cursor::new(bytes);
    let rows = cursor.read_u32::<LittleEndian>().unwrap() as usize;
    let cols = cursor.read_u32::<LittleEndian>().unwrap() as usize;
    let mut elements = Vec::with_capacity(rows * cols);
    // Read in column-major order
    for _c in 0..cols {
      for _r in 0..rows {
        let elem = T::from_le(&bytes[cursor.position() as usize..]);
        let mut buf = Vec::new();
        elem.write_le(&mut buf);
        cursor.set_position(cursor.position() + buf.len() as u64);
        elements.push(elem);
      }
    }
    RowDVector::from_vec(elements)
  }
  fn value_kind(&self) -> ValueKind { self.value_kind() }
  fn align() -> u8 { 8 }
}

#[cfg(feature = "matrix1")]
impl_const_elem_matrix!(Matrix1<T>);
#[cfg(feature = "matrix2")]
impl_const_elem_matrix!(Matrix2<T>);
#[cfg(feature = "matrix3")]
impl_const_elem_matrix!(Matrix3<T>);
#[cfg(feature = "matrix4")]
impl_const_elem_matrix!(Matrix4<T>);
#[cfg(feature = "matrix2x3")]
impl_const_elem_matrix!(Matrix2x3<T>);
#[cfg(feature = "matrix3x2")]
impl_const_elem_matrix!(Matrix3x2<T>);
#[cfg(feature = "row_vector2")]
impl_const_elem_matrix!(RowVector2<T>);
#[cfg(feature = "row_vector3")]
impl_const_elem_matrix!(RowVector3<T>);
#[cfg(feature = "row_vector4")]
impl_const_elem_matrix!(RowVector4<T>);
#[cfg(feature = "vector2")]
impl_const_elem_matrix!(Vector2<T>);
#[cfg(feature = "vector3")]
impl_const_elem_matrix!(Vector3<T>);
#[cfg(feature = "vector4")]
impl_const_elem_matrix!(Vector4<T>);

#[cfg(feature = "matrix")]
impl<T> ConstElem for Matrix<T> 
where
  T: ConstElem + std::fmt::Debug + std::clone::Clone + PartialEq + 'static,
{
  fn write_le(&self, out: &mut Vec<u8>) {
    match self {
      #[cfg(feature = "matrixd")]
      Matrix::DMatrix(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "vectord")]
      Matrix::DVector(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "row_vectord")]
      Matrix::RowDVector(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "matrix1")]
      Matrix::Matrix1(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "matrix2")]
      Matrix::Matrix2(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "matrix3")]
      Matrix::Matrix3(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "matrix4")]
      Matrix::Matrix4(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "matrix2x3")]
      Matrix::Matrix2x3(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "matrix3x2")]
      Matrix::Matrix3x2(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "row_vector2")]
      Matrix::RowVector2(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "row_vector3")]
      Matrix::RowVector3(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "row_vector4")]
      Matrix::RowVector4(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "vector2")]
      Matrix::Vector2(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "vector3")]
      Matrix::Vector3(mat) => mat.borrow().write_le(out),
      #[cfg(feature = "vector4")]
      Matrix::Vector4(mat) => mat.borrow().write_le(out),
    }
  }
  fn from_le(bytes: &[u8]) -> Self {
    let mut cursor = Cursor::new(bytes);
    let rows = cursor.read_u32::<LittleEndian>().unwrap() as usize;
    let cols = cursor.read_u32::<LittleEndian>().unwrap() as usize;
    let mut elements = Vec::with_capacity(rows * cols);
    // Read in column-major order
    for _c in 0..cols {
      for _r in 0..rows {
        let elem = T::from_le(&bytes[cursor.position() as usize..]);
        let mut buf = Vec::new();
        elem.write_le(&mut buf);
        cursor.set_position(cursor.position() + buf.len() as u64);
        elements.push(elem);
      }
    }
    if rows == 0 || cols == 0 {
      panic!("Cannot create Matrix with zero rows or columns");
    } else if cols == 1 {
      match rows {
        #[cfg(feature = "matrix1")]
        1 => Matrix::Matrix1(Ref::new(Matrix1::from_vec(elements))),
        #[cfg(all(feature = "matrixd", not(feature = "matrix1")))]
        1 => Matrix::DMatrix(Ref::new(DMatrix::from_vec(1,1, elements))),
        #[cfg(feature = "vector2")]
        2 => Matrix::Vector2(Ref::new(Vector2::from_vec(elements))),
        #[cfg(feature = "vector3")]
        3 => Matrix::Vector3(Ref::new(Vector3::from_vec(elements))),
        #[cfg(feature = "vector4")]
        4 => Matrix::Vector4(Ref::new(Vector4::from_vec(elements))),
        #[cfg(feature = "vectord")]
        _ => Matrix::DVector(Ref::new(DVector::from_vec(elements))),
      }
    } else if rows == 1 {
      match cols {
        #[cfg(feature = "row_vector2")]
        2 => Matrix::RowVector2(Ref::new(RowVector2::from_vec(elements))),
        #[cfg(feature = "row_vector3")]
        3 => Matrix::RowVector3(Ref::new(RowVector3::from_vec(elements))),
        #[cfg(feature = "row_vector4")]
        4 => Matrix::RowVector4(Ref::new(RowVector4::from_vec(elements))),
        #[cfg(feature = "row_vectord")]
        _ => Matrix::RowDVector(Ref::new(RowDVector::from_vec(elements))),
      }
    } else {
      match (rows, cols) {
        #[cfg(feature = "matrix1")]
        (1, 1) => Matrix::Matrix1(Ref::new(Matrix1::from_row_slice(&elements))),
        #[cfg(feature = "matrix2")]
        (2, 2) => Matrix::Matrix2(Ref::new(Matrix2::from_row_slice(&elements))),
        #[cfg(feature = "matrix3")]
        (3, 3) => Matrix::Matrix3(Ref::new(Matrix3::from_row_slice(&elements))),
        #[cfg(feature = "matrix4")]
        (4, 4) => Matrix::Matrix4(Ref::new(Matrix4::from_row_slice(&elements))),
        #[cfg(feature = "matrix2x3")]
        (2, 3) => Matrix::Matrix2x3(Ref::new(Matrix2x3::from_row_slice(&elements))),
        #[cfg(feature = "matrix3x2")]
        (3, 2) => Matrix::Matrix3x2(Ref::new(Matrix3x2::from_row_slice(&elements))),
        #[cfg(feature = "matrixd")]
        _ => Matrix::DMatrix(Ref::new(DMatrix::from_vec(rows, cols, elements))),
      }
    }
  }
  fn value_kind(&self) -> ValueKind { self.value_kind() }
  fn align() -> u8 { T::align() }
}


impl ConstElem for Value {
  fn write_le(&self, out: &mut Vec<u8>) {
    // Write the kind tag first
    self.kind().write_le(out);

    // Then write the payload
    match self {
      Value::Empty => { 
        // no payload for Empty 
      },
      #[cfg(feature = "bool")]
      Value::Bool(x) => x.borrow().write_le(out),
      #[cfg(feature = "string")]
      Value::String(x) => x.borrow().write_le(out),
      #[cfg(feature = "u8")]
      Value::U8(x) => x.borrow().write_le(out),
      #[cfg(feature = "u16")]
      Value::U16(x) => x.borrow().write_le(out),
      #[cfg(feature = "u32")]
      Value::U32(x) => x.borrow().write_le(out),
      #[cfg(feature = "u64")]
      Value::U64(x) => x.borrow().write_le(out),
      #[cfg(feature = "u128")]
      Value::U128(x) => x.borrow().write_le(out),
      #[cfg(feature = "i8")]
      Value::I8(x) => x.borrow().write_le(out),
      #[cfg(feature = "i16")]
      Value::I16(x) => x.borrow().write_le(out),
      #[cfg(feature = "i32")]
      Value::I32(x) => x.borrow().write_le(out),
      #[cfg(feature = "i64")]
      Value::I64(x) => x.borrow().write_le(out),
      #[cfg(feature = "i128")]
      Value::I128(x) => x.borrow().write_le(out),
      #[cfg(feature = "f32")]
      Value::F32(x) => x.borrow().write_le(out),
      #[cfg(feature = "f64")]
      Value::F64(x) => x.borrow().write_le(out),
      #[cfg(feature = "rational")]
      Value::R64(x) => x.borrow().write_le(out),
      #[cfg(feature = "complex")]
      Value::C64(x) => x.borrow().write_le(out),
      #[cfg(feature = "set")]
      Value::Set(x) => x.borrow().write_le(out),
      _ => unimplemented!("write_le not implemented for this Value variant"),
    }
  }
  fn from_le(bytes: &[u8]) -> Self {
    let mut cursor = std::io::Cursor::new(bytes);

    // 1. read ValueKind
    let kind = ValueKind::from_le(cursor.get_ref());

    // 2. determine the offset of the payload (length of encoded kind)
    let mut kind_buf = Vec::new();
    kind.write_le(&mut kind_buf);
    let payload = &bytes[kind_buf.len()..];

    // 3. dispatch based on ValueKind
    match kind {
      ValueKind::Empty => Value::Empty,
      #[cfg(feature = "bool")]
      ValueKind::Bool => Value::Bool(Ref::new(<bool as ConstElem>::from_le(payload))),
      #[cfg(feature = "string")]
      ValueKind::String => Value::String(Ref::new(<String as ConstElem>::from_le(payload))),
      #[cfg(feature = "u8")]
      ValueKind::U8 => Value::U8(Ref::new(<u8 as ConstElem>::from_le(payload))),
      #[cfg(feature = "u16")]
      ValueKind::U16 => Value::U16(Ref::new(<u16 as ConstElem>::from_le(payload))),
      #[cfg(feature = "u32")]
      ValueKind::U32 => Value::U32(Ref::new(<u32 as ConstElem>::from_le(payload))),
      #[cfg(feature = "u64")]
      ValueKind::U64 => Value::U64(Ref::new(<u64 as ConstElem>::from_le(payload))),
      #[cfg(feature = "u128")]
      ValueKind::U128 => Value::U128(Ref::new(<u128 as ConstElem>::from_le(payload))),
      #[cfg(feature = "i8")]
      ValueKind::I8 => Value::I8(Ref::new(<i8 as ConstElem>::from_le(payload))),
      #[cfg(feature = "i16")]
      ValueKind::I16 => Value::I16(Ref::new(<i16 as ConstElem>::from_le(payload))),
      #[cfg(feature = "i32")]
      ValueKind::I32 => Value::I32(Ref::new(<i32 as ConstElem>::from_le(payload))),
      #[cfg(feature = "i64")]
      ValueKind::I64 => Value::I64(Ref::new(<i64 as ConstElem>::from_le(payload))),
      #[cfg(feature = "i128")]
      ValueKind::I128 => Value::I128(Ref::new(<i128 as ConstElem>::from_le(payload))),
      #[cfg(feature = "f32")]
      ValueKind::F32 => Value::F32(Ref::new(<F32 as ConstElem>::from_le(payload))),
      #[cfg(feature = "f64")]
      ValueKind::F64 => Value::F64(Ref::new(<F64 as ConstElem>::from_le(payload))),
      #[cfg(feature = "rational")]
      ValueKind::R64 => Value::R64(Ref::new(<R64 as ConstElem>::from_le(payload))),
      #[cfg(feature = "complex")]
      ValueKind::C64 => Value::C64(Ref::new(<C64 as ConstElem>::from_le(payload))),
      x => unimplemented!("from_le not implemented for this ValueKind variant: {:?}", x),
    }
  }
  fn value_kind(&self) -> ValueKind {
    self.value_kind()
  }
  fn align() -> u8 {
    1
  }
}

impl ConstElem for ValueKind {
  fn write_le(&self, out: &mut Vec<u8>) {
    match self {
      ValueKind::U8 => out.write_u8(1).expect("write value kind"),
      ValueKind::U16 => out.write_u8(2).expect("write value kind"),
      ValueKind::U32 => out.write_u8(3).expect("write value kind"),
      ValueKind::U64 => out.write_u8(4).expect("write value kind"),
      ValueKind::U128 => out.write_u8(5).expect("write value kind"),
      ValueKind::I8 => out.write_u8(6).expect("write value kind"),
      ValueKind::I16 => out.write_u8(7).expect("write value kind"),
      ValueKind::I32 => out.write_u8(8).expect("write value kind"),
      ValueKind::I64 => out.write_u8(9).expect("write value kind"),
      ValueKind::I128 => out.write_u8(10).expect("write value kind"),
      ValueKind::F32 => out.write_u8(11).expect("write value kind"),
      ValueKind::F64 => out.write_u8(12).expect("write value kind"),
      ValueKind::C64 => out.write_u8(13).expect("write value kind"),
      ValueKind::R64 => out.write_u8(14).expect("write value kind"),
      ValueKind::String => out.write_u8(15).expect("write value kind"),
      ValueKind::Bool => out.write_u8(16).expect("write value kind"),
      ValueKind::Id => out.write_u8(17).expect("write value kind"),
      ValueKind::Index => out.write_u8(18).expect("write value kind"),
      ValueKind::Empty => out.write_u8(19).expect("write value kind"),
      ValueKind::Any => out.write_u8(20).expect("write value kind"),
      ValueKind::Matrix(elem_vk, dims) => {
        out.write_u8(21).expect("write value kind");
        elem_vk.write_le(out);
        out.write_u32::<LittleEndian>(dims.len() as u32).expect("write matrix dims length");
        for d in dims.iter() {
          out.write_u32::<LittleEndian>(*d as u32).expect("write matrix dim");
        }
      },
      ValueKind::Enum(id) => {
        out.write_u8(22).expect("write value kind");
        out.write_u64::<LittleEndian>(*id).expect("write enum id");
      },
      ValueKind::Record(fields) => {
        out.write_u8(23).expect("write value kind");
        out.write_u32::<LittleEndian>(fields.len() as u32).expect("write record fields length");
        for (name, vk) in fields.iter() {
          name.write_le(out);
          vk.write_le(out);
        }
      },
      ValueKind::Map(key_vk, val_vk) => {
        out.write_u8(24).expect("write value kind");
        key_vk.write_le(out);
        val_vk.write_le(out);
      },
      ValueKind::Atom(id) => {
        out.write_u8(25).expect("write value kind");
        out.write_u64::<LittleEndian>(*id).expect("write atom id");
      },
      ValueKind::Table(fields, row_count) => {
        out.write_u8(26).expect("write value kind");
        out.write_u32::<LittleEndian>(fields.len() as u32).expect("write table fields length");
        for (name, vk) in fields.iter() {
          name.write_le(out);
          vk.write_le(out);
        }
        out.write_u32::<LittleEndian>(*row_count as u32).expect("write table row count");
      },
      ValueKind::Tuple(vks) => {
        out.write_u8(27).expect("write value kind");
        out.write_u32::<LittleEndian>(vks.len() as u32).expect("write tuple length");
        for vk in vks.iter() {
          vk.write_le(out);
        }
      },
      ValueKind::Reference(vk) => {
        out.write_u8(28).expect("write value kind");
        vk.write_le(out);
      },
      ValueKind::Set(vk, opt_size) => {
        out.write_u8(29).expect("write value kind");
        vk.write_le(out);
        match opt_size {
          Some(sz) => {
            out.write_u8(1).expect("write set size flag");
            out.write_u32::<LittleEndian>(*sz as u32).expect("write set size");
          },
          None => {
            out.write_u8(0).expect("write set size flag");
          }
        }
      },
      ValueKind::Option(vk) => {
        out.write_u8(30).expect("write value kind");
        vk.write_le(out);
      },
    }
  }
  fn from_le(bytes: &[u8]) -> Self {
    let mut cursor = Cursor::new(bytes);
    let tag = cursor.read_u8().expect("read value kind tag");

    match tag {
      0 => ValueKind::Empty,
      1 => ValueKind::U8,
      2 => ValueKind::U16,
      3 => ValueKind::U32,
      4 => ValueKind::U64,
      5 => ValueKind::U128,
      6 => ValueKind::I8,
      7 => ValueKind::I16,
      8 => ValueKind::I32,
      9 => ValueKind::I64,
      10 => ValueKind::I128,
      11 => ValueKind::F32,
      12 => ValueKind::F64,
      13 => ValueKind::C64,
      14 => ValueKind::R64,
      15 => ValueKind::String,
      16 => ValueKind::Bool,
      17 => ValueKind::Id,
      18 => ValueKind::Index,
      19 => ValueKind::Empty,
      20 => ValueKind::Any,
      // Matrix
      21 => {
        let elem_vk = ValueKind::from_le(&bytes[cursor.position() as usize..]);
        cursor.set_position(cursor.position() + 1); // advance past elem_vk tag
        let dim_count = cursor.read_u32::<LittleEndian>().expect("read matrix dim count") as usize;
        let mut dims = Vec::with_capacity(dim_count);
        for _ in 0..dim_count {
            dims.push(cursor.read_u32::<LittleEndian>().expect("read matrix dim") as usize);
        }
        ValueKind::Matrix(Box::new(elem_vk), dims)
      }
      22 => ValueKind::Enum(cursor.read_u64::<LittleEndian>().expect("read enum id")),
      // Table
      26 => {
        let field_count = cursor.read_u32::<LittleEndian>().expect("read table fields length") as usize;
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
          let name = String::from_le(&bytes[cursor.position() as usize..]);
          let mut buf = Vec::new();
          name.write_le(&mut buf);
          cursor.set_position(cursor.position() + buf.len() as u64);
          let vk = ValueKind::from_le(&bytes[cursor.position() as usize..]);
          let mut buf = Vec::new();
          vk.write_le(&mut buf);
          cursor.set_position(cursor.position() + buf.len() as u64);
          fields.push((name, vk));
        }
        let row_count = cursor.read_u32::<LittleEndian>().expect("read table row count") as usize;
        ValueKind::Table(fields, row_count)
      }
      // Set
      29 => {
        let elem_vk = ValueKind::from_le(&bytes[cursor.position() as usize..]);
        cursor.set_position(cursor.position() + 1);
        let size_flag = cursor.read_u8().expect("read set size flag");
        let opt_size = if size_flag != 0 {
            Some(cursor.read_u32::<LittleEndian>().expect("read set size") as usize)
        } else {
            None
        };
        ValueKind::Set(Box::new(elem_vk), opt_size)
      }
      x => unimplemented!("from_le not implemented for this ValueKind variant: {:?}", x),
    }
  }
  fn value_kind(&self) -> ValueKind { self.clone() }
  fn align() -> u8 { 1 }
}

// helper to read a length-prefixed string from cursor
fn read_string_from_cursor(cursor: &mut std::io::Cursor<&[u8]>) -> Vec<u8> {
  let len = cursor.read_u32::<LittleEndian>().expect("read string len") as usize;
  let mut buf = vec![0u8; len];
  cursor.read_exact(&mut buf).expect("read string bytes");
  buf
}

#[cfg(feature = "enum")]
impl ConstElem for MechEnum {
  fn write_le(&self, out: &mut Vec<u8>) {
    // write the enum id
    out.write_u64::<LittleEndian>(self.id).expect("write enum id");

    // write the number of variants
    out.write_u32::<LittleEndian>(self.variants.len() as u32).expect("write enum variants length");

    // write each variant: (variant id, has value, value data)
    for (variant_id, variant_value) in self.variants.iter() {
      // variant id
      out.write_u64::<LittleEndian>(*variant_id).expect("write enum variant id");
      match variant_value {
        Some(v) => {
          // has value
          out.write_u8(1).expect("write enum variant has value");
          // value kind
          let value_kind = v.kind();
          value_kind.write_le(out);
          // value data
          v.write_le(out);
        },
        None => {
          // has no value
          out.write_u8(0).expect("write enum variant has no value");
        }
      }
    }
  }
  fn from_le(_bytes: &[u8]) -> Self {
    unimplemented!("from_le not implemented for MechEnum")
  }
  fn value_kind(&self) -> ValueKind { ValueKind::Enum(0) } // id 0 as placeholder
  fn align() -> u8 { 8 }
}

#[cfg(feature = "table")]
impl ConstElem for MechTable {
  fn write_le(&self, out: &mut Vec<u8>) {
    // Write kind
    self.value_kind().write_le(out);
    // Write number of rows and columns
    out.write_u32::<LittleEndian>(self.rows as u32).expect("write table rows");
    out.write_u32::<LittleEndian>(self.cols as u32).expect("write table cols");
    // Write each column: (id, kind, data, name)
    for (col_id, (vk, col_data)) in &self.data {
      // Column id
      out.write_u64::<LittleEndian>(*col_id).expect("write column id");
      // Value kind
      vk.write_le(out);
      // Column data matrix
      col_data.write_le(out);
      // Column name
      if let Some(name) = self.col_names.get(col_id) {
        name.write_le(out);
      } else {
        String::from("").write_le(out);
      }
    }
  }
  fn from_le(data: &[u8]) -> Self {
    use indexmap::IndexMap;
    let mut cursor = Cursor::new(data);
    // Kind
    let kind = ValueKind::from_le(cursor.get_ref());
    let mut buf = Vec::new();
    kind.write_le(&mut buf);
    cursor.set_position(buf.len() as u64);

    // Read row and column counts
    let rows = cursor.read_u32::<LittleEndian>().expect("read rows") as usize;
    let cols = cursor.read_u32::<LittleEndian>().expect("read cols") as usize;

    let mut data_map: IndexMap<u64, (ValueKind, Matrix<Value>)> = IndexMap::new();
    let mut col_names: HashMap<u64, String> = HashMap::new();

    // Decode each column
    for _ in 0..cols {
      let col_id = cursor.read_u64::<LittleEndian>().expect("read column id");

      // read value kind
      let kind = ValueKind::from_le(&data[cursor.position() as usize..]);
      let mut tmp = Vec::new();
      kind.write_le(&mut tmp);
      cursor.set_position(cursor.position() + tmp.len() as u64);

      // read matrix
      let matrix = Matrix::<Value>::from_le(&data[cursor.position() as usize..]);
      let mut tmp = Vec::new();
      matrix.write_le(&mut tmp);
      cursor.set_position(cursor.position() + tmp.len() as u64);

      // read column name
      let name = String::from_le(&data[cursor.position() as usize..]);
      let mut tmp = Vec::new();
      name.write_le(&mut tmp);
      cursor.set_position(cursor.position() + tmp.len() as u64);

      data_map.insert(col_id, (kind, matrix));
      col_names.insert(col_id, name);
    }

    MechTable { rows, cols, data: data_map, col_names }
  }
  fn value_kind(&self) -> ValueKind { self.kind() }
  fn align() -> u8 { 8 }
}

#[cfg(feature = "table")]
impl CompileConst for MechTable {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();
    self.value_kind().write_le(&mut payload);
    payload.write_u32::<LittleEndian>(self.rows as u32)?;
    payload.write_u32::<LittleEndian>(self.cols as u32)?;
    for (col_id, (vk, col_data)) in &self.data {
      payload.write_u64::<LittleEndian>(*col_id)?;
      vk.write_le(&mut payload);
      col_data.write_le(&mut payload);

      if let Some(name) = self.col_names.get(col_id) {
        name.write_le(&mut payload);
      } else {
        String::from("").write_le(&mut payload);
      }
    }
    ctx.compile_const(&payload, self.value_kind())
  }
}


#[cfg(feature = "set")]
impl ConstElem for MechSet {
  fn write_le(&self, out: &mut Vec<u8>) {
    // write kind
    self.kind.write_le(out);
    // write element count
    out.write_u32::<LittleEndian>(self.num_elements as u32)
      .expect("write set element count");
    // write each element
    for value in &self.set {
      value.write_le(out);
    }
  }
  fn from_le(data: &[u8]) -> Self {
    use indexmap::IndexSet;
    let mut cursor = Cursor::new(data);
    // 1) read kind from current position
    let start = cursor.position() as usize;
    let kind = ValueKind::from_le(&data[start..]);
    // compute how many bytes the kind encoding consumes (so we can advance)
    let mut kind_buf = Vec::new();
    kind.write_le(&mut kind_buf);
    cursor.set_position(start as u64 + kind_buf.len() as u64);
    // 2) element count (little endian)
    let num_elements = cursor
      .read_u32::<LittleEndian>()
      .expect("read set element count") as usize;
    // 3) read each Value (advance cursor using each value's encoded length)
    let mut set = IndexSet::with_capacity(num_elements);
    for _ in 0..num_elements {
      let pos = cursor.position() as usize;
      let value = Value::from_le(&data[pos..]);
      // measure its encoded length by re-serializing
      let mut tmp = Vec::new();
      value.write_le(&mut tmp);
      cursor.set_position(pos as u64 + tmp.len() as u64);
      set.insert(value);
    }
    Self { kind, num_elements, set }
  }
  fn value_kind(&self) -> ValueKind { self.kind.clone() }
  fn align() -> u8 { 8 }
}
