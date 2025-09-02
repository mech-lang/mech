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
    // allocate one register as an array
    let mut registers = [0];

    // Compile out
    let out_addr = self.out.addr();
    let out_reg = ctx.alloc_register_for_ptr(out_addr);
    let out_const_id = self.out.compile_const(ctx).unwrap();
    ctx.emit_const_load(out_reg, out_const_id);
    registers[0] = out_reg;

    ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Tuple));

    // Emit the operation
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
      return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IncorrectNumberOfArguments});
    }
    let ix = &arguments[1];
    let src = &arguments[0];
    match (src,ix) {
      (Value::Tuple(tpl), Value::Index(ix)) => {
        let tpl_brrw = tpl.borrow();
        let ix_brrw = ix.borrow();
        if *ix_brrw > tpl_brrw.elements.len() || *ix_brrw < 1 {
            return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IndexOutOfBounds});
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
              return Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::IndexOutOfBounds});
            }
            let element = tpl_brrw.elements[*ix_brrw - 1].clone();
            let new_fxn = TupleAccessElement{ out: *element };
            Ok(Box::new(new_fxn))
          },
          _ => Err(MechError{file: file!().to_string(), tokens: vec![], msg: "".to_string(), id: line!(), kind: MechErrorKind::UnhandledFunctionArgumentKind }),
        }
      },
      _ => todo!(),
    }
  }
}