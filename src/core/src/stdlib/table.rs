#[macro_use]
use crate::stdlib::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};

// Access ---------------------------------------------------------------------




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

macro_rules! generate_access_column_match_arms {
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

fn generate_access_column_fxn(source: Value, key: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_column_match_arms!(
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
    match generate_access_column_fxn(tbl.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (tbl,&key) {
          (Value::MutableReference(tbl),_) => { generate_access_column_fxn(tbl.borrow().clone(), key.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}