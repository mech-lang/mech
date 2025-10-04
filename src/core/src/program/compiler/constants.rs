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

#[cfg(feature = "matrix")]
impl<T> CompileConst for Matrix<T> 
where
  T: CompileConst + ConstElem
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
  T: CompileConst + ConstElem
{
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    self.borrow().compile_const(ctx)
  }
}

#[cfg(feature = "vectord")]
impl<T> CompileConst for Ref<DVector<T>> 
where
  T: CompileConst + ConstElem
{
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    self.borrow().compile_const(ctx)
  }
}

#[cfg(feature = "row_vectord")]
impl<T> CompileConst for Ref<RowDVector<T>> 
where
  T: CompileConst + ConstElem
{
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    self.borrow().compile_const(ctx)
  }
}

#[cfg(feature = "table")]
impl CompileConst for MechTable {
  fn compile_const(&self, ctx: &mut CompileCtx) -> MResult<u32> {
    let mut payload = Vec::<u8>::new();

    // write the number of rows and columns
    payload.write_u32::<LittleEndian>(self.rows as u32)?;
    payload.write_u32::<LittleEndian>(self.cols as u32)?;

    // write each column: (name hash, value kind, data column)
    for (col_id, (vk, col_data)) in self.data.iter() {
      // column name hash
      payload.write_u64::<LittleEndian>(*col_id)?;
      // value kind
      let value_kind = col_data.index1d(0).kind();
      value_kind.write_le(&mut payload);

      // column data as matrix
      col_data.write_le(&mut payload);
    }

    // Write the name strings into the payload
    for (_col_id, col_name) in self.col_names.iter() {
      col_name.write_le(&mut payload);
    }
    ctx.compile_const(&payload, self.kind())
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
impl ConstElem for R64 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_i64::<LittleEndian>(*self.numer()).expect("write rational numer");
    out.write_i64::<LittleEndian>(*self.denom()).expect("write rational denom");
  }
  fn value_kind() -> ValueKind { ValueKind::R64 }
  fn align() -> u8 { 16 }
}

#[cfg(feature = "complex")]
impl ConstElem for C64 {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_f64::<LittleEndian>(self.0.re).expect("write complex real");
    out.write_f64::<LittleEndian>(self.0.im).expect("write complex imag");
  }
  fn value_kind() -> ValueKind { ValueKind::C64 }
  fn align() -> u8 { 16 }
}

#[cfg(feature = "string")]
impl ConstElem for String {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u32::<LittleEndian>(self.len() as u32).expect("write string length");
    out.extend_from_slice(self.as_bytes());
  }
  fn value_kind() -> ValueKind { ValueKind::String }
  fn align() -> u8 { 1 }
}

#[cfg(feature = "bool")]
impl ConstElem for bool {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u8(if *self { 1 } else { 0 }).expect("write bool");
  }
  fn value_kind() -> ValueKind { ValueKind::Bool }
  fn align() -> u8 { 1 }
}

impl ConstElem for usize {
  fn write_le(&self, out: &mut Vec<u8>) {
    out.write_u64::<LittleEndian>(*self as u64).expect("write usize");
  }
  fn value_kind() -> ValueKind { ValueKind::Index }
  fn align() -> u8 { 8 }
}

macro_rules! impl_const_elem_matrix {
  ($matrix_type:ty, $rows:expr, $cols:expr) => {
    impl<T> ConstElem for $matrix_type
    where
      T: ConstElem
    {
      fn write_le(&self, out: &mut Vec<u8>) {
        for c in 0..self.ncols() {
          for r in 0..self.nrows() {
            self[(r, c)].write_le(out);
          }
        }
      }
      fn value_kind() -> ValueKind { ValueKind::Matrix(Box::new(T::value_kind()), vec![$rows, $cols]) }
      fn align() -> u8 { 8 }
    }
  };
}

#[cfg(feature = "matrix1")]
impl_const_elem_matrix!(Matrix1<T>, 1, 1);
#[cfg(feature = "matrix2")]
impl_const_elem_matrix!(Matrix2<T>, 2, 2);
#[cfg(feature = "matrix3")]
impl_const_elem_matrix!(Matrix3<T>, 3, 3);
#[cfg(feature = "matrix4")]
impl_const_elem_matrix!(Matrix4<T>, 4, 4);
#[cfg(feature = "matrix2x3")]
impl_const_elem_matrix!(Matrix2x3<T>, 2, 3);
#[cfg(feature = "matrix3x2")]
impl_const_elem_matrix!(Matrix3x2<T>, 3, 2);
#[cfg(feature = "row_vector2")]
impl_const_elem_matrix!(RowVector2<T>, 1, 2);
#[cfg(feature = "row_vector3")]
impl_const_elem_matrix!(RowVector3<T>, 1, 3);
#[cfg(feature = "row_vector4")]
impl_const_elem_matrix!(RowVector4<T>, 1, 4);
#[cfg(feature = "vector2")]
impl_const_elem_matrix!(Vector2<T>, 2, 1);
#[cfg(feature = "vector3")]
impl_const_elem_matrix!(Vector3<T>, 3, 1);
#[cfg(feature = "vector4")]
impl_const_elem_matrix!(Vector4<T>, 4, 1);
#[cfg(feature = "matrixd")]
impl_const_elem_matrix!(DMatrix<T>, 0, 0);
#[cfg(feature = "vectord")]
impl_const_elem_matrix!(DVector<T>, 0, 1);
#[cfg(feature = "row_vectord")]
impl_const_elem_matrix!(RowDVector<T>, 1, 0);

impl<T> ConstElem for Matrix<T> 
where
  T: ConstElem
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
  fn value_kind() -> ValueKind { ValueKind::Matrix(Box::new(T::value_kind()), vec![0,0]) }
  fn align() -> u8 { T::align() }
}


impl ConstElem for Value {
  fn write_le(&self, out: &mut Vec<u8>) {
    match self {
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
      _ => todo!(),
    }
  }
  fn value_kind() -> ValueKind {ValueKind::Any}
  fn align() -> u8 { 1 }
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
  fn value_kind() -> ValueKind { ValueKind::Any }
  fn align() -> u8 { 1 }
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
  fn value_kind() -> ValueKind { ValueKind::Enum(0) } // id 0 as placeholder
  fn align() -> u8 { 8 }
}