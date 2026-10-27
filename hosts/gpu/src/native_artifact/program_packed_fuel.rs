//! Prepare a common full-batch quote before entering the worker escrow lock.
//! The original division path remains authoritative near limits/overflow.
use super::{BATCH, Error, Price, add, mul};

pub(super) struct Prepared {
    unit: Price,
    full_threshold: Option<Price>,
}
impl Prepared {
    pub(super) fn full_debit(&self) -> Option<Price> {
        self.full_threshold.map(|full| Price {
            compute: full.compute / 4,
            comparison: full.comparison / 4,
        })
    }
    pub(super) fn new(unit: Price) -> Self {
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

    pub(super) fn select(
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
mod tests {
    use super::*;

    fn original(
        price: Price,
        remaining: (u64, u64),
        funded: u64,
    ) -> Result<(u64, u64, (u64, u64)), Error> {
        let count = (remaining.0 / price.compute.max(1) / 4)
            .min(remaining.1 / price.comparison.max(1) / 4)
            .min(BATCH)
            .max(1);
        let funded = add(funded, count)?;
        Ok((
            mul(price.compute, count)?,
            mul(price.comparison, count)?,
            (count, funded),
        ))
    }

    fn compare(price: Price, remaining: (u64, u64), funded: u64) {
        let expected = original(price, remaining, funded);
        let actual = Prepared::new(price).select(remaining, funded);
        match (expected, actual) {
            (Ok(expected), Ok(actual)) => assert_eq!(actual, expected),
            (Err(Error::Overflow), Err(Error::Overflow)) => {}
            (expected, actual) => {
                panic!("different funding: expected={expected:?}, actual={actual:?}")
            }
        }
    }

    #[test]
    fn prepared_refill_matches_every_boundary_and_arithmetic_failure() {
        let prices = [
            0,
            1,
            3,
            256,
            1_000_003,
            u64::MAX / (4 * BATCH),
            u64::MAX / (4 * BATCH) + 1,
            u64::MAX / BATCH,
            u64::MAX,
        ];
        for compute in prices {
            for comparison in prices {
                let price = Price {
                    compute,
                    comparison,
                };
                let mut remaining = vec![0, 1, 3, 4, BATCH - 1, BATCH, u64::MAX];
                for unit in [compute.max(1), comparison.max(1)] {
                    for batches in [1, BATCH - 1, BATCH, BATCH + 1] {
                        if let Some(bound) =
                            unit.checked_mul(4).and_then(|n| n.checked_mul(batches))
                        {
                            remaining.extend([
                                bound.saturating_sub(1),
                                bound,
                                bound.saturating_add(1),
                            ]);
                        }
                    }
                }
                remaining.sort_unstable();
                remaining.dedup();
                for &a in &remaining {
                    for &b in &remaining {
                        for funded in [0, 1, u64::MAX - BATCH, u64::MAX - 1, u64::MAX] {
                            compare(price, (a, b), funded);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn prepared_refill_matches_arbitrary_finite_prices_and_allowances() {
        let mut random = 0xb34a2f9dc72185e1u64;
        let mut next = || {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            random
        };
        for _ in 0..20_000 {
            let price = Price {
                compute: next(),
                comparison: next(),
            };
            compare(price, (next(), next()), next());
            let price = Price {
                compute: next() >> 24,
                comparison: next() >> 24,
            };
            compare(price, (next(), next()), next());
        }
    }
}
