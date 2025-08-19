#[macro_use]
use crate::stdlib::*;
use self::assign::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};

// x.a = 1 --------------------------------------------------------------------

// Table Set ------------------------------------------------------------------

macro_rules! impl_col_set_fxn {
  ($fxn_name:ident, $vector_size_in:ident, $vector_size_out:ident, $out_type:ty) => {
    #[derive(Debug)]
    struct $fxn_name {
      source: Ref<$vector_size_in<$out_type>>,
      sink: Ref<$vector_size_out<Value>>,
    }
    impl MechFunction for $fxn_name {
      fn solve(&self) {
        let source_ptr = self.source.as_ptr();
        let sink_ptr = self.sink.as_mut_ptr();
        unsafe { 
          for i in 0..(*source_ptr).len() {
            paste! {
              (&mut (*sink_ptr))[i] = Value::[<$out_type:camel>](Ref::new((*source_ptr).index(i).clone()));
            }
          }
        }
      }
      fn out(&self) -> Value { Value::MatrixValue(Matrix::$vector_size_out(self.sink.clone())) }
      fn to_string(&self) -> String { format!("{:#?}", self) }
      fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        todo!();
      }
    }
  }
}

macro_rules! impl_col_set_fxn_shapes {
  ($type:ident) => {
    paste!{
      #[cfg(feature = "matrix1")]
      impl_col_set_fxn!([<TableSetCol $type:camel M1>], Matrix1, Matrix1, $type);
      #[cfg(feature = "vector2")]
      impl_col_set_fxn!([<TableSetCol $type:camel V2>], Vector2, Vector2, $type);
      #[cfg(feature = "vector3")]
      impl_col_set_fxn!([<TableSetCol $type:camel V3>], Vector3, Vector3, $type);
      #[cfg(feature = "vector4")]
      impl_col_set_fxn!([<TableSetCol $type:camel V4>], Vector4, Vector4, $type);
      #[cfg(feature = "vectord")]
      impl_col_set_fxn!([<TableSetCol $type:camel VD>], DVector, DVector, $type);
      #[cfg(all(feature = "vectord", feature = "vector4"))]
      impl_col_set_fxn!([<TableSetCol $type:camel VDV4>], Vector4, DVector, $type);
      #[cfg(all(feature = "vectord", feature = "vector3"))]
      impl_col_set_fxn!([<TableSetCol $type:camel VDV3>], Vector3, DVector, $type);
      #[cfg(all(feature = "vectord", feature = "vector2"))]
      impl_col_set_fxn!([<TableSetCol $type:camel VDV2>], Vector2, DVector, $type);
      #[cfg(all(feature = "vectord", feature = "matrix1"))]
      impl_col_set_fxn!([<TableSetCol $type:camel VDM1>], Matrix1, DVector, $type);
    }
  }
}

#[cfg(feature = "bool")]
impl_col_set_fxn_shapes!(bool);
#[cfg(feature = "i8")]
impl_col_set_fxn_shapes!(i8);
#[cfg(feature = "i16")]
impl_col_set_fxn_shapes!(i16);
#[cfg(feature = "i32")]
impl_col_set_fxn_shapes!(i32);
#[cfg(feature = "i64")]
impl_col_set_fxn_shapes!(i64);
#[cfg(feature = "i128")]
impl_col_set_fxn_shapes!(i128);
#[cfg(feature = "u8")]
impl_col_set_fxn_shapes!(u8);
#[cfg(feature = "u16")]
impl_col_set_fxn_shapes!(u16);
#[cfg(feature = "u32")]
impl_col_set_fxn_shapes!(u32);
#[cfg(feature = "u64")]
impl_col_set_fxn_shapes!(u64);
#[cfg(feature = "u128")]
impl_col_set_fxn_shapes!(u128);
#[cfg(feature = "f32")]
impl_col_set_fxn_shapes!(F32);
#[cfg(feature = "f64")]
impl_col_set_fxn_shapes!(F64);
#[cfg(feature = "string")]
impl_col_set_fxn_shapes!(String);
#[cfg(feature = "complex")]
impl_col_set_fxn_shapes!(ComplexNumber);
#[cfg(feature = "rational")]
impl_col_set_fxn_shapes!(RationalNumber);

