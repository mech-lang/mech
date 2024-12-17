#[macro_use]
use crate::stdlib::*;
use self::assign::*;
use na::{Vector3, DVector, Vector2, Vector4, RowDVector, Matrix1, Matrix3, Matrix4, RowVector3, RowVector4, RowVector2, DMatrix, Rotation3, Matrix2x3, Matrix3x2, Matrix6, Matrix2};

// x.a = 1 --------------------------------------------------------------------

// Table Set ------------------------------------------------------------------

macro_rules! impl_col_set_fxn {
  ($fxn_name:ident, $vector_size:ident, $out_type:ty) => {
    #[derive(Debug)]
    struct $fxn_name {
      source: Ref<$vector_size<$out_type>>,
      sink: Ref<$vector_size<Value>>,
    }
    impl MechFunction for $fxn_name {
      fn solve(&self) {
        let source_ptr = self.source.as_ptr();
        let sink_ptr = self.sink.as_ptr();
        unsafe { 
          for i in 0..(*source_ptr).len() {
            paste! {
              (*sink_ptr)[i] = Value::[<$out_type:camel>](new_ref((*source_ptr).index(i).clone()));
            }
          }
        }
      }
      fn out(&self) -> Value { Value::MatrixValue(Matrix::$vector_size(self.sink.clone())) }
      fn to_string(&self) -> String { format!("{:?}", self) }
    }
  }
}

macro_rules! impl_col_set_fxn_shapes {
  ($type:ident) => {
    paste!{
      impl_col_set_fxn!([<TableSetCol $type:camel M1>], Matrix1, $type);
      impl_col_set_fxn!([<TableSetCol $type:camel V2>], Vector2, $type);
      impl_col_set_fxn!([<TableSetCol $type:camel V3>], Vector3, $type);
      impl_col_set_fxn!([<TableSetCol $type:camel V4>], Vector4, $type);
      impl_col_set_fxn!([<TableSetCol $type:camel VD>], DVector, $type);
    }
  }
}

impl_col_set_fxn_shapes!(bool);
impl_col_set_fxn_shapes!(i8);
impl_col_set_fxn_shapes!(i16);
impl_col_set_fxn_shapes!(i32);
impl_col_set_fxn_shapes!(i64);
impl_col_set_fxn_shapes!(i128);
impl_col_set_fxn_shapes!(u8);
impl_col_set_fxn_shapes!(u16);
impl_col_set_fxn_shapes!(u32);
impl_col_set_fxn_shapes!(u64);
impl_col_set_fxn_shapes!(u128);
impl_col_set_fxn_shapes!(F32);
impl_col_set_fxn_shapes!(F64);

macro_rules! impl_set_column_match_arms {
  ($arg:expr, $($lhs_type:ident, $($default:expr),+);+ $(;)?) => {
    paste!{
      match $arg {
        (Value::Record(rcrd),source,Value::Id(k)) => {
          let key = Value::Id(k);
          match (rcrd.map.get(&key),source) {
            (Some(Value::Bool(sink)), Value::Bool(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::I8(sink)), Value::I8(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::I16(sink)), Value::I16(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::I32(sink)), Value::I32(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::I64(sink)), Value::I64(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::I128(sink)), Value::I128(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::U8(sink)), Value::U8(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::U16(sink)), Value::U16(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::U32(sink)), Value::U32(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::U64(sink)), Value::U64(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::U128(sink)), Value::U128(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::F32(sink)), Value::F32(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            (Some(Value::F64(sink)), Value::F64(source)) => Ok(Box::new(RecordSet{sink: sink.clone(), source: source.clone()})),
            _ => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)}),
          }
        }
        (Value::Table(tbl),source,Value::Id(k)) => {
          let key = Value::Id(k);
          match (tbl.data.get(&key),tbl.rows,source) {
            $(
                (Some((ValueKind::$lhs_type,Matrix::Matrix1(sink))),1,Value::[<Matrix $lhs_type>](Matrix::Matrix1(source))) => Ok(Box::new([<TableSetCol $lhs_type M1>]{source: source.clone(), sink: sink.clone() })),
                (Some((ValueKind::$lhs_type,Matrix::Vector2(sink))),2,Value::[<Matrix $lhs_type>](Matrix::Vector2(source))) => Ok(Box::new([<TableSetCol $lhs_type V2>]{source: source.clone(), sink: sink.clone() })),
                (Some((ValueKind::$lhs_type,Matrix::Vector3(sink))),3,Value::[<Matrix $lhs_type>](Matrix::Vector3(source))) => Ok(Box::new([<TableSetCol $lhs_type V3>]{source: source.clone(), sink: sink.clone() })),
                (Some((ValueKind::$lhs_type,Matrix::Vector4(sink))),4,Value::[<Matrix $lhs_type>](Matrix::Vector4(source))) => Ok(Box::new([<TableSetCol $lhs_type V4>]{source: source.clone(), sink: sink.clone() })),
                (Some((ValueKind::$lhs_type,Matrix::DVector(sink))),n,Value::[<Matrix $lhs_type>](Matrix::DVector(source))) => Ok(Box::new([<TableSetCol $lhs_type VD>]{source: source.clone(), sink: sink.clone() })),
            )+
            x => return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UndefinedField(k)}),
          }
        }
        x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
      }
    }
  }
}

fn impl_set_column_fxn(sink: Value, source: Value, key: Value) -> Result<Box<dyn MechFunction>, MechError> {
  impl_set_column_match_arms!(
    (sink,source,key),
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

pub struct SetColumn {}
impl NativeFunctionCompiler for SetColumn {
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