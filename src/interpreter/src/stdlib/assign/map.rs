#[macro_use] 
use crate::stdlib::*; 
use self::assign::*;

#[derive(Debug)]
pub struct MapAssign<T> {
  pub sink: Ref<T>,   // map value slot
  pub source: Ref<T>, // rhs value
}

impl<T> MechFunctionImpl for MapAssign<T>
where
  T: Debug + Clone + Sync + Send + PartialEq + 'static,
  Ref<T>: ToValue,
{
  fn solve(&self) {
    let source_ptr = self.source.as_ptr();
    let sink_ptr = self.sink.as_mut_ptr();
    unsafe {
      *sink_ptr = (*source_ptr).clone();
    }
  }
  fn out(&self) -> Value { self.sink.to_value()}
  fn to_string(&self) -> String {format!("{:#?}", self)}
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for MapAssign<T>
where
  T: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("MapAssign<{}>", T::as_value_kind());
    compile_unop!(name,self.sink,self.source,ctx,FeatureFlag::Builtin(FeatureKind::Assign));
  }
}


fn impl_set_map_value_fxn(
  sink: Value,
  source: Value,
  key: Value,
) -> MResult<Box<dyn MechFunction>> {
  match &sink {
    Value::Map(map_ref) => {
      let mut map = map_ref.borrow_mut();

      // Key kind check
      if key.kind() != map.key_kind {
        return Err(MechError2::new(
          MapKeyKindMismatchError {
            expected_kind: map.key_kind.clone(),
            actual_kind: key.kind(),
          },
          None,
        )
        .with_compiler_loc());
      }

      // Get existing value slot or insert a default value
      let value = map.map.entry(key.clone()).or_insert_with(|| {
        source.clone()
      }).clone();

      // Dispatch by concrete value type
      match (&value, &source) {
        #[cfg(feature = "bool")]
        (Value::Bool(sink), Value::Bool(source)) => {
          Ok(Box::new(MapAssign { sink: sink.clone(), source: source.clone() }))
        }

        #[cfg(feature = "i64")]
        (Value::I64(sink), Value::I64(source)) => {
          Ok(Box::new(MapAssign { sink: sink.clone(), source: source.clone() }))
        }

        #[cfg(feature = "f64")]
        (Value::F64(sink), Value::F64(source)) => {
          Ok(Box::new(MapAssign { sink: sink.clone(), source: source.clone() }))
        }

        #[cfg(feature = "string")]
        (Value::String(sink), Value::String(source)) => {
          Ok(Box::new(MapAssign { sink: sink.clone(), source: source.clone() }))
        }

        _ => Err(MechError2::new(
          MapValueKindMismatchError {
            expected_kind: map.value_kind.clone(),
            actual_kind: source.kind(),
          },
          None,
        )
        .with_compiler_loc()),
      }
    }

    _ => Err(MechError2::new(
      UnhandledFunctionArgumentKind3 {
        arg: (sink.kind(), source.kind(), key.kind()),
        fxn_name: "map/assign".to_string(),
      },
      None,
    )
    .with_compiler_loc()),
  }
}


pub struct MapAssignScalar {}

impl NativeFunctionCompiler for MapAssignScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
    if arguments.len() < 3 {
      return Err(MechError2::new(
        IncorrectNumberOfArguments {
          expected: 3,
          found: arguments.len(),
        },
        None,
      )
      .with_compiler_loc());
    }

    let sink = arguments[0].clone();
    let source = arguments[1].clone();
    let key = arguments[2].clone();

    match impl_set_map_value_fxn(sink.clone(), source.clone(), key.clone()) {
      Ok(fxn) => Ok(fxn),

      Err(_) => match &sink {
        Value::MutableReference(sink_ref) => {
          impl_set_map_value_fxn(sink_ref.borrow().clone(), source, key)
        }
        _ => Err(MechError2::new(
          UnhandledFunctionArgumentKind3 {
            arg: (arguments[0].kind(), arguments[1].kind(), arguments[2].kind()),
            fxn_name: "map/assign".to_string(),
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
  pub key: ValueKind,
}

impl MechErrorKind2 for UndefinedMapKeyError {
  fn name(&self) -> &str {
    "UndefinedMapKey"
  }

  fn message(&self) -> String {
    format!("Key {:?} is not defined in this map.", self.key)
  }
}
