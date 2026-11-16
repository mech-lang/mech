//! Selected maintained Histogram scalar mechanics; never a second evaluator.
use super::{InvokeError, NativePreparedLibraryCall};

impl NativePreparedLibraryCall {
    fn require_selected_histogram(&self) -> Result<(), InvokeError> {
        #[cfg(feature = "map_ops")]
        if self.template.primitive()
            == crate::native_artifact::NativeCollectionPrimitive::StringHistogram
            && self.template.implementation_memory()
                == mech_core::ImplementationMemoryClass::CanonicalSortUnique
        {
            return Ok(());
        }
        Err(InvokeError::InvalidInput)
    }

    /// Exact maintained family eligible for a compiled immutable-matrix batch.
    /// This checks semantic schema shape, not a source name or key contents.
    pub fn supports_compiled_string_histogram(&self) -> bool {
        use mech_core::{CardinalitySpec, IntegerWidth, SchemaBody};
        self.require_selected_histogram().is_ok()
            && self.template.inputs().len() == 1
            && self.template.output().dimension_parameters().is_empty()
            && matches!(self.template.inputs()[0].body(),
                SchemaBody::Matrix { element, dimensions }
                if element.as_ref() == &SchemaBody::String && dimensions.len() == 2)
            && matches!(self.template.output().body(),
                SchemaBody::Map { key, value, cardinality }
                if key.as_ref() == &SchemaBody::String
                    && value.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                    && cardinality == &CardinalitySpec::Dynamic { upper_bound: None })
            && matches!(self.template.contract(),
                mech_core::ResolvedOperationContract::Declared(contract)
                if contract.inputs.len() == 1 && contract.outputs.len() == 1)
    }

    /// Storage-only immutable view. The caller must separately fund validation,
    /// retain genuine current ownership, and admit physical work/publication.
    /// A declined representation leaves the original selected call available.
    pub fn compiled_histogram_strings<'a>(
        &self,
        source: &'a mech_core::Value,
    ) -> Option<&'a [Box<str>]> {
        if !self.supports_compiled_string_histogram() {
            return None;
        }
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return None;
        };
        if source.schema() != contract.inputs[0].schema
            || source.schema_key() != self.input_keys[0]
            || source.shape().parameter_values().len()
                != self.template.inputs()[0].dimension_parameters().len()
            || source.validate_against(&self.schemas).is_err()
        {
            return None;
        }
        let mech_core::ValueData::Matrix(matrix) = source.data() else {
            return None;
        };
        let mech_core::snapshot::SequenceView::String(strings) = matrix.elements() else {
            return None;
        };
        Some(strings)
    }

    /// The caller has already paid for every byte that this comparison may
    /// inspect. Use the ordinary family's exact raw UTF-8 key order.
    pub fn selected_histogram_compare(
        &self,
        left: &str,
        right: &str,
    ) -> Result<core::cmp::Ordering, InvokeError> {
        self.require_selected_histogram()?;
        #[cfg(feature = "map_ops")]
        return Ok(crate::intrinsics::map_ops::histogram_key_order(left, right));
        #[cfg(not(feature = "map_ops"))]
        {
            let _ = (left, right);
            Err(InvokeError::InvalidInput)
        }
    }

    pub fn selected_histogram_increment(&self, count: u64) -> Result<u64, InvokeError> {
        self.require_selected_histogram()?;
        #[cfg(feature = "map_ops")]
        return crate::intrinsics::map_ops::increment_histogram_count(count).ok_or(
            InvokeError::Kernel(mech_core::ResidentKernelError::InvalidShape),
        );
        #[cfg(not(feature = "map_ops"))]
        {
            let _ = count;
            Err(InvokeError::InvalidInput)
        }
    }

    pub fn selected_histogram_footprint(
        &self,
        count: u64,
        payload: u64,
    ) -> Result<mech_core::snapshot::ValueFootprint, InvokeError> {
        self.require_selected_histogram()?;
        #[cfg(feature = "map_ops")]
        return crate::intrinsics::map_ops::prospective(
            crate::intrinsics::map_ops::Operation::Histogram,
            count,
            payload,
            self.output_parameters,
        )
        .map_err(|_| InvokeError::Kernel(mech_core::ResidentKernelError::InvalidShape));
        #[cfg(not(feature = "map_ops"))]
        {
            let _ = (count, payload);
            Err(InvokeError::InvalidInput)
        }
    }

    pub fn selected_histogram_draft_bytes(&self, count: u64) -> Result<u64, InvokeError> {
        self.require_selected_histogram()?;
        #[cfg(feature = "map_ops")]
        return crate::intrinsics::map_ops::canonical_draft_bytes(count).ok_or(
            InvokeError::Kernel(mech_core::ResidentKernelError::InvalidShape),
        );
        #[cfg(not(feature = "map_ops"))]
        {
            let _ = count;
            Err(InvokeError::InvalidInput)
        }
    }

    /// Actual ordinary radix scratch layout, including its private frame type.
    /// This is a representability proof only; the streaming strategy neither
    /// allocates this array nor discounts its own concrete vector/key owners.
    pub fn selected_histogram_original_scratch_bytes(
        &self,
        count: u64,
    ) -> Result<u64, InvokeError> {
        self.require_selected_histogram()?;
        #[cfg(feature = "map_ops")]
        return usize::try_from(count)
            .ok()
            .and_then(crate::intrinsics::map_ops::histogram_scratch_bytes)
            .filter(|bytes| *bytes <= isize::MAX as usize)
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(InvokeError::Kernel(
                mech_core::ResidentKernelError::InvalidShape,
            ));
        #[cfg(not(feature = "map_ops"))]
        {
            let _ = count;
            Err(InvokeError::InvalidInput)
        }
    }

    pub fn selected_histogram_finalization_work(
        &self,
        count: u64,
        payload: u64,
    ) -> Result<u64, InvokeError> {
        self.require_selected_histogram()?;
        #[cfg(feature = "map_ops")]
        return crate::intrinsics::map_ops::canonical_finalization_work(count, payload).ok_or(
            InvokeError::Kernel(mech_core::ResidentKernelError::InvalidShape),
        );
        #[cfg(not(feature = "map_ops"))]
        {
            let _ = (count, payload);
            Err(InvokeError::InvalidInput)
        }
    }
}
