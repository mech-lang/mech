#[macro_use]
use crate::intrinsics::*;
use self::assign::*;

// x.a = 1 --------------------------------------------------------------------

// Record Set -----------------------------------------------------------------

#[derive(Debug)]
pub struct RecordAssign<T> {
    pub sink: Ref<T>,
    pub source: Ref<T>,
}
impl<T> MechFunctionImpl for RecordAssign<T>
where
    T: Debug + Clone + Sync + Send + PartialEq + 'static,
    Ref<T>: ToValue,
{
    fn solve_result(&self) -> MResult<()> {
        let source_ptr = self.source.as_ptr();
        let sink_ptr = self.sink.as_mut_ptr();
        unsafe {
            *sink_ptr = (*source_ptr).clone();
        };
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.sink.to_value()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl<T> MechFunctionCompiler for RecordAssign<T>
where
    T: CompileConst + ConstElem + AsValueKind,
{
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let name = format!("RecordAssign<{}>", T::as_value_kind());
        compile_unop!(name, self.sink, self.source, ctx);
    }
}

fn impl_set_record_column_fxn(
    sink: LegacyValue,
    source: LegacyValue,
    key: LegacyValue,
) -> MResult<Box<dyn MechFunction>> {
    match (&sink, &source, &key) {
        (LegacyValue::Record(rcrd), source, LegacyValue::Id(k)) => {
            let rcrd_brrw = rcrd.borrow();
            match (rcrd_brrw.data.get(k), source) {
                #[cfg(all(feature = "bool", feature = "record"))]
                (Some(LegacyValue::Bool(sink)), LegacyValue::Bool(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "i8", feature = "record"))]
                (Some(LegacyValue::I8(sink)), LegacyValue::I8(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "i16", feature = "record"))]
                (Some(LegacyValue::I16(sink)), LegacyValue::I16(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "i32", feature = "record"))]
                (Some(LegacyValue::I32(sink)), LegacyValue::I32(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "i64", feature = "record"))]
                (Some(LegacyValue::I64(sink)), LegacyValue::I64(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "i128", feature = "record"))]
                (Some(LegacyValue::I128(sink)), LegacyValue::I128(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "u8", feature = "record"))]
                (Some(LegacyValue::U8(sink)), LegacyValue::U8(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "u16", feature = "record"))]
                (Some(LegacyValue::U16(sink)), LegacyValue::U16(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "u32", feature = "record"))]
                (Some(LegacyValue::U32(sink)), LegacyValue::U32(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "u64", feature = "record"))]
                (Some(LegacyValue::U64(sink)), LegacyValue::U64(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "u128", feature = "record"))]
                (Some(LegacyValue::U128(sink)), LegacyValue::U128(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "f32", feature = "record"))]
                (Some(LegacyValue::F32(sink)), LegacyValue::F32(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "f64", feature = "record"))]
                (Some(LegacyValue::F64(sink)), LegacyValue::F64(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "string", feature = "record"))]
                (Some(LegacyValue::String(sink)), LegacyValue::String(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "complex", feature = "record"))]
                (Some(LegacyValue::C64(sink)), LegacyValue::C64(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                #[cfg(all(feature = "rational", feature = "record"))]
                (Some(LegacyValue::R64(sink)), LegacyValue::R64(source)) => {
                    return Ok(Box::new(RecordAssign {
                        sink: sink.clone(),
                        source: source.clone(),
                    }));
                }
                _ => {
                    return Err(
                        MechError::new(UndefinedRecordFieldError { id: k.clone() }, None)
                            .with_compiler_loc(),
                    );
                }
            }
        }
        _ => {
            return Err(MechError::new(
                UnhandledFunctionArgumentKind3 {
                    arg: (sink.kind(), source.kind(), key.kind()),
                    fxn_name: "record/assign-field".to_string(),
                },
                None,
            )
            .with_compiler_loc());
        }
    }
}

pub struct AssignRecordField {}
impl FunctionSpecializer for AssignRecordField {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() < 3 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let sink = arguments[0].clone();
        let source = arguments[1].clone();
        let key = arguments[2].clone();
        match impl_set_record_column_fxn(sink.clone(), source.clone(), key.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match (&sink, &source, &key) {
                (LegacyValue::MutableReference(sink), _, _) => {
                    impl_set_record_column_fxn(sink.borrow().clone(), source.clone(), key.clone())
                }
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind3 {
                        arg: (
                            arguments[0].kind(),
                            arguments[1].kind(),
                            arguments[2].kind(),
                        ),
                        fxn_name: "record/assign-field".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct UndefinedRecordFieldError {
    pub id: u64,
}
impl MechErrorKind for UndefinedRecordFieldError {
    fn name(&self) -> &str {
        "UndefinedRecordField"
    }
    fn message(&self) -> String {
        format!("Field {:?} is not defined in this record.", self.id)
    }
}
