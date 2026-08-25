use std::{collections::BTreeMap, sync::Arc};

use mech_core::{CellSlotId, ConstantId};
use mech_engine::ArtifactSource;

use crate::{
    ComputeInputError, ComputeInputUpdate, ComputeKernel, ComputePhysicalPlan,
    ComputeRegionInterface, FixedShapeStoragePlan,
};

/// Backend-neutral storage information needed to materialize an elementwise
/// program without consulting the compiler artifact again.
#[derive(Clone, Debug, Default)]
pub struct ElementwiseStoragePlan {
    pub slot_elements: BTreeMap<CellSlotId, u64>,
    pub constants: BTreeMap<ConstantId, Arc<[f32]>>,
    pub states: Box<[ElementwiseStateStorage]>,
    pub dispatch_elements: u64,
}

#[derive(Clone, Debug)]
pub struct ElementwiseStateStorage {
    pub slot: CellSlotId,
    pub source: ArtifactSource,
    pub elements: u64,
    pub initializer: Arc<[f32]>,
}

/// A backend-neutral, fully planned resident compute region.
#[derive(Clone, Debug)]
pub struct ComputeProgram {
    interface: ComputeRegionInterface,
    plan: ComputePhysicalPlan,
    kernel: ComputeKernel,
    elementwise_storage: Option<ElementwiseStoragePlan>,
    fixed_shape_storage: Option<FixedShapeStoragePlan>,
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
            elementwise_storage: None,
            fixed_shape_storage: None,
        }
    }

    pub fn with_elementwise_storage(mut self, storage: ElementwiseStoragePlan) -> Self {
        self.elementwise_storage = Some(storage);
        self
    }

    pub fn with_fixed_shape_storage(mut self, storage: FixedShapeStoragePlan) -> Self {
        self.fixed_shape_storage = Some(storage);
        self
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

    pub fn elementwise_storage(&self) -> Option<&ElementwiseStoragePlan> {
        self.elementwise_storage.as_ref()
    }

    pub fn fixed_shape_storage(&self) -> Option<&FixedShapeStoragePlan> {
        self.fixed_shape_storage.as_ref()
    }

    /// Validates an input update against the compiled interface and converts
    /// tensor storage to the canonical compute layout before session delivery.
    pub fn normalize_input_update(
        &self,
        update: ComputeInputUpdate,
    ) -> Result<ComputeInputUpdate, ComputeInputError> {
        let port = self
            .interface
            .input(update.port)
            .ok_or(ComputeInputError::UnknownInputPort { port: update.port })?;
        let value = if let Some(storage) = self.fixed_shape_storage() {
            port.normalize_broadcast_value(update.value, Some(storage.instances))
                .map(|(value, _)| value)
        } else {
            port.normalize_value(update.value)
        }
        .map_err(ComputeInputError::InvalidValue)?;
        Ok(ComputeInputUpdate {
            port: update.port,
            value,
        })
    }
}
