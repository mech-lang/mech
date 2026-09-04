//! Checked layout and capacity derivation.

#[cfg(feature = "functions")]
use super::{
    ImplementationMemoryClass, MemoryFootprintWitness, PhysicalStorageDescriptor, RegionAccessPlan,
    TargetMemoryProfile,
};
#[cfg(feature = "functions")]
use crate::BoundCall;

#[cfg(feature = "functions")]
pub struct CallMemoryPlanningRequest<'a> {
    pub bound_call: &'a BoundCall,
    pub input_storage: &'a [PhysicalStorageDescriptor],
    pub output_storage: &'a [PhysicalStorageDescriptor],
    pub input_witnesses: &'a [MemoryFootprintWitness],
    pub output_witnesses: &'a [MemoryFootprintWitness],
    pub implementation_memory: ImplementationMemoryClass,
    pub target: &'a TargetMemoryProfile,
    pub regions: &'a [RegionAccessPlan],
}