macro_rules! impl_set_column_match_arms {
  ($arg:expr, $($lhs_type:ident, $type_ident:ident, $type_feature:literal);+ $(;)?) => {
    paste::paste! {
      match $arg {
        (Value::Table(tbl), source, Value::Id(k)) => {
          let tbl_brrw = tbl.borrow();
          match (tbl_brrw.get(&k), tbl_brrw.rows(), source) {
            $(
              #[cfg(all(feature = $type_feature, feature = "matrix1"))]
              (Some((ValueKind::$lhs_type, Matrix::Matrix1(sink))), 1, Value::[<Matrix $lhs_type>](Matrix::Matrix1(source))) =>Ok(Box::new([<TableSetCol $lhs_type M1>]{ source: source.clone(), sink: sink.clone() })),
              #[cfg(all(feature = $type_feature, feature = "vector2"))]
              (Some((ValueKind::$lhs_type, Matrix::Vector2(sink))), 2, Value::[<Matrix $lhs_type>](Matrix::Vector2(source))) =>Ok(Box::new([<TableSetCol $lhs_type V2>]{ source: source.clone(), sink: sink.clone() })),
              #[cfg(all(feature = $type_feature, feature = "vector3"))]
              (Some((ValueKind::$lhs_type, Matrix::Vector3(sink))), 3, Value::[<Matrix $lhs_type>](Matrix::Vector3(source))) =>Ok(Box::new([<TableSetCol $lhs_type V3>]{ source: source.clone(), sink: sink.clone() })),
              #[cfg(all(feature = $type_feature, feature = "vector4"))]
              (Some((ValueKind::$lhs_type, Matrix::Vector4(sink))), 4, Value::[<Matrix $lhs_type>](Matrix::Vector4(source))) =>Ok(Box::new([<TableSetCol $lhs_type V4>]{ source: source.clone(), sink: sink.clone() })),
              #[cfg(all(feature = $type_feature, feature = "vectord"))]
              (Some((ValueKind::$lhs_type, Matrix::DVector(sink))), n, Value::[<Matrix $lhs_type>](Matrix::DVector(source))) =>Ok(Box::new([<TableSetCol $lhs_type VD>]{ source: source.clone(), sink: sink.clone() })),
              #[cfg(all(feature = $type_feature, feature = "vectord", feature = "vector4"))]
              (Some((ValueKind::$lhs_type, Matrix::DVector(sink))), n, Value::[<Matrix $lhs_type>](Matrix::Vector4(source))) =>Ok(Box::new([<TableSetCol $lhs_type VDV4>]{ source: source.clone(), sink: sink.clone() })),
              #[cfg(all(feature = $type_feature, feature = "vectord", feature = "vector3"))]
              (Some((ValueKind::$lhs_type, Matrix::DVector(sink))), n, Value::[<Matrix $lhs_type>](Matrix::Vector3(source))) =>Ok(Box::new([<TableSetCol $lhs_type VDV3>]{ source: source.clone(), sink: sink.clone() })),
              #[cfg(all(feature = $type_feature, feature = "vectord", feature = "vector2"))]
              (Some((ValueKind::$lhs_type, Matrix::DVector(sink))), n, Value::[<Matrix $lhs_type>](Matrix::Vector2(source))) =>Ok(Box::new([<TableSetCol $lhs_type VDV2>]{ source: source.clone(), sink: sink.clone() })),
              #[cfg(all(feature = $type_feature, feature = "vectord", feature = "matrix1"))]
              (Some((ValueKind::$lhs_type, Matrix::DVector(sink))), n, Value::[<Matrix $lhs_type>](Matrix::Matrix1(source))) =>Ok(Box::new([<TableSetCol $lhs_type VDM1>]{ source: source.clone(), sink: sink.clone() })),
            )+
            x => return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "".to_string(),id: line!(),kind: MechErrorKind::UndefinedField(k)}),
          }
        }
        x => Err(MechError {file: file!().to_string(),tokens: vec![],msg: "".to_string(),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind}),
      }
    }
  }
}

