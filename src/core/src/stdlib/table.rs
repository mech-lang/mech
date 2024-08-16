#[macro_use]
use crate::stdlib::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};

// Access ---------------------------------------------------------------------

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
              (*out_ptr)[i-1] = self.source.index1d(i).[<as_ $out_type>]().unwrap().borrow().clone();
            }
          }
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  }
}

impl_col_access_fxn!(TableAccessColBoolV2, Vector2, bool);
impl_col_access_fxn!(TableAccessColI64V2, Vector2, i64);
impl_col_access_fxn!(TableAccessColU8V2, Vector2, u8);
impl_col_access_fxn!(TableAccessColBoolV3, Vector3, bool);
impl_col_access_fxn!(TableAccessColI64V3, Vector3, i64);
impl_col_access_fxn!(TableAccessColU8V3, Vector3, u8);

macro_rules! generate_access_column_match_arms {
  ($arg:expr, $($lhs_type:ident, $($default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        (Value::Table(tbl),Value::Id(k)) => {
          let key = Value::Id(k);
          match (tbl.data.get(&key),tbl.rows) {
            $(
              $(
                (Some((ValueKind::$lhs_type,value)),2) => {
                  Ok(Box::new([<TableAccessCol $lhs_type V2>]{source: value.clone(), out: new_ref(Vector2::from_element($default)) }))
                }
                (Some((ValueKind::$lhs_type,value)),3) => {
                  Ok(Box::new([<TableAccessCol $lhs_type V3>]{source: value.clone(), out: new_ref(Vector3::from_element($default)) }))
                }
              )+
            )+
            z => { 
              println!("{:?}", z);
              return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)});
            }
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
    I64,i64::zero();
    U8,u8::zero();
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