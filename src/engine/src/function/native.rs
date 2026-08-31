use std::sync::Arc;

#[cfg(feature = "semantic-compiler")]
use mech_core::{BytecodeCompilerContext, MechError, MechFunctionCompiler, Register};
use mech_core::{
    CanonicalFunctionSpecializer, FunctionInstance, FunctionInvocation, GuardFunctionSafety,
    MResult, MechErrorKind, MechFunctionImpl, SpecializationContext, SpecializationInput,
    SpecializationInvocation, SpecializedFunction, Value, ValueCell,
};

pub type NativeClosure = dyn Fn(Vec<Value>) -> MResult<Value> + Send + Sync + 'static;

#[derive(Clone)]
pub struct ClosureFunctionSpecializer {
    name: String,
    function: Arc<NativeClosure>,
}

impl ClosureFunctionSpecializer {
    pub fn new(
        name: impl Into<String>,
        function: impl Fn(Vec<Value>) -> MResult<Value> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            function: Arc::new(function),
        }
    }
}

impl CanonicalFunctionSpecializer for ClosureFunctionSpecializer {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        _context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let arguments = invocation
            .inputs()
            .iter()
            .map(SpecializationInput::snapshot)
            .collect::<MResult<Vec<_>>>()?;
        let output = ValueCell::from_snapshot((self.function)(arguments)?)?;
        let inputs = invocation
            .inputs()
            .iter()
            .map(SpecializationInput::cell)
            .map(|cell| cell.cloned())
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        let bound = FunctionInvocation::variadic(output, inputs);
        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(ClosureNativeFunction {
                name: self.name.clone(),
            }),
            bound,
        )))
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }
}

#[derive(Clone, Debug)]
pub struct ClosureNativeFunction {
    name: String,
}

impl MechFunctionImpl for ClosureNativeFunction {
    fn solve_result(&self) -> MResult<()> {
        // Pure closure functions are executed once during native function specialization.
        Ok(())
    }

    fn to_string(&self) -> String {
        format!("ClosureNativeFunction::{}", self.name)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for ClosureNativeFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Err(MechError::new(
            ClosureNativeFunctionNotBytecodeCompilableError {
                function: self.name.clone(),
            },
            None,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct ClosureNativeFunctionNotBytecodeCompilableError {
    pub function: String,
}

impl MechErrorKind for ClosureNativeFunctionNotBytecodeCompilableError {
    fn name(&self) -> &str {
        "ClosureNativeFunctionNotBytecodeCompilable"
    }

    fn message(&self) -> String {
        format!(
            "Native closure function `{}` cannot be compiled to bytecode yet",
            self.function,
        )
    }
}