fn impl_set_column_fxn(sink: Value, source: Value, key: Value) -> MResult<Box<dyn MechFunction>> {
  impl_set_column_match_arms!(
    (sink,source,key),
    Bool, bool, "bool";
    I8,   i8,   "i8";
    I16,  i16,  "i16";
    I32,  i32,  "i32";
    I64,  i64,  "i64";
    I128, i128, "i128";
    U8,   u8,   "u8";
    U16,  u16,  "u16";
    U32,  u32,  "u32";
    U64,  u64,  "u64";
    U128, u128, "u128";
    F32,  F32,  "f32";
    F64,  F64,  "f64";
    String, String, "string";
    ComplexNumber, ComplexNumber,"complex";
    RationalNumber, RationalNumber,"rational";
  )
}

pub struct AssignTableColumn {}
impl NativeFunctionCompiler for AssignTableColumn {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 3 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    let key = arguments[2].clone();
    match impl_set_column_fxn(sink.clone(), source.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (&sink,&source,&key) {
          (Value::MutableReference(sink),_,_) => { impl_set_column_fxn(sink.borrow().clone(), source.clone(), key.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// table1 += table2 ------------------------------------------------------------

#[derive(Debug)]
struct TableAppendRecord {
  sink: Ref<MechTable>,
  source: Ref<MechRecord>,
}
impl MechFunction for TableAppendRecord {
  fn solve(&self) {
    unsafe {
      let mut sink_ptr = (&mut *(self.sink.as_mut_ptr()));
      let source_ptr = &(*(self.source.as_ptr()));
      sink_ptr.append_record(source_ptr.clone());
    }
  }
  fn out(&self) -> Value { Value::Table(self.sink.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

#[derive(Debug)]
struct TableAppendTable {
  sink: Ref<MechTable>,
  source: Ref<MechTable>,
}
impl MechFunction for TableAppendTable {
  fn solve(&self) {
    unsafe {
      let mut sink_ptr = (&mut *(self.sink.as_mut_ptr()));
      let source_ptr = &(*(self.source.as_ptr()));
      sink_ptr.append_table(&source_ptr);
    }
  }
  fn out(&self) -> Value { Value::Table(self.sink.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

pub fn add_assign_table_fxn(sink: Value, source: Value) -> Result<Box<dyn MechFunction>, MechError> {
  match (sink.clone(),source.clone()) {
    (Value::Table(tbl), Value::Record(rcrd)) => {
      tbl.borrow().check_record_schema(&rcrd.borrow())?;
      return Ok(Box::new(TableAppendRecord{ sink: tbl, source: rcrd }))
    }
    (Value::Table(tbl_sink), Value::Table(tbl_src)) => {
      tbl_sink.borrow().check_table_schema(&tbl_src.borrow())?;
      return Ok(Box::new(TableAppendTable{ sink: tbl_sink, source: tbl_src }))
    }
    x => return Err(MechError{file: file!().to_string(),tokens: vec![],msg: format!("Unhandled args {:?}, {:?}", sink, source),id: line!(),kind: MechErrorKind::UnhandledFunctionArgumentKind,}),
  }
}

pub struct AddAssignTable {}
impl NativeFunctionCompiler for AddAssignTable {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match add_assign_table_fxn(sink.clone(),source.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(x) => {
        match (sink,source) {
          (Value::MutableReference(sink),Value::MutableReference(source)) => { add_assign_table_fxn(sink.borrow().clone(),source.borrow().clone()) },
          (sink,Value::MutableReference(source)) => { add_assign_table_fxn(sink.clone(),source.borrow().clone()) },
          (Value::MutableReference(sink),source) => { add_assign_table_fxn(sink.borrow().clone(),source.clone()) },
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}