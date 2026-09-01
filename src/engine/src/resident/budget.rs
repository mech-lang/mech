use mech_core::ResidentKernelError;

macro_rules! resident_cost {
    (@value $field:ident, $value:expr) => {
        $value
    };
    (@value $field:ident) => {
        $field
    };
    (
        $( $field:ident $( : $value:expr )? ,)*
        .. $base:expr $(,)?
    ) => {{
        $crate::resident::budget::KernelCostEstimate {
            $(
                $field: $crate::resident::budget::checked_u64(
                    $crate::resident::budget::resident_cost!(@value $field $(, $value)?)
                )?,
            )*
            ..$base
        }
    }};
}

pub(crate) use resident_cost;

pub(crate) const MAX_RESIDENT_COMPARISON_WORK: u64 = 65_536;
pub(crate) const MAX_RESIDENT_COMPUTE_WORK: u64 = 16_777_216;
pub(crate) const MAX_RESIDENT_OUTPUT_ELEMENTS: u64 = 65_536;
pub(crate) const MAX_RESIDENT_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RESIDENT_TEMPORARY_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RESIDENT_CLONED_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_RESIDENT_RETAINED_NODES: u64 = 65_536;

/// A checked, operation-independent estimate made before a resident kernel
/// allocates an expansion or starts mutating its output.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct KernelCostEstimate {
    pub comparison_work: u64,
    pub compute_work: u64,
    pub output_elements: u64,
    /// Retained output storage after publication.
    pub output_bytes: u64,
    /// Peak staging storage retained simultaneously before publication.
    pub temporary_bytes: u64,
    pub cloned_bytes: u64,
    pub container_bytes: u64,
    pub selector_bytes: u64,
    pub index_bytes: u64,
    pub retained_nodes: u64,
}

/// Complete retained state after a mutation publishes, never merely the bytes
/// changed by the current execution.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PublishedOutputFootprint {
    pub elements: u64,
    pub retained_bytes: u64,
    pub retained_nodes: u64,
}

/// A mutation plan whose final published footprint is part of the admission
/// authority rather than an optional call-site convention.
#[derive(Clone, Debug)]
pub(crate) struct PreparedMutationPlan<P> {
    operation: P,
    final_output: PublishedOutputFootprint,
    cost: KernelCostEstimate,
}

#[derive(Clone, Debug)]
pub(crate) struct AdmittedMutationPlan<P> {
    operation: P,
    _permit: ResidentBudgetPermit,
}

/// Incremental fail-closed accounting for data-dependent borrowed traversals.
/// Every charge is checked immediately, so measuring a late or missing key
/// cannot perform more resident work than the shared limits permit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ResidentBudgetMeter {
    accumulated: KernelCostEstimate,
}

/// Authority proving one complete checked estimate passed central resident
/// admission. Its fields and constructor are private so materializers cannot
/// manufacture a permit locally.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ResidentBudgetPermit {
    _cost: AdmittedKernelCost,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AdmittedKernelCost {
    comparison_work: u64,
    compute_work: u64,
    output_elements: u64,
    output_bytes: u64,
    temporary_peak_bytes: u64,
    cloned_bytes: u64,
    retained_nodes: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedKernel<P> {
    plan: P,
    cost: KernelCostEstimate,
}

#[derive(Clone, Debug)]
pub(crate) struct AdmittedKernel<P> {
    plan: P,
    _permit: ResidentBudgetPermit,
}

impl KernelCostEstimate {
    fn checked(self) -> Result<AdmittedKernelCost, ResidentKernelError> {
        let temporary_peak_bytes = self
            .temporary_bytes
            .checked_add(self.container_bytes)
            .and_then(|bytes| bytes.checked_add(self.selector_bytes))
            .and_then(|bytes| bytes.checked_add(self.index_bytes))
            .ok_or(ResidentKernelError::InvalidShape)?;
        let cost = AdmittedKernelCost {
            comparison_work: self.comparison_work,
            compute_work: self.compute_work,
            output_elements: self.output_elements,
            output_bytes: self.output_bytes,
            temporary_peak_bytes,
            cloned_bytes: self.cloned_bytes,
            retained_nodes: self.retained_nodes,
        };
        if cost.comparison_work > MAX_RESIDENT_COMPARISON_WORK
            || cost.compute_work > MAX_RESIDENT_COMPUTE_WORK
            || cost.output_elements > MAX_RESIDENT_OUTPUT_ELEMENTS
            || cost.output_bytes > MAX_RESIDENT_OUTPUT_BYTES
            || cost.temporary_peak_bytes > MAX_RESIDENT_TEMPORARY_BYTES
            || cost.cloned_bytes > MAX_RESIDENT_CLONED_BYTES
            || cost.retained_nodes > MAX_RESIDENT_RETAINED_NODES
        {
            return Err(ResidentKernelError::InvalidShape);
        }
        Ok(cost)
    }

    pub(crate) fn admit(self) -> Result<(), ResidentKernelError> {
        self.checked().map(drop)
    }

    fn permit(self) -> Result<ResidentBudgetPermit, ResidentKernelError> {
        Ok(ResidentBudgetPermit {
            _cost: self.checked()?,
        })
    }
}

impl<P> PreparedKernel<P> {
    pub(crate) const fn new(plan: P, cost: KernelCostEstimate) -> Self {
        Self { plan, cost }
    }

    pub(crate) fn admit(self) -> Result<AdmittedKernel<P>, ResidentKernelError> {
        Ok(AdmittedKernel {
            plan: self.plan,
            _permit: self.cost.permit()?,
        })
    }
}

impl<P> AdmittedKernel<P> {
    /// Consumes the admission authority immediately before materialization.
    pub(crate) fn into_plan(self) -> P {
        self.plan
    }
}

impl<P> PreparedMutationPlan<P> {
    pub(crate) fn new(
        operation: P,
        final_output: PublishedOutputFootprint,
        mut cost: KernelCostEstimate,
    ) -> Self {
        cost.output_elements = final_output.elements;
        cost.output_bytes = final_output.retained_bytes;
        cost.retained_nodes = cost.retained_nodes.max(final_output.retained_nodes);
        Self {
            operation,
            final_output,
            cost,
        }
    }

    pub(crate) fn admit(self) -> Result<AdmittedMutationPlan<P>, ResidentKernelError> {
        let _final_output = self.final_output;
        Ok(AdmittedMutationPlan {
            operation: self.operation,
            _permit: self.cost.permit()?,
        })
    }
}

impl<P> AdmittedMutationPlan<P> {
    /// Consumes the complete post-state admission immediately before staging.
    pub(crate) fn into_plan(self) -> P {
        self.operation
    }
}

impl ResidentBudgetMeter {
    fn charge(
        &mut self,
        update: impl FnOnce(&mut KernelCostEstimate) -> Result<(), ResidentKernelError>,
    ) -> Result<(), ResidentKernelError> {
        let mut next = self.accumulated;
        update(&mut next)?;
        next.checked()?;
        self.accumulated = next;
        Ok(())
    }

    pub(crate) fn charge_comparison_work(
        &mut self,
        amount: u64,
    ) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.comparison_work = cost
                .comparison_work
                .checked_add(amount)
                .ok_or(ResidentKernelError::InvalidShape)?;
            cost.compute_work = cost
                .compute_work
                .checked_add(amount)
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(())
        })
    }

    pub(crate) fn charge_temporary_bytes(
        &mut self,
        amount: u64,
    ) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.temporary_bytes = cost
                .temporary_bytes
                .checked_add(amount)
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(())
        })
    }

    pub(crate) fn charge_cloned_bytes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.cloned_bytes = cost
                .cloned_bytes
                .checked_add(amount)
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(())
        })
    }

    pub(crate) fn charge_retained_nodes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.retained_nodes = cost
                .retained_nodes
                .checked_add(amount)
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(())
        })
    }

    pub(crate) fn charge_selector_bytes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge(|cost| {
            cost.selector_bytes = cost
                .selector_bytes
                .checked_add(amount)
                .ok_or(ResidentKernelError::InvalidShape)?;
            Ok(())
        })
    }

    pub(crate) fn estimate(self) -> KernelCostEstimate {
        self.accumulated
    }
}

