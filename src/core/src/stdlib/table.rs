#[macro_use]
use crate::stdlib::*;

// Access ---------------------------------------------------------------------

macro_rules! impl_access_fxn {
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
          for i in 0..(*source_ptr).rows {
            (*out_ptr)[i] = (*source_ptr)[i].into();
          }
        }
      }
      fn out(&self) -> Value { self.out.to_value() }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  };
}

impl_access_fxn!(TableAccessCol, Vector2<T>);
  
macro_rules! generate_access_match_arms {
  ($fxn_name:ident,$macro_name:ident, $arg:expr) => {
    paste!{
      [<generate_access_ $macro_name _match_arms>]!(
        $fxn_name,
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
}
  
macro_rules! generate_access_column_match_arms {
  ($fxn_name:ident, $arg:expr, $($input_type:ident => $($table_kind:ident, $target_type:ident, $default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        $(
          $(
            (Value::Table(tbl), Value::Id(k)) => {
              let key = Value::Id(k);
              match (tbl.data.get(&key),tbl.kind.get(&key)) {
                Some(value) => {
                  Ok(Box::new(TableAccessColumn{source: input.clone(), column_ix: ix.clone(), out: new_ref(DVector::from_element(tbl.rows,$default)) })),
                }
                None => { return Err(MechError{tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UndefinedField(key)});}
              }
            },
          )+
        )+
        x => Err(MechError { tokens: vec![], msg: format!("{:?}",x), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}
  
fn generate_access_column_fxn(lhs_value: Value, ixes: Vec<Value>) -> Result<Box<dyn MechFunction>, MechError> {
  generate_access_column_match_arms!(Access1DS, scalar, (lhs_value, ixes.as_slice()))
}

pub struct TableAccessColumn {}
impl NativeFunctionCompiler for TableAccessColumn {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() <= 1 {
      return Err(MechError {tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ixes = arguments.clone().split_off(1);
    let mat = arguments[0].clone();
    match generate_access_column_fxn(mat.clone(), ixes.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (mat,ixes) {
          (Value::MutableReference(lhs),rhs_value) => { generate_access_column_fxn(lhs.borrow().clone(), rhs_value.clone()) }
          x => Err(MechError { tokens: vec![], msg: file!().to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}