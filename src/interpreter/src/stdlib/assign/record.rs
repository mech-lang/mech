#[macro_use]
use crate::stdlib::*;
use self::assign::*;

// x.a = 1 --------------------------------------------------------------------

// Record Set -----------------------------------------------------------------

#[derive(Debug)]
pub struct RecordAssign<T> {
  pub sink: Ref<T>,
  pub source: Ref<T>,
}
impl<T> MechFunction for RecordAssign<T> 
  where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<T>: ToValue
{
  fn solve(&self) {
    let source_ptr = self.source.as_ptr();
    let sink_ptr = self.sink.as_mut_ptr();
    unsafe {
      *sink_ptr = (*source_ptr).clone();
    }
  }
  fn out(&self) -> Value { self.sink.to_value() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    todo!();
  }
}

fn impl_set_record_column_fxn(sink: Value, source: Value, key: Value) -> MResult<Box<dyn MechFunction>> {
  match (&sink, &source, &key) {
    (Value::Record(rcrd),source,Value::Id(k)) => {
      let rcrd_brrw = rcrd.borrow();
      match (rcrd_brrw.data.get(k),source) {
        #[cfg(all(feature = "bool", feature = "record"))]
        (Some(Value::Bool(sink)), Value::Bool(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "i8", feature = "record"))]
        (Some(Value::I8(sink)), Value::I8(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "i16", feature = "record"))]
        (Some(Value::I16(sink)), Value::I16(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "i32", feature = "record"))]
        (Some(Value::I32(sink)), Value::I32(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "i64", feature = "record"))]
        (Some(Value::I64(sink)), Value::I64(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "i128", feature = "record"))]
        (Some(Value::I128(sink)), Value::I128(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "u8", feature = "record"))]
        (Some(Value::U8(sink)), Value::U8(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "u16", feature = "record"))]
        (Some(Value::U16(sink)), Value::U16(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "u32", feature = "record"))]
        (Some(Value::U32(sink)), Value::U32(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "u64", feature = "record"))]
        (Some(Value::U64(sink)), Value::U64(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "u128", feature = "record"))]
        (Some(Value::U128(sink)), Value::U128(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "f32", feature = "record"))]
        (Some(Value::F32(sink)), Value::F32(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "f64", feature = "record"))]
        (Some(Value::F64(sink)), Value::F64(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "string", feature = "record"))]
        (Some(Value::String(sink)), Value::String(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "complex", feature = "record"))]
        (Some(Value::ComplexNumber(sink)), Value::ComplexNumber(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        #[cfg(all(feature = "rational", feature = "record"))]
        (Some(Value::RationalNumber(sink)), Value::RationalNumber(source)) => return Ok(Box::new(RecordAssign{sink: sink.clone(), source: source.clone()})),
        _ => return Err(MechError {file: file!().to_string(),tokens: vec![],msg: "".to_string(),id: line!(),kind: MechErrorKind::UndefinedField(*k)}),
      }
    }
    _ => return Err(MechError{file: file!().to_string(),tokens: vec![],msg: "".to_string(),id: line!(),kind: MechErrorKind::None}),
  }
}

pub struct AssignRecordColumn {}
impl NativeFunctionCompiler for AssignRecordColumn {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 3 {
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    let key = arguments[2].clone();
    match impl_set_record_column_fxn(sink.clone(), source.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match (&sink,&source,&key) {
          (Value::MutableReference(sink),_,_) => { impl_set_record_column_fxn(sink.borrow().clone(), source.clone(), key.clone()) }
          x => Err(MechError{file: file!().to_string(),  tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      }
    }
  }
}