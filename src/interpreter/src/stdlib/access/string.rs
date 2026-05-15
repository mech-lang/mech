#[macro_use]
use crate::stdlib::*;
use grapheme::Graphemes;

// String Access -------------------------------------------------------------

fn access_grapheme(s: &str, ix: usize) -> MResult<String> {
  if ix < 1 {
    return Err(MechError::new(IndexOutOfBoundsError, None).with_compiler_loc());
  }
  Graphemes::from_usvs(s)
    .iter()
    .nth(ix - 1)
    .map(|g| g.as_str().to_string())
    .ok_or_else(|| MechError::new(IndexOutOfBoundsError, None).with_compiler_loc())
}

#[derive(Debug)]
struct StringAccessElement {
  out: Value,
}

impl MechFunctionImpl for StringAccessElement {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.out.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for StringAccessElement {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0];
    registers[0] = compile_register!(self.out, ctx);
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::String));
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Access));
    ctx.emit_nullop(
      hash_str(stringify!("StringAccessElement")),
      registers[0],
    );
    return Ok(registers[0]);
  }
}

pub struct StringAccessScalar {}
impl NativeFunctionCompiler for StringAccessScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 2 {
      return Err(MechError::new(IncorrectNumberOfArguments { expected: 2, found: arguments.len() }, None).with_compiler_loc());
    }
    let src = &arguments[0];
    let ix1 = &arguments[1];
    match (src.clone(), ix1.clone()) {
      (Value::String(s), Value::Index(ix)) => {
        let s_brrw = s.borrow();
        let ix_brrw = ix.borrow();
        let grapheme = access_grapheme(&s_brrw, *ix_brrw)?;
        let new_fxn = StringAccessElement { out: Value::String(Ref::new(grapheme)) };
        Ok(Box::new(new_fxn))
      },
      (Value::MutableReference(r), Value::Index(ix)) => {
        match &*r.borrow() {
          Value::String(s) => {
            let s_brrw = s.borrow();
            let ix_brrw = ix.borrow();
            let grapheme = access_grapheme(&s_brrw, *ix_brrw)?;
            let new_fxn = StringAccessElement { out: Value::String(Ref::new(grapheme)) };
            Ok(Box::new(new_fxn))
          },
          _ => Err(MechError::new(
              UnhandledFunctionArgumentKind2 { arg: (src.kind(), ix1.kind()), fxn_name: "access/scalar-string".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      },
      _ => Err(MechError::new(
          UnhandledFunctionArgumentKind2 { arg: (src.kind(), ix1.kind()), fxn_name: "access/scalar-string".to_string() },
          None
        ).with_compiler_loc()
      ),
    }
  }
}
