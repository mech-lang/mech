#[macro_use]
use crate::stdlib::*;
use self::assign::*;

use crate::*;

// Tuple Assign ----------------------------------------------------------------

#[derive(Debug)]
pub struct TupleAssign<T> {
  pub sink: Ref<T>,   // tuple element slot
  pub source: Ref<T> // rhs value
}

impl<T> MechFunctionImpl for TupleAssign<T>
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

  fn out(&self) -> Value {
    self.sink.to_value()
  }

  fn to_string(&self) -> String {
    format!("{:#?}", self)
  }
}

#[cfg(feature = "compiler")]
impl<T> MechFunctionCompiler for TupleAssign<T>
where
  T: CompileConst + ConstElem + AsValueKind,
{
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    let name = format!("TupleAssign<{}>", T::as_value_kind());
    compile_unop!(
      name,
      self.sink,
      self.source,
      ctx,
      FeatureFlag::Builtin(FeatureKind::Assign)
    );
  }
}

// -----------------------------------------------------------------------------

fn impl_tuple_assign_fxn(
  tuple: Value,
  source: Value,
  index: usize, // 0-based internally
) -> MResult<Box<dyn MechFunction>> {

  match &tuple {
    Value::Tuple(tuple_ref) => {
      let tuple = tuple_ref.borrow();

      if index >= tuple.size() {
        return Err(MechError2::new(
          TupleIndexOutOfBoundsError {
            ix: index + 1,
            len: tuple.size(),
          },
          None,
        ).with_compiler_loc());
      }

      let sink = tuple.elements[index].clone();

      match (&*sink, &source) {

        #[cfg(feature = "bool")]
        (Value::Bool(sink), Value::Bool(source)) => {
          Ok(Box::new(TupleAssign {
            sink: sink.clone(),
            source: source.clone(),
          }))
        }

        #[cfg(feature = "i64")]
        (Value::I64(sink), Value::I64(source)) => {
          Ok(Box::new(TupleAssign {
            sink: sink.clone(),
            source: source.clone(),
          }))
        }

        #[cfg(feature = "f64")]
        (Value::F64(sink), Value::F64(source)) => {
          Ok(Box::new(TupleAssign {
            sink: sink.clone(),
            source: source.clone(),
          }))
        }

        #[cfg(feature = "string")]
        (Value::String(sink), Value::String(source)) => {
          Ok(Box::new(TupleAssign {
            sink: sink.clone(),
            source: source.clone(),
          }))
        }

        _ => Err(MechError2::new(
          TupleElementKindMismatchError {
            expected: sink.kind(),
            actual: source.kind(),
          },
          None,
        ).with_compiler_loc()),
      }
    }

    _ => Err(MechError2::new(
      DestructureExpectedTupleError {
        value: tuple.kind(),
      },
      None,
    ).with_compiler_loc()),
  }
}

// -----------------------------------------------------------------------------

pub struct TupleAssignScalar {}

impl NativeFunctionCompiler for TupleAssignScalar {
  fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {

    if arguments.len() != 3 {
      return Err(MechError2::new(
        IncorrectNumberOfArguments {
          expected: 3,
          found: arguments.len(),
        },
        None,
      ).with_compiler_loc());
    }

    let tuple = arguments[0].clone();
    let source = arguments[1].clone();
    let index_val = arguments[2].clone();

    let index = match &index_val {
      Value::Index(ix) => {
        let ix = *ix.borrow() as isize;
        if ix <= 0 {
          return Err(MechError2::new(
            TupleIndexOutOfBoundsError {
              ix: ix as usize,
              len: 0,
            },
            None,
          ).with_compiler_loc());
        }
        (ix - 1) as usize
      }

      _ => {
        return Err(MechError2::new(
          UnhandledFunctionArgumentKind3 {
            arg: (tuple.kind(), source.kind(), index_val.kind()),
            fxn_name: "tuple/assign".to_string(),
          },
          None,
        ).with_compiler_loc());
      }
    };

    match impl_tuple_assign_fxn(tuple.clone(), source.clone(), index) {
      Ok(fxn) => Ok(fxn),

      Err(_) => match &tuple {
        Value::MutableReference(tuple_ref) => {
          impl_tuple_assign_fxn(tuple_ref.borrow().clone(), source, index)
        }

        _ => Err(MechError2::new(
          UnhandledFunctionArgumentKind3 {
            arg: (arguments[0].kind(), arguments[1].kind(), arguments[2].kind()),
            fxn_name: "tuple/assign".to_string(),
          },
          None,
        ).with_compiler_loc()),
      },
    }
  }
}

// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TupleElementKindMismatchError {
  pub expected: ValueKind,
  pub actual: ValueKind,
}

impl MechErrorKind2 for TupleElementKindMismatchError {
  fn name(&self) -> &str {
    "TupleElementKindMismatch"
  }

  fn message(&self) -> String {
    format!(
      "Tuple element kind mismatch: expected {:?}, found {:?}",
      self.expected, self.actual
    )
  }
}
