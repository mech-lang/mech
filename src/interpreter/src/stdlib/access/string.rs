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
enum StringAccessSource {
  Direct(Ref<String>),
  Mutable(MutableReference),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StringAccessCompileMode {
  Constant,
  LiveDirect,
  Dynamic,
}

#[derive(Debug)]
enum StringAccessIndex {
  Direct(Ref<usize>),
  Mutable(MutableReference),
}

thread_local! {
  static NEXT_STRING_ACCESS_COMPILE_MODE: std::cell::RefCell<Option<StringAccessCompileMode>> = std::cell::RefCell::new(None);
}

pub(crate) fn set_next_string_access_compile_mode(mode: StringAccessCompileMode) {
  NEXT_STRING_ACCESS_COMPILE_MODE.with(|slot| {
    *slot.borrow_mut() = Some(mode);
  });
}

fn take_next_string_access_compile_mode() -> Option<StringAccessCompileMode> {
  NEXT_STRING_ACCESS_COMPILE_MODE.with(|slot| slot.borrow_mut().take())
}

#[derive(Debug)]
struct StringAccessElement {
  source: StringAccessSource,
  index: StringAccessIndex,
  out: Ref<String>,
  compile_mode: StringAccessCompileMode,
}

impl StringAccessElement {
  fn current_source_string(&self) -> MResult<String> {
    match &self.source {
      StringAccessSource::Direct(s) => Ok(s.borrow().clone()),
      StringAccessSource::Mutable(r) => match &*r.borrow() {
        Value::String(s) => Ok(s.borrow().clone()),
        other => Err(MechError::new(
          UnhandledFunctionArgumentKind2 { arg: (other.kind(), self.index_value_for_error().kind()), fxn_name: "access/scalar-string".to_string() },
          None,
        ).with_compiler_loc()),
      },
    }
  }

  fn current_index(&self) -> MResult<usize> {
    match &self.index {
      StringAccessIndex::Direct(ix) => Ok(*ix.borrow()),
      StringAccessIndex::Mutable(r) => {
        let current = r.borrow();
        match current.as_index()? {
          Value::Index(ix) => Ok(*ix.borrow()),
          other => Err(MechError::new(
            UnhandledFunctionArgumentKind2 { arg: (self.current_source_string().map(|_| ValueKind::String).unwrap_or(ValueKind::Empty), other.kind()), fxn_name: "access/scalar-string".to_string() },
            None,
          ).with_compiler_loc()),
        }
      }
    }
  }

  fn index_value_for_error(&self) -> Value {
    match &self.index {
      StringAccessIndex::Direct(ix) => Value::Index(ix.clone()),
      StringAccessIndex::Mutable(r) => Value::MutableReference(r.clone()),
    }
  }
}

impl MechFunctionImpl for StringAccessElement {
  fn solve(&self) {
    let _ = self.solve_result();
  }
  fn solve_result(&self) -> MResult<()> {
    let source = self.current_source_string()?;
    let ix = self.current_index()?;
    let grapheme = access_grapheme(&source, ix)?;
    *self.out.borrow_mut() = grapheme;
    Ok(())
  }
  fn out(&self) -> Value { Value::String(self.out.clone()) }
  fn to_string(&self) -> String { format!("{:#?}", self) }
}
#[cfg(feature = "compiler")]
impl MechFunctionCompiler for StringAccessElement {
  fn compile(&self, ctx: &mut CompileCtx) -> MResult<Register> {
    match self.compile_mode {
      StringAccessCompileMode::Constant => {
        ctx.features.insert(FeatureFlag::Builtin(FeatureKind::String));
        ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Access));
        let reg = compile_register!(Value::String(self.out.clone()), ctx);
        Ok(reg)
      }
      StringAccessCompileMode::LiveDirect => Err(MechError::new(
        GenericError {
          msg: "string scalar access cannot be bytecode-compiled because its source or index may be live; compile-time constant string access is supported, mutable/dynamic access is not yet supported".to_string(),
        },
        None,
      ).with_compiler_loc()),
      StringAccessCompileMode::Dynamic => Err(MechError::new(
        GenericError {
          msg: "dynamic string scalar access is not bytecode-compilable yet because it depends on live source/index registers".to_string(),
        },
        None,
      ).with_compiler_loc()),
    }
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
    fn direct_compile_mode(_s: &Ref<String>, _ix: &Ref<usize>) -> StringAccessCompileMode {
      // Expression lowering sets an explicit semantic mode when it can see that
      // a direct ref is a live plan output. Otherwise direct refs are constants
      // or immutable aliases and remain bytecode-compilable.
      take_next_string_access_compile_mode().unwrap_or(StringAccessCompileMode::Constant)
    }
    match (src.clone(), ix1.clone()) {
      (Value::String(s), Value::Index(ix)) => {
        let grapheme = access_grapheme(&s.borrow(), *ix.borrow())?;
        let compile_mode = direct_compile_mode(&s, &ix);
        let new_fxn = StringAccessElement { source: StringAccessSource::Direct(s), index: StringAccessIndex::Direct(ix), out: Ref::new(grapheme), compile_mode };
        Ok(Box::new(new_fxn))
      },
      (Value::String(s), Value::MutableReference(ix_ref)) => {
        let ix = match ix_ref.borrow().as_index()? {
          Value::Index(ix) => *ix.borrow(),
          other => return Err(MechError::new(UnhandledFunctionArgumentKind2 { arg: (src.kind(), other.kind()), fxn_name: "access/scalar-string".to_string() }, None).with_compiler_loc()),
        };
        let grapheme = access_grapheme(&s.borrow(), ix)?;
        let new_fxn = StringAccessElement { source: StringAccessSource::Direct(s), index: StringAccessIndex::Mutable(ix_ref), out: Ref::new(grapheme), compile_mode: StringAccessCompileMode::Dynamic };
        Ok(Box::new(new_fxn))
      },
      (Value::MutableReference(r), Value::Index(ix)) => {
        match &*r.borrow() {
          Value::String(s) => {
            let grapheme = access_grapheme(&s.borrow(), *ix.borrow())?;
            let new_fxn = StringAccessElement { source: StringAccessSource::Mutable(r.clone()), index: StringAccessIndex::Direct(ix), out: Ref::new(grapheme), compile_mode: StringAccessCompileMode::Dynamic };
            Ok(Box::new(new_fxn))
          },
          _ => Err(MechError::new(
              UnhandledFunctionArgumentKind2 { arg: (src.kind(), ix1.kind()), fxn_name: "access/scalar-string".to_string() },
              None
            ).with_compiler_loc()
          ),
        }
      },
      (Value::MutableReference(r), Value::MutableReference(ix_ref)) => {
        let ix = match ix_ref.borrow().as_index()? {
          Value::Index(ix) => *ix.borrow(),
          other => return Err(MechError::new(UnhandledFunctionArgumentKind2 { arg: (src.kind(), other.kind()), fxn_name: "access/scalar-string".to_string() }, None).with_compiler_loc()),
        };
        match &*r.borrow() {
          Value::String(s) => {
            let grapheme = access_grapheme(&s.borrow(), ix)?;
            let new_fxn = StringAccessElement { source: StringAccessSource::Mutable(r.clone()), index: StringAccessIndex::Mutable(ix_ref), out: Ref::new(grapheme), compile_mode: StringAccessCompileMode::Dynamic };
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
