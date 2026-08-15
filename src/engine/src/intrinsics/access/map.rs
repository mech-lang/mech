#[macro_use]
use crate::intrinsics::*;

#[derive(Debug)]
pub struct MapAccessField {
    pub out: LegacyValue,
    pub source: Ref<MechMap>,
}

impl MechFunctionImpl for MapAccessField {
    fn solve_result(&self) -> MResult<()> {
        ();
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        self.out.clone()
    }
    fn to_string(&self) -> String {
        format!("{:#?}", self)
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}
#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for MapAccessField {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let mut registers = [0, 0];
        registers[0] = compile_register!(self.out, ctx);
        registers[1] = compile_register_brrw!(self.source, ctx);
        ctx.emit_unop(hash_str("MapAccessField"), registers[0], registers[1]);
        return Ok(registers[0]);
    }
}

pub fn impl_access_map_fxn(
    source: LegacyValue,
    key: LegacyValue,
) -> MResult<Box<dyn MechFunction>> {
    match (source, key) {
        (LegacyValue::Map(map), key) => {
            let map_ref = map.borrow();

            match map_ref.map.get(&key) {
                Some(value) => Ok(Box::new(MapAccessField {
                    out: value.clone(),
                    source: map.clone(),
                })),
                None => Err(MechError::new(
                    UndefinedMapKeyError {
                        key: key.to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            }
        }

        (source, key) => Err(MechError::new(
            UnhandledFunctionArgumentKind2 {
                arg: (source.kind(), key.kind()),
                fxn_name: "MapAccess".to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

pub struct MapAccess {}

impl FunctionSpecializer for MapAccess {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let src = &arguments[0];
        let key = &arguments[1];

        // Verify that the key as the right kind for the map
        match src.kind().deref_kind() {
            #[cfg(feature = "map")]
            ValueKind::Map(key_kind, _) => {
                if key.kind() != *key_kind {
                    return Err(MechError::new(
                        UnhandledFunctionArgumentKind2 {
                            arg: (src.kind(), key.kind()),
                            fxn_name: "MapAccess".to_string(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            }
            _ => unreachable!(),
        };

        match impl_access_map_fxn(src.clone(), key.clone()) {
            Ok(fxn) => Ok(fxn),
            Err(_) => match src {
                LegacyValue::MutableReference(map) => {
                    impl_access_map_fxn(map.borrow().clone(), key.clone())
                }
                _ => Err(MechError::new(
                    UnhandledFunctionArgumentKind2 {
                        arg: (src.kind(), key.kind()),
                        fxn_name: "MapAccess".to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct UndefinedMapKeyError {
    pub key: String,
}

impl MechErrorKind for UndefinedMapKeyError {
    fn name(&self) -> &str {
        "UndefinedMapKey"
    }
    fn message(&self) -> String {
        format!("Key id `{}` not found in key_index.", self.key)
    }
}
