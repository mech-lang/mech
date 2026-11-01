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
