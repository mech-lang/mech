#[macro_use]
use crate::stdlib::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};

// ----------------------------------------------------------------------------
// Set 
// ----------------------------------------------------------------------------

// Scalar Set -----------------------------------------------------------------

#[derive(Debug)]
struct SetF64{
  sink: Ref<F64>,
  source: Ref<F64>,
}
impl MechFunction for SetF64 {
  fn solve(&self) {
    let sink_ptr = self.sink.as_ptr();
    let source_ptr = self.source.as_ptr();
    unsafe {
      *sink_ptr = (*source_ptr).clone();
    }
  }
  fn out(&self) -> Value { Value::F64(self.sink.clone()) }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct SetValue {}
impl NativeFunctionCompiler for SetValue {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    match (sink,source) {
      (Value::F64(sink),Value::F64(source)) => {
        Ok(Box::new(SetF64{sink: sink.clone(), source: source.clone()}))
      }
      x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
    }
  }
}

// ----------------------------------------------------------------------------
// Access 
// ----------------------------------------------------------------------------

// Record Access --------------------------------------------------------------

#[derive(Debug)]
struct RecordAccess {
  source: Value,
}
impl MechFunction for RecordAccess {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.source.clone() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

// Table Access ---------------------------------------------------------------

macro_rules! impl_col_access_fxn {
  ($fxn_name:ident, $vector_size:ident, $out_type:ty) => {
    #[derive(Debug)]
    struct $fxn_name {
      source: Matrix<Value>,
      out: Ref<$vector_size<$out_type>>,
    }
    impl MechFunction for $fxn_name {
      fn solve(&self) {
        let out_ptr = self.out.as_ptr();
        unsafe { 
          for i in 1..=self.source.shape()[0] {
            paste! {
              (*out_ptr)[i-1] = self.source.index1d(i).[<as_ $out_type:lower>]().unwrap().borrow().clone();
            }
          }
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  }
}

macro_rules! impl_col_access_fxn_shapes {
  ($type:ident) => {
    paste!{
      impl_col_access_fxn!([<TableAccessCol $type:camel M1>], Matrix1, [<$type>]);
      impl_col_access_fxn!([<TableAccessCol $type:camel V2>], Vector2, [<$type>]);
      impl_col_access_fxn!([<TableAccessCol $type:camel V3>], Vector3, [<$type>]);
      impl_col_access_fxn!([<TableAccessCol $type:camel V4>], Vector4, [<$type>]);
      impl_col_access_fxn!([<TableAccessCol $type:camel VD>], DVector, [<$type>]);
    }
  }
}

impl_col_access_fxn_shapes!(bool);
impl_col_access_fxn_shapes!(i8);
impl_col_access_fxn_shapes!(i16);
impl_col_access_fxn_shapes!(i32);
impl_col_access_fxn_shapes!(i64);
impl_col_access_fxn_shapes!(i128);
impl_col_access_fxn_shapes!(u8);
impl_col_access_fxn_shapes!(u16);
impl_col_access_fxn_shapes!(u32);
impl_col_access_fxn_shapes!(u64);
impl_col_access_fxn_shapes!(u128);
impl_col_access_fxn_shapes!(F32);
impl_col_access_fxn_shapes!(F64);

macro_rules! impl_access_column_match_arms {
  ($arg:expr, $($lhs_type:ident, $($default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        (Value::Record(rcrd),Value::Id(k)) => {
          let key = Value::Id(k);
          match rcrd.map.get(&key) {
            Some(value) => Ok(Box::new(RecordAccess{source: value.clone()})),
            _ => return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)}),
          }
        }
        (Value::Table(tbl),Value::Id(k)) => {
          let key = Value::Id(k);
          match (tbl.data.get(&key),tbl.rows) {
            $(
              $(
                (Some((ValueKind::$lhs_type,value)),1) => Ok(Box::new([<TableAccessCol $lhs_type M1>]{source: value.clone(), out: new_ref(Matrix1::from_element($default)) })),
                (Some((ValueKind::$lhs_type,value)),2) => Ok(Box::new([<TableAccessCol $lhs_type V2>]{source: value.clone(), out: new_ref(Vector2::from_element($default)) })),
                (Some((ValueKind::$lhs_type,value)),3) => Ok(Box::new([<TableAccessCol $lhs_type V3>]{source: value.clone(), out: new_ref(Vector3::from_element($default)) })),
                (Some((ValueKind::$lhs_type,value)),4) => Ok(Box::new([<TableAccessCol $lhs_type V4>]{source: value.clone(), out: new_ref(Vector4::from_element($default)) })),
                (Some((ValueKind::$lhs_type,value)),n) => Ok(Box::new([<TableAccessCol $lhs_type VD>]{source: value.clone(), out: new_ref(DVector::from_element(n,$default)) })),
              )+
            )+
            _ => return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)}),
          }
        }
        x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_access_column_fxn(source: Value, key: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_access_column_match_arms!(
    (source,key),
    Bool,false;
    I8,i8::zero();
    I16,i16::zero();
    I32,i32::zero();
    I64,i64::zero();
    I128,i128::zero();
    U8,u8::zero();
    U16,u16::zero();
    U32,u32::zero();
    U64,u64::zero();
    U128,u128::zero();
    F32,F32::zero();
    F64,F64::zero();
  )
}

pub struct AccessColumn {}
impl NativeFunctionCompiler for AccessColumn {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let tbl = arguments[0].clone();
    let key = arguments[1].clone();
    match impl_access_column_fxn(tbl.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (tbl,&key) {
          (Value::MutableReference(tbl),_) => { impl_access_column_fxn(tbl.borrow().clone(), key.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}

// Table Access Swizzle -------------------------------------------------------

#[derive(Debug)]
struct RecordAccessSwizzle {
  source: Value,
}

impl MechFunction for RecordAccessSwizzle {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.source.clone() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

#[derive(Debug)]
struct TableAccessSwizzle {
  out: Value,
}

impl MechFunction for TableAccessSwizzle {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.out.clone() }
  fn to_string(&self) -> String { format!("{:?}", self) }
}

pub struct AccessSwizzle {}
impl NativeFunctionCompiler for AccessSwizzle {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 3 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let keys = &arguments.clone().split_off(1);
    let src = &arguments[0];
    match src {
      Value::Record(rcrd) => {
        let mut values = vec![];
        for k in keys {
          match rcrd.map.get(k) {
            Some(value) => values.push(value.clone()),
            None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(*k.as_u64().unwrap().borrow())});}
          }
        }
        Ok(Box::new(RecordAccessSwizzle{source: Value::Tuple(MechTuple::from_vec(values))}))
      }
      Value::Table(tbl) => {
        let mut elements = vec![];
        for k in keys {
          match tbl.data.get(k) {
            Some((kind, mat_values)) => {
              elements.push(Box::new(mat_values.to_value()));
            }
            None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(*k.as_u64().unwrap().borrow())});}
          }
        }
        let tuple = Value::Tuple(MechTuple{elements});
        Ok(Box::new(TableAccessSwizzle{out: tuple}))
      }
      Value::MutableReference(r) => match &*r.borrow() {
        Value::Record(rcrd) => {
          let mut values = vec![];
          for k in keys {
            match rcrd.map.get(k) {
              Some(value) => values.push(value.clone()),
              None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(*k.as_u64().unwrap().borrow())});}
            }
          }
          Ok(Box::new(RecordAccessSwizzle{source: Value::Tuple(MechTuple::from_vec(values))}))
        }
        Value::Table(tbl) => {
          let mut elements = vec![];
          for k in keys {
            match tbl.data.get(k) {
              Some((kind, mat_values)) => {
                elements.push(Box::new(mat_values.to_value()));
              }
              None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(*k.as_u64().unwrap().borrow())});}
            }
          }
          let tuple = Value::Tuple(MechTuple{elements});
          Ok(Box::new(TableAccessSwizzle{out: tuple}))
        }
        _ => todo!(),
      }
      _ => todo!(),
    }
  }
}