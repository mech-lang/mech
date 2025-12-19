#[macro_use]
use crate::stdlib::*;

#[derive(Debug)]
pub struct MapAccessField {
  pub out: Value,
  pub source: Ref<MechMap>,
}

impl MechFunctionImpl for MapAccessField {
  fn solve(&self) {
    ()
  }
  fn out(&self) -> Value {self.out.clone()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for MapAccessField {
    fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
        let mut registers = [0, 0];
        registers[0] = compile_register!(self.out, ctx);
        registers[1] = compile_register_brrw!(self.source, ctx);
        ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Access));
        ctx.emit_unop(
          hash_str("MapAccessField"),
          registers[0],
          registers[1],
        );
        return Ok(registers[0]);
    }
    }

pub fn impl_access_map_fxn(source: Value, key: Value) -> MResult<Box<dyn MechFunction>> {
  match (source, key) {

    (Value::Map(map), key) => {
      let map_ref = map.borrow();

      match map_ref.map.get(&key) {
        Some(value) => Ok(Box::new(MapAccessField {
          out: value.clone(),
          source: map.clone(),
        })),
        None => Err(MechError2::new(
          UndefinedMapKeyError { key: key.to_string() },
          None,
        ).with_compiler_loc()),
      }
    }

    (source, key) => Err(MechError2::new(
      UnhandledFunctionArgumentKind2 {
        arg: (source.kind(), key.kind()),
        fxn_name: "MapAccess".to_string(),
      },
      None,
    ).with_compiler_loc()),
  }
}

pub struct MapAccess {}

impl NativeFunctionCompiler for MapAccess {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() != 2 {
      return Err(MechError2::new(
        IncorrectNumberOfArguments {
          expected: 2,
          found: arguments.len(),
        },
        None,
      ).with_compiler_loc());
    }

    let src = &arguments[0];
    let key = &arguments[1];

    // Verify that the key as the right kind for the map
    match src.kind().deref_kind() {
      #[cfg(feature = "map")]
      ValueKind::Map(key_kind, _) => {
        if key.kind() != *key_kind {
          return Err(MechError2::new(
            UnhandledFunctionArgumentKind2 { arg: (src.kind(), key.kind()), fxn_name: "MapAccess".to_string() },
            None,
          ).with_compiler_loc());
        }
      }
      _ => unreachable!(),
    };

    match impl_access_map_fxn(src.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),
      Err(_) => {
        match src {
          Value::MutableReference(map) => {
            impl_access_map_fxn(map.borrow().clone(), key.clone())
          }
          _ => Err(MechError2::new(
            UnhandledFunctionArgumentKind2 {
              arg: (src.kind(), key.kind()),
              fxn_name: "MapAccess".to_string(),
            },
            None,
          ).with_compiler_loc()),
        }
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct UndefinedMapKeyError {
  pub key: String,
}

impl MechErrorKind2 for UndefinedMapKeyError {
  fn name(&self) -> &str {
    "UndefinedMapKey"
  }
  fn message(&self) -> String {
    format!("Key id `{}` not found in key_index.", self.key)
  }
}