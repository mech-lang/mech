#[macro_use]
use crate::stdlib::*;

// Record Access --------------------------------------------------------------

#[derive(Debug)]
pub struct RecordAccessField {
  pub source: Value,
}
impl MechFunctionImpl for RecordAccessField {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.source.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for RecordAccessField {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0];

    registers[0] = compile_register!(self.source, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Access));

    ctx.emit_nullop(
      hash_str("RecordAccessField"),
      registers[0],
    );

    return Ok(registers[0]);
  }
}

pub fn impl_access_record_fxn(source: Value, key: Value) -> MResult<Box<dyn MechFunction>> {
  match (source,key) {
    (Value::Record(rcd), Value::Id(id)) => {
      let k = id;
      match rcd.borrow().get(&k) {
        Some(value) => Ok(Box::new(RecordAccessField{source: value.clone()})),
        None => Err(MechError2::new(
            UndefinedRecordFieldError { id: k.clone() },
            None
          ).with_compiler_loc()),
      }
    }
    x => return Err(MechError2::new(
        UnhandledFunctionArgumentKind2 { arg: x, fxn_name: "RecordAccess".to_string() },
        None
      ).with_compiler_loc()
    ),
  }
}

pub struct RecordAccess {}
impl NativeFunctionCompiler for RecordAccess {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let key = &arguments[1];
    let src = &arguments[0];
    match impl_access_record_fxn(src.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match src {
          Value::MutableReference(rcrd) => { impl_access_record_fxn(rcrd.borrow().clone(), key.clone()) },
          x => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (src.clone(), key.clone()), fxn_name: "RecordAccess".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      }
    }
  }
}


#[derive(Debug)]
pub struct RecordAccessSwizzle {
  pub source: Value,
}

impl MechFunctionImpl for RecordAccessSwizzle {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.source.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for RecordAccessSwizzle {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0];

    registers[0] = compile_register!(self.source, ctx);

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Swizzle));

    ctx.emit_nullop(
      hash_str("RecordAccessSwizzle"),
      registers[0],
    );

    return Ok(registers[0]);
  }
}
