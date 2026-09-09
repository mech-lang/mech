use mech_core::{
    ExecutionHostFunctionRequest, InitialSolvePolicy, MResult, MechError, MechExecutionServices,
    MechFunctionImpl, ReactiveDependencyScope, Ref, Value, ValueCell,
};

#[cfg(feature = "semantic-compiler")]
use mech_core::{ApplicationRequirement, BytecodeCompilerContext, MechFunctionCompiler, Register};

#[derive(Clone, Debug)]
pub struct ExternalHostCallFunction {
    pub request: ExecutionHostFunctionRequest,
    pub arguments: Vec<ValueCell>,
    pub output: ValueCell,
    pub initial_solve_policy: InitialSolvePolicy,
    prepared_result: Ref<Option<Value>>,
}

impl ExternalHostCallFunction {
    pub fn new(
        request: ExecutionHostFunctionRequest,
        arguments: Vec<ValueCell>,
        output: ValueCell,
        initial_solve_policy: InitialSolvePolicy,
    ) -> Self {
        Self {
            request,
            arguments,
            output,
            initial_solve_policy,
            prepared_result: Ref::new(None),
        }
    }

    fn solve_with_services(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        let _ = services;
        let result = self.prepared_result.borrow_mut().take().ok_or_else(|| {
            MechError::new(
                mech_core::GenericError {
                    msg: "host result was not captured before managed output planning".to_owned(),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        frame.stage_output_value(&self.output, result)
    }
}

impl MechFunctionImpl for ExternalHostCallFunction {
    fn payload_output_plan_policy(&self) -> mech_core::PayloadOutputPlanPolicy {
        mech_core::PayloadOutputPlanPolicy::ExternalAdoption
    }

    fn prepare_external_output(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        // Keep stable reactive inputs inside the plan, while exposing their
        // current logical values across the execution-service boundary.
        let arguments = self
            .arguments
            .iter()
            .map(ValueCell::snapshot)
            .collect::<MResult<Vec<_>>>()?;
        let result = services.invoke_host_function(&self.request, &arguments)?;
        *self.prepared_result.borrow_mut() = Some(result);
        Ok(())
    }

    fn planned_output_shapes(&self) -> MResult<Option<Box<[mech_core::ShapeInstance]>>> {
        Ok(self
            .prepared_result
            .borrow()
            .as_ref()
            .map(|value| vec![value.shape().clone()].into_boxed_slice()))
    }

    fn planned_output_footprints(
        &self,
    ) -> MResult<Option<Box<[mech_core::CurrentMemoryFootprint]>>> {
        let prepared = self.prepared_result.borrow();
        let Some(value) = prepared.as_ref() else {
            return Ok(None);
        };
        Ok(Some(
            vec![ValueCell::prospective_snapshot_memory_footprint(value)?].into_boxed_slice(),
        ))
    }

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
