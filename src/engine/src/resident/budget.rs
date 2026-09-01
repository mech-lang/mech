use mech_core::ResidentKernelError;

pub(crate) const MAX_RESIDENT_COMPARISON_WORK: usize = 65_536;
pub(crate) const MAX_RESIDENT_COMPUTE_WORK: usize = 16_777_216;
pub(crate) const MAX_RESIDENT_OUTPUT_ELEMENTS: usize = 65_536;
pub(crate) const MAX_RESIDENT_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_RESIDENT_TEMPORARY_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const MAX_RESIDENT_CLONED_BYTES: usize = 16 * 1024 * 1024;

/// A checked, operation-independent estimate made before a resident kernel
/// allocates an expansion or starts mutating its output.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct KernelCostEstimate {
    pub comparison_work: usize,
    pub compute_work: usize,
    pub output_elements: usize,
    pub output_bytes: usize,
    pub temporary_bytes: usize,
    pub cloned_bytes: usize,
}

impl KernelCostEstimate {
    pub(crate) fn admit(self) -> Result<(), ResidentKernelError> {
        if self.comparison_work > MAX_RESIDENT_COMPARISON_WORK
            || self.compute_work > MAX_RESIDENT_COMPUTE_WORK
            || self.output_elements > MAX_RESIDENT_OUTPUT_ELEMENTS
            || self.output_bytes > MAX_RESIDENT_OUTPUT_BYTES
            || self.temporary_bytes > MAX_RESIDENT_TEMPORARY_BYTES
            || self.cloned_bytes > MAX_RESIDENT_CLONED_BYTES
        {
            return Err(ResidentKernelError::InvalidShape);
        }
        Ok(())
    }
}

pub(crate) fn checked_product(values: &[usize]) -> Result<usize, ResidentKernelError> {
    values.iter().try_fold(1usize, |product, value| {
        product
            .checked_mul(*value)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

pub(crate) fn checked_sum(values: &[usize]) -> Result<usize, ResidentKernelError> {
    values.iter().try_fold(0usize, |sum, value| {
        sum.checked_add(*value)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_budget_fails_closed_on_limits_and_arithmetic() {
        assert_eq!(KernelCostEstimate::default().admit(), Ok(()));
        assert_eq!(
            KernelCostEstimate {
                cloned_bytes: MAX_RESIDENT_CLONED_BYTES + 1,
                ..KernelCostEstimate::default()
            }
            .admit(),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            checked_product(&[usize::MAX, 2]),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            checked_sum(&[usize::MAX, 1]),
            Err(ResidentKernelError::InvalidShape)
        );
    }
}
