//! Borrowed current-root sort facts; no snapshot or canonical draft is built.
use super::*;

impl ValueCell {
    /// Retains observation work for source planning; this is not an incremental
    /// active-meter denial. The real call is admitted before materialization.
    pub fn current_canonical_conversion_comparison_work(
        &self,
        target: &Schema,
        plan: &crate::ConversionPlan,
        payload: u64,
        phases: u64,
    ) -> MResult<Option<(u64, u64)>> {
        let storage = self.binding.storage()?;
        let Some(canonical) = storage
            .as_any()
            .downcast_ref::<ManagedCanonicalCellStorage>()
        else {
            return Ok(None);
        };
        let arena = canonical.value.schemas().ok_or_else(|| {
            MechError::new(crate::MemoryPlanError::UnsupportedStorageLayout, None)
                .with_compiler_loc()
        })?;
        let source = arena.get(canonical.value.schema()).ok_or_else(|| {
            MechError::new(crate::MemoryPlanError::UnsupportedStorageLayout, None)
                .with_compiler_loc()
        })?;
        let mut observed = 0u64;
        let comparisons = crate::conversion_comparison_work(
            source.body(),
            target.body(),
            &plan.step,
            canonical.value.data(),
            payload,
            phases,
            |units| {
                observed = observed.checked_add(units).ok_or(
                    crate::MemoryPlanError::ArithmeticOverflow {
                        field: "conversion comparison observation work",
                    },
                )?;
                Ok::<(), crate::MemoryPlanError>(())
            },
        )
        .map_err(|error| {
            let error = match error {
                crate::ConversionPopulationError::Observer(error) => error,
                crate::ConversionPopulationError::ArithmeticOverflow => {
                    crate::MemoryPlanError::ArithmeticOverflow {
                        field: "conversion comparison bound",
                    }
                }
                crate::ConversionPopulationError::InvalidValue => {
                    crate::MemoryPlanError::UnsupportedStorageLayout
                }
            };
            MechError::new(error, None).with_compiler_loc()
        })?;
        Ok(Some((comparisons, observed)))
    }
}
