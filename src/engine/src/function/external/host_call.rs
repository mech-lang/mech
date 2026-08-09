use crate::apply_stable_value_update;
use mech_core::{
    ExecutionHostFunctionRequest, InitialSolvePolicy, LegacyValue, MResult, MechExecutionServices,
    MechFunctionImpl, NoMechExecutionServices, ReactiveDependencyScope, ReactiveSolveStatus,
    ValRef,
};

#[cfg(feature = "compiler")]
use mech_core::{ApplicationRequirement, BytecodeCompilerContext, MechFunctionCompiler, Register};

#[derive(Clone, Debug)]
pub struct ExternalHostCallFunction {
    pub request: ExecutionHostFunctionRequest,
    pub arguments: Vec<LegacyValue>,
    pub output: ValRef,
    pub initial_solve_policy: InitialSolvePolicy,
}

impl ExternalHostCallFunction {
    fn solve_with_services(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        // Keep stable reactive inputs inside the plan, while exposing their
        // current logical values across the execution-service boundary.
        let arguments = self
            .arguments
            .iter()
            .map(LegacyValue::try_deep_snapshot)
            .collect::<MResult<Vec<_>>>()?;
        let result = services.invoke_host_function(&self.request, &arguments)?;
        apply_stable_value_update(self.output.clone(), result)?;
        Ok(())
    }
}

impl MechFunctionImpl for ExternalHostCallFunction {
    fn solve_result(&self) -> MResult<()> {
        self.solve_with_services(&mut NoMechExecutionServices)
    }

    fn solve_result_with(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        self.solve_with_services(services)
    }

    fn solve_reactive(&self) -> MResult<ReactiveSolveStatus> {
        self.solve_result()?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn solve_reactive_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<ReactiveSolveStatus> {
        self.solve_with_services(services)?;
        Ok(ReactiveSolveStatus::Changed)
    }

    fn initial_solve_policy(&self) -> InitialSolvePolicy {
        self.initial_solve_policy
    }

    fn reactive_dependency_scopes(
        &self,
        argument_count: usize,
    ) -> Option<Vec<ReactiveDependencyScope>> {
        Some(vec![ReactiveDependencyScope::Logical; argument_count])
    }

    fn out(&self) -> LegacyValue {
        self.output.borrow().clone()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(vec![LegacyValue::MutableReference(self.output.clone())])
    }

    fn to_string(&self) -> String {
        format!("ExternalHostCallFunction::{:?}", self.request)
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for ExternalHostCallFunction {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = super::compile_external_output(&self.output, context)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| super::compile_external_value(argument, context))
            .collect::<MResult<Vec<Register>>>()?;
        let requirement = context
            .intern_requirement(ApplicationRequirement::HostFunction(self.request.clone()))?;
        context.emit_host_call(requirement, output, arguments);
        Ok(output)
    }
}
