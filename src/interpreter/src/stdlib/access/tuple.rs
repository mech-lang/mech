#[macro_use]
use crate::stdlib::*;

// Tuple Access --------------------------------------------------------------

#[derive(Debug)]
struct TupleAccessElement {
  out: Value,
}

impl MechFunctionImpl for TupleAccessElement {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value { self.out.clone() }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TupleAccessElement {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let mut registers = [0];
    registers[0] = compile_register!(self.out, ctx);
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Tuple));
    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Access));
    ctx.emit_nullop(
      hash_str(stringify!("TupleAccessElement")),
      registers[0],
    );
    return Ok(registers[0]);
  }
}
  
pub struct TupleAccess {}
impl NativeFunctionCompiler for TupleAccess{
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 2 {
      return Err(MechError2::new(IncorrectNumberOfArguments { expected: 1, found: arguments.len() }, None).with_compiler_loc());
    }
    let ix1 = &arguments[1];
    let src = &arguments[0];
    match (src.clone(),ix1.clone()) {
      (Value::Tuple(tpl), Value::Index(ix)) => {
        let tpl_brrw = tpl.borrow();
        let ix_brrw = ix.borrow();
        if *ix_brrw > tpl_brrw.elements.len() || *ix_brrw < 1 {
            return Err(MechError2::new(
                TupleIndexOutOfBoundsError { ix: *ix_brrw, len: tpl_brrw.elements.len() },
                None
              ).with_compiler_loc());
        }
        let element = tpl_brrw.elements[*ix_brrw - 1].clone();
        let new_fxn = TupleAccessElement{ out: *element };
        Ok(Box::new(new_fxn))
      },
      (Value::MutableReference(tpl), Value::Index(ix)) => {
        match &*tpl.borrow() {
          Value::Tuple(ref tpl) => {
            let ix_brrw = ix.borrow();
            let tpl_brrw = tpl.borrow();
            if *ix_brrw > tpl_brrw.elements.len() || *ix_brrw < 1 {
              return Err(MechError2::new(
                  TupleIndexOutOfBoundsError { ix: *ix_brrw, len: tpl_brrw.elements.len() },
                  None
                ).with_compiler_loc());
            }
            let element = tpl_brrw.elements[*ix_brrw - 1].clone();
            let new_fxn = TupleAccessElement{ out: *element };
            Ok(Box::new(new_fxn))
          },
          _ => Err(MechError2::new(
              UnhandledFunctionArgumentKind2 { arg: (src.clone(), ix1.clone()), fxn_name: "access/tuple-element".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      },
      _ => todo!(),
    }
  }
}