use mech_core::{
    ExecutionHostFunctionRequest, InitialSolvePolicy, MResult, MechExecutionServices,
    MechFunctionImpl, ReactiveDependencyScope, ValueCell,
};

#[cfg(feature = "semantic-compiler")]
use mech_core::{ApplicationRequirement, BytecodeCompilerContext, MechFunctionCompiler, Register};

#[derive(Clone, Debug)]
pub struct ExternalHostCallFunction {
    pub request: ExecutionHostFunctionRequest,
    pub arguments: Vec<ValueCell>,
    pub output: ValueCell,
    pub initial_solve_policy: InitialSolvePolicy,
}

impl ExternalHostCallFunction {
    fn solve_with_services(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        // Keep stable reactive inputs inside the plan, while exposing their
        // current logical values across the execution-service boundary.
        let arguments = self
            .arguments
            .iter()
            .map(ValueCell::snapshot)
            .collect::<MResult<Vec<_>>>()?;
        let result = services.invoke_host_function(&self.request, &arguments)?;
        frame.stage_output_value(&self.output, &result)
    }
}

impl MechFunctionImpl for ExternalHostCallFunction {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        self.solve_with_services(frame, services)?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
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

    fn to_string(&self) -> String {
        format!("ExternalHostCallFunction::{:?}", self.request)
    }
}

#[cfg(feature = "semantic-compiler")]
impl MechFunctionCompiler for ExternalHostCallFunction {
    fn compiler_owned_value_cells(&self) -> Vec<ValueCell> {
        vec![self.output.clone()]
    }

    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = super::compile_external_output(&self.output, context)?;
        let arguments = self
            .arguments
            .iter()
            .map(|argument| super::compile_external_cell(argument, context))
            .collect::<MResult<Vec<Register>>>()?;
        let requirement = context
            .intern_requirement(ApplicationRequirement::HostFunction(self.request.clone()))?;
        context.emit_host_call(requirement, output, arguments);
        Ok(output)
    }
}
