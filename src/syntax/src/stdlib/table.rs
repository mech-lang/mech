#[macro_use]
use crate::stdlib::*;

// Access ---------------------------------------------------------------------

macro_rules! access_1d {
    ($source:expr, $ix:expr, $out:expr) => {
      unsafe { *$out = (*$source).index(*$ix-1).clone() }
    };}
  
  macro_rules! impl_access_fxn_shape {
    ($name:ident, $ix_type:ty, $out_type:ty, $fxn:ident) => {
      paste!{
        impl_access_fxn!([<$name V2>],   Vector2<T>,    $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name V3>],   Vector3<T>,    $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name V4>],   Vector4<T>,    $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name R2>],   RowVector2<T>, $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name R3>],   RowVector3<T>, $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name R4>],   RowVector4<T>, $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name M1>],   Table1<T>,    $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name M2>],   Table2<T>,    $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name M3>],   Table3<T>,    $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name M4>],   Table4<T>,    $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name M2x3>], Table2x3<T>,  $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name M3x2>], Table3x2<T>,  $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name MD>],   DTable<T>,    $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name RD>],   RowDVector<T>, $ix_type, $out_type, $fxn);
        impl_access_fxn!([<$name VD>],   DVector<T>,    $ix_type, $out_type, $fxn);
      }
    };}
  
  // x[1]
  impl_access_fxn_shape!(Access1DS, usize, T, access_1d);
  
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
              (Value::Table(tbl), Value::Id(key)) => {
                match tbl.data.get(&Value::Id(key)) {
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