pub(crate) fn checked_u64<T>(value: T) -> Result<u64, ResidentKernelError>
where
    T: TryInto<u64>,
{
    value
        .try_into()
        .map_err(|_| ResidentKernelError::InvalidShape)
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

pub(crate) fn checked_cost_product(values: &[u64]) -> Result<u64, ResidentKernelError> {
    values.iter().try_fold(1u64, |product, value| {
        product
            .checked_mul(*value)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

pub(crate) fn checked_cost_sum(values: &[u64]) -> Result<u64, ResidentKernelError> {
    values.iter().try_fold(0u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or(ResidentKernelError::InvalidShape)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_budget_fails_closed_on_limits_and_arithmetic() {
        assert!(KernelCostEstimate::default().admit().is_ok());
        assert_eq!(
            KernelCostEstimate {
                cloned_bytes: MAX_RESIDENT_CLONED_BYTES + 1,
                ..KernelCostEstimate::default()
            }
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape
        );
        assert_eq!(
            checked_product(&[usize::MAX, 2]),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            checked_sum(&[usize::MAX, 1]),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            checked_cost_product(&[u64::MAX, 2]),
            Err(ResidentKernelError::InvalidShape)
        );
        assert_eq!(
            checked_cost_sum(&[u64::MAX, 1]),
            Err(ResidentKernelError::InvalidShape)
        );
    }

    #[test]
    fn permit_requires_one_complete_peak_estimate() {
        let prepared = PreparedKernel::new(
            41_u64,
            KernelCostEstimate {
                temporary_bytes: MAX_RESIDENT_TEMPORARY_BYTES - 2,
                selector_bytes: 1,
                index_bytes: 1,
                ..KernelCostEstimate::default()
            },
        );
        assert_eq!(prepared.admit().unwrap().into_plan(), 41);
        assert_eq!(
            PreparedKernel::new(
                (),
                KernelCostEstimate {
                    temporary_bytes: MAX_RESIDENT_TEMPORARY_BYTES - 1,
                    selector_bytes: 1,
                    index_bytes: 1,
                    ..KernelCostEstimate::default()
                },
            )
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape
        );
        assert_eq!(
            PreparedKernel::new(
                (),
                KernelCostEstimate {
                    temporary_bytes: u64::MAX,
                    selector_bytes: 1,
                    ..KernelCostEstimate::default()
                },
            )
            .admit()
            .unwrap_err(),
            ResidentKernelError::InvalidShape
        );
    }

    #[test]
    fn incremental_meter_rejects_the_first_over_limit_charge() {
        let mut meter = ResidentBudgetMeter::default();
        meter
            .charge_comparison_work(MAX_RESIDENT_COMPARISON_WORK)
            .unwrap();
        assert_eq!(
            meter.charge_comparison_work(1),
            Err(ResidentKernelError::InvalidShape),
        );
        assert_eq!(
            meter.estimate().comparison_work,
            MAX_RESIDENT_COMPARISON_WORK,
        );
    }
}
