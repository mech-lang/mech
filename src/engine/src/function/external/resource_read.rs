use crate::apply_stable_value_update;
use mech_core::{
    ExecutionResourceRequest, InitialSolvePolicy, MResult, MechExecutionServices, MechFunctionImpl,
    NoMechExecutionServices, ReactiveSolveStatus, ResourceDelivery, ValRef, Value,
};

#[cfg(feature = "compiler")]
use mech_core::{ApplicationRequirement, BytecodeCompilerContext, MechFunctionCompiler, Register};

#[derive(Clone, Debug)]
pub struct ExternalResourceReadFunction {
    pub interpreter_id: u64,
    pub request: ExecutionResourceRequest,
    pub output: ValRef,
    pub initial_solve_policy: InitialSolvePolicy,
}

impl ExternalResourceReadFunction {
    fn solve_with_services(&self, services: &mut dyn MechExecutionServices) -> MResult<()> {
        let result = services.read_resource(&self.request)?;
        apply_stable_value_update(self.output.clone(), result)?;
        if self.request.delivery == ResourceDelivery::Live {
            services.bind_live_resource(self.interpreter_id, &self.request, self.output.clone())?;
        }
        Ok(())
    }
}

impl MechFunctionImpl for ExternalResourceReadFunction {
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

    fn initialize_preserved_output_with(
        &self,
        services: &mut dyn MechExecutionServices,
    ) -> MResult<()> {
        if self.request.delivery == ResourceDelivery::Live {
            services.bind_live_resource(self.interpreter_id, &self.request, self.output.clone())?;
        }
        Ok(())
    }

    fn out(&self) -> Value {
        self.output.borrow().clone()
    }

    fn transaction_state_values(&self) -> MResult<Vec<Value>> {
        Ok(vec![Value::MutableReference(self.output.clone())])
    }

    fn to_string(&self) -> String {
        format!("ExternalResourceReadFunction::{:?}", self.request)
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for ExternalResourceReadFunction {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = super::compile_external_output(&self.output, context)?;
        let requirement =
            context.intern_requirement(ApplicationRequirement::Resource(self.request.clone()))?;
        context.emit_resource_read(requirement, output);
        Ok(output)
    }
}
