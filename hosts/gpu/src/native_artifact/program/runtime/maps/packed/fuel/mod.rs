//! Prepare a common full-batch quote before entering the worker escrow lock.
//! The original division path remains authoritative near limits/overflow.
use crate::native_artifact::program::runtime::maps::packed::{BATCH, Price, add, mul};
use crate::native_artifact::program::runtime::maps::string::Error;

pub(in crate::native_artifact::program::runtime::maps::packed) struct Prepared {
    unit: Price,
    full_threshold: Option<Price>,
}
impl Prepared {
    pub(in crate::native_artifact::program::runtime::maps::packed) fn full_debit(
        &self,
    ) -> Option<Price> {
        self.full_threshold.map(|full| Price {
            compute: full.compute / 4,
            comparison: full.comparison / 4,
        })
    }
    pub(in crate::native_artifact::program::runtime::maps::packed) fn new(unit: Price) -> Self {
        // remaining / price / 4 >= BATCH exactly when remaining >=
        // price * (4 * BATCH), provided that product is representable.
        // Zero prices retain the original max(1) denominator path.
        let full_threshold = (unit.compute != 0 && unit.comparison != 0)
            .then(|| {
                unit.compute
                    .checked_mul(4 * BATCH)
                    .zip(unit.comparison.checked_mul(4 * BATCH))
            })
            .flatten()
            .map(|(compute, comparison)| Price {
                compute,
                comparison,
            });
        Self {
            unit,
            full_threshold,
        }
    }

    pub(in crate::native_artifact::program::runtime::maps::packed) fn select(
        &self,
        remaining: (u64, u64),
        funded: u64,
    ) -> Result<(u64, u64, (u64, u64)), Error> {
        if let Some(full) = self.full_threshold
            && remaining.0 >= full.compute
            && remaining.1 >= full.comparison
        {
            // Every threshold is an exact multiple of four. These shifts
            // recover precisely the original per-body price * BATCH debit.
            return Ok((
                full.compute / 4,
                full.comparison / 4,
                (BATCH, add(funded, BATCH)?),
            ));
        }
        // Preserve the old count, headroom, arithmetic-error order and minimum
        // one-iteration request, including a request the ledger may reject.
        let count = (remaining.0 / self.unit.compute.max(1) / 4)
            .min(remaining.1 / self.unit.comparison.max(1) / 4)
            .min(BATCH)
            .max(1);
        let funded = add(funded, count)?;
        Ok((
            mul(self.unit.compute, count)?,
            mul(self.unit.comparison, count)?,
            (count, funded),
        ))
    }
}

#[cfg(test)]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::runtime::maps::packed::fuel) mod tests;
