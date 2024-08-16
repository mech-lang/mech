#[macro_use]
use crate::stdlib::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};

// Access ---------------------------------------------------------------------

/*macro_rules! impl_access_fxn {
  ($struct_name:ident, $out_type:ty) => {
    #[derive(Debug)]
    struct $struct_name<T> {
      source: Ref<Vec<Value>>,
      out: Ref<$out_type>,
    }
    impl<T> MechFunction for $struct_name<T>
    where
      T: Copy + Debug + Clone + Sync + Send + PartialEq + 'static,
      Ref<$out_type>: ToValue
    {
      fn solve(&self) {
        let source_ptr = self.source.as_ptr();
        let out_ptr = self.out.as_ptr();
        unsafe { 
          for i in 0..(*source_ptr).len() {
            (*out_ptr)[i] = 0; //(*source_ptr)[i].into();
          }
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };
}*/


macro_rules! impl_col_access_fxn {
  ($fxn_name:ident, $out_type:ty) => {
    #[derive(Debug)]
    struct $fxn_name {
      source: Matrix<Value>,
      out: Ref<Vector2<$out_type>>,
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

impl_col_access_fxn!(TableAccessColI64,i64);
impl_col_access_fxn!(TableAccessColU8,u8);


//impl_access_fxn!(TableAccessCol, Vector2<usize>);
    //($arg:expr, $($input_type:ident => $($target_type:ident, $default:expr),+);+ $(;)?) => {

macro_rules! generate_access_column_match_arms {
  ($arg:expr, $default:expr) => {
    //paste!{
      match $arg {
        (Value::Table(tbl),Value::Id(k)) => {
          let key = Value::Id(k);
          match tbl.data.get(&key) {
            Some((ValueKind::I64,value)) => {
              Ok(Box::new(TableAccessColI64{source: value.clone(), out: new_ref(Vector2::from_element(i64::zero())) }))
            }
            Some((ValueKind::U8,value)) => {
              Ok(Box::new(TableAccessColU8{source: value.clone(), out: new_ref(Vector2::from_element(u8::zero())) }))
            }
            _ => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)});}
          }
        }
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    //}
  }
}

fn generate_access_column_fxn(source: Value, key: Value) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_column_match_arms!(
    
    (source,key),
    0
  
  )
}


/*macro_rules! generate_access_match_arms {
  ($macro_name:ident, $arg:expr) => {
    paste!{
      [<generate_access_ $macro_name _match_arms>]!(
        $arg,
        Bool => bool, false;
        I8   => i8,   i8::zero();
        I16  => i16,  i16::zero();
        I32  => i32,  i32::zero();
        I64  => i64,  i64::zero();
        I128 => i128, i128::zero();
        U8   => u8,   u8::zero();
        U16  => u16,  u16::zero();
        U32  => u32,  u32::zero();
        U64  => u64,  u64::zero();
        U128 => u128, u128::zero();
        F32  => F32,  F32::zero();
        F64  => F64,  F64::zero();
      )
    }
  }
}*/

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