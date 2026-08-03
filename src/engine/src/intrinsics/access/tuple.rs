#[macro_use]
use crate::intrinsics::*;

// Tuple Access --------------------------------------------------------------

#[derive(Debug)]
struct TupleAccessElement {
    out: Value,
}

impl MechFunctionImpl for TupleAccessElement {
    fn solve(&self) {
        ()
    }
    fn out(&self) -> Value {
        self.out.clone()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(self.reactive_output_values())
    }
}
impl MechFunctionFactory for TupleAccessElement {
    fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        match args {
            FunctionArgs::Nullary(out) => Ok(Box::new(Self { out })),
            _ => Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 0,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc()),
        }
    }
}

mech_core::declare_native_runtime_factory! {
    cfg: all(feature = "access", feature = "tuple"),
    registration: register_tuple_access_element,
    installer: install_tuple_access_element,
    name: "TupleAccessElement",
    factory: TupleAccessElement::new,
    package: "mech-engine", crate_name: "mech_engine",
    installer_path: "mech_engine::__mech_native::install_tuple_access_element",
    cargo_features: ["access", "native-link", "runtime", "tuple"],
}

pub(super) fn install_runtime(builder: &mut FunctionCatalogBuilder) -> MResult<()> {
    register_tuple_access_element(builder)
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for TupleAccessElement {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0];
        registers[0] = compile_register!(self.out, ctx);
        ctx.emit_nullop(hash_str("TupleAccessElement"), registers[0]);
        return Ok(registers[0]);
    }
}

pub struct TupleAccess {}
impl FunctionSpecializer for TupleAccess {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() < 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let ix1 = &arguments[1];
        let src = &arguments[0];
        match (src.clone(), ix1.clone()) {
            (Value::Tuple(tpl), Value::Index(ix)) => {
                let tpl_brrw = tpl.borrow();
                let ix_brrw = ix.borrow();
                if *ix_brrw > tpl_brrw.elements.len() || *ix_brrw < 1 {
                    return Err(MechError::new(
                        TupleIndexOutOfBoundsError {
                            ix: *ix_brrw,
                            len: tpl_brrw.elements.len(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
                let element = tpl_brrw.elements[*ix_brrw - 1].clone();
                let new_fxn = TupleAccessElement { out: *element };
                Ok(Box::new(new_fxn))
            }
            (Value::MutableReference(tpl), Value::Index(ix)) => match &*tpl.borrow() {
                Value::Tuple(tpl) => {
                    let ix_brrw = ix.borrow();
                    let tpl_brrw = tpl.borrow();
                    if *ix_brrw > tpl_brrw.elements.len() || *ix_brrw < 1 {
                        return Err(MechError::new(
                            TupleIndexOutOfBoundsError {
                                ix: *ix_brrw,
                                len: tpl_brrw.elements.len(),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                    let element = tpl_brrw.elements[*ix_brrw - 1].clone();
                    let new_fxn = TupleAccessElement { out: *element };
                    Ok(Box::new(new_fxn))
                }
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (src.kind(), ix1.kind()),
                        fxn_name: "access/tuple-element".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
            _ => todo!(),
        }
    }
}
