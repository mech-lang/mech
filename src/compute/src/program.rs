use crate::{
    ComputeInputError, ComputeInputUpdate, ComputeKernel, ComputePhysicalPlan,
    ComputeRegionInterface,
};

/// A backend-neutral, fully planned resident compute region.
#[derive(Clone, Debug)]
pub struct ComputeProgram {
    interface: ComputeRegionInterface,
    plan: ComputePhysicalPlan,
    kernel: ComputeKernel,
}

impl ComputeProgram {
    pub fn new(
        interface: ComputeRegionInterface,
        plan: ComputePhysicalPlan,
        kernel: ComputeKernel,
    ) -> Self {
        Self {
            interface,
            plan,
            kernel,
        }
    }

    pub fn interface(&self) -> &ComputeRegionInterface {
        &self.interface
    }

    pub fn plan(&self) -> &ComputePhysicalPlan {
        &self.plan
    }

    pub fn kernel(&self) -> &ComputeKernel {
        &self.kernel
    }

    /// Validates an input update against the compiled interface and converts
    /// tensor storage to the canonical compute layout before session delivery.
    pub fn normalize_input_update(
        &self,
        update: ComputeInputUpdate,
    ) -> Result<ComputeInputUpdate, ComputeInputError> {
        self.interface.normalize_input_update(update)
    }
}
