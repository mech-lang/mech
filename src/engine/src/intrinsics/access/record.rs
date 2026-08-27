use crate::intrinsics::*;

// Record Access --------------------------------------------------------------

#[derive(Debug)]
pub struct RecordAccessField {
    pub source: LegacyValue,
}
impl MechFunctionImpl for RecordAccessField {
    fn solve_result(&self) -> MResult<()> {
        ();
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.source.clone()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for RecordAccessField {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0];

        registers[0] = compile_register!(self.source, ctx);

        ctx.emit_nullop(hash_str("RecordAccessField"), registers[0]);

        return Ok(registers[0]);
    }
}

pub fn impl_access_record_fxn(
    source: LegacyValue,
    key: LegacyValue,
) -> MResult<Box<dyn MechFunction>> {
    match (source, key) {
        (LegacyValue::Record(rcd), LegacyValue::Id(id)) => {
            let k = id;
            match rcd.borrow().get(&k) {
                Some(value) => Ok(Box::new(RecordAccessField {
                    source: value.clone(),
                })),
                None => Err(
                    MechError::new(UndefinedRecordFieldError { id: k.clone() }, None)
                        .with_compiler_loc(),
                ),
            }
        }
        (source, key) => {
            return Err(MechError::new(
                UnhandledFunctionArgumentKind2 {
                    arg: (source.kind(), key.kind()),
                    fxn_name: "RecordAccess".to_string(),
                },
                None,
            )
            .with_compiler_loc());
        }
    }
}

pub struct RecordAccess {}
impl FunctionSpecializer for RecordAccess {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let key = &arguments[1];
        let src = &arguments[0];
        match impl_access_record_fxn(src.clone(), key.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match src {
                LegacyValue::MutableReference(rcrd) => {
                    impl_access_record_fxn(rcrd.borrow().clone(), key.clone())
                }
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (src.kind(), key.kind()),
                        fxn_name: "RecordAccess".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

#[derive(Debug)]
pub struct RecordAccessSwizzle {
    pub source: LegacyValue,
}

impl MechFunctionImpl for RecordAccessSwizzle {
    fn solve_result(&self) -> MResult<()> {
        ();
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.source.clone()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for RecordAccessSwizzle {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0];

        registers[0] = compile_register!(self.source, ctx);

        ctx.emit_nullop(hash_str("RecordAccessSwizzle"), registers[0]);

        return Ok(registers[0]);
    }
}
