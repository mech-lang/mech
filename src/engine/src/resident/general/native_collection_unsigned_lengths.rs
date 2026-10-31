//! Selected matrix length into an existing native scalar lane. The ordinary
//! maintained length implementation supplies shape and cardinality semantics.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::{
    native_artifact::NativeCollectionPrimitive as Primitive,
    resident::budget::{
        NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
    },
};
use mech_core::{IntegerWidth, MemoryBudgetLimits, ResourceDemand, SchemaBody, Value, ValueData};

impl NativePreparedLibraryCall {
    pub fn supports_unsigned_matrix_length(&self) -> bool {
        let [source] = self.template.inputs() else {
            return false;
        };
        matches!(source.body(), SchemaBody::Matrix { element, dimensions }
            if dimensions.len() == 2 && matches!(element.as_ref(),
                SchemaBody::UnsignedInteger(IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64)))
            && self.template.primitive() == Primitive::SequenceLength
            && self.template.output().body() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            && self.template.output().dimension_parameters().is_empty()
    }

    /// The caller establishes the exact current visit and supplies its owner
    /// coexistence. This method grants no execution or publication permission.
    pub fn unsigned_matrix_length(
        &self,
        source: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_unsigned_matrix_length() {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                coexistence.output_elements = coexistence
                    .output_elements
                    .checked_add(1)
                    .ok_or(InvokeError::InvalidInput)?;
                if let Some(violation) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: 1,
                        ..ResourceDemand::default()
                    },
                    8,
                    0,
                    limits,
                ) {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                    ));
                }
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(0, 0)?;
                context.charge(self.lookup_work, 0)?;
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if source.schema() != contract.inputs[0].schema
                    || source.schema_key() != self.input_keys[0]
                    || source.shape().parameter_values().len()
                        != self.template.inputs()[0].dimension_parameters().len()
                    || !matches!(source.data(), ValueData::Matrix(_))
                {
                    return Err(InvokeError::InvalidInput);
                }
                source
                    .validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let memory = work.memory_budget();
                let bytes = source
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let nodes = source
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let context = context.with_added_capacity(
                    bytes.checked_add(8).ok_or(InvokeError::InvalidInput)?,
                    nodes.checked_add(1).ok_or(InvokeError::InvalidInput)?,
                )?;
                context.check_capacity(0, 0)?;
                context.charge(128, 1)?;
                let ValueData::Matrix(matrix) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                // Canonical validation already proves rows * columns equals
                // the stored cell count. Read that header without walking
                // dimension expressions, using the maintained checked count.
                crate::resident::pure_text::matrix_length(1, matrix.elements().len())
                    .map_err(InvokeError::Kernel)
            },
        )?
    }
}
