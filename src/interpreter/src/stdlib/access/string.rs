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
enum StringAccessCompileMode {
  Static,
  Dynamic,
}

#[derive(Debug)]
struct StringAccessElement {
  source: StringAccessSource,
  ix: Ref<usize>,
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
          UnhandledFunctionArgumentKind2 { arg: (other.kind(), Value::Index(self.ix.clone()).kind()), fxn_name: "access/scalar-string".to_string() },
          None,
        ).with_compiler_loc()),
      },
    }
  }
}

impl MechFunctionImpl for StringAccessElement {
  fn solve(&self) {
    let _ = self.solve_result();
  }
  fn solve_result(&self) -> MResult<()> {
    let source = self.current_source_string()?;
    let ix = *self.ix.borrow();
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
      StringAccessCompileMode::Static => {
        ctx.features.insert(FeatureFlag::Builtin(FeatureKind::String));
        ctx.features.insert(FeatureFlag::Builtin(FeatureKind::Access));
        let reg = compile_register!(Value::String(self.out.clone()), ctx);
        Ok(reg)
      }
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
    match (src.clone(), ix1.clone()) {
      (Value::String(s), Value::Index(ix)) => {
        let grapheme = access_grapheme(&s.borrow(), *ix.borrow())?;
        let new_fxn = StringAccessElement { source: StringAccessSource::Direct(s), ix, out: Ref::new(grapheme), compile_mode: StringAccessCompileMode::Static };
        Ok(Box::new(new_fxn))
      },
      (Value::MutableReference(r), Value::Index(ix)) => {
        match &*r.borrow() {
          Value::String(s) => {
            let grapheme = access_grapheme(&s.borrow(), *ix.borrow())?;
            let new_fxn = StringAccessElement { source: StringAccessSource::Mutable(r.clone()), ix, out: Ref::new(grapheme), compile_mode: StringAccessCompileMode::Dynamic };
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
