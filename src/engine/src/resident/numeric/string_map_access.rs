//! Shared maintained String-key planning and lookup. Borrowing a key does not
//! change canonical ordering, worst-case admission, or the missing-key error.
use mech_core::{ResidentKernelError, ValueData, snapshot::MapValue};

pub(in crate::resident) trait Budget {
    fn compute(&mut self, amount: u64) -> Result<(), ResidentKernelError>;
    fn comparison(&mut self, amount: u64) -> Result<(), ResidentKernelError>;
    fn nodes(&mut self, amount: u64) -> Result<(), ResidentKernelError>;
    fn selector_bytes(&mut self, amount: u64) -> Result<(), ResidentKernelError>;
    fn admit(&mut self) -> Result<(), ResidentKernelError>;
}

impl Budget for super::super::budget::ResidentBudgetMeter {
    fn compute(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge_compute_work(amount)
    }
    fn comparison(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge_comparison_work(amount)
    }
    fn nodes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge_retained_nodes(amount)
    }
    fn selector_bytes(&mut self, amount: u64) -> Result<(), ResidentKernelError> {
        self.charge_selector_bytes(amount)
    }
    fn admit(&mut self) -> Result<(), ResidentKernelError> {
        super::super::budget::PreparedKernel::new((), self.estimate())
            .admit()?
            .into_plan();
        Ok(())
    }
}

/// The ordinary scalar String selector and the non-Value selected projection
/// both use this exact candidate header charge. Snapshot selectors retain their
/// existing canonical-value footprint measurement before entering `find`.
pub(in crate::resident) fn candidate_work(
    selector: &str,
    meter: &mut impl Budget,
) -> Result<u64, ResidentKernelError> {
    let work = super::super::budget::checked_u64(selector.len())?
        .checked_add(8)
        .ok_or(ResidentKernelError::InvalidShape)?;
    meter.comparison(work)?;
    meter.nodes(1)?;
    Ok(work)
}

/// Admission precedes the first payload comparison, including absent and late
/// keys. All String maps share this same header scan and raw canonical search.
#[cfg(all(
    feature = "native-artifact",
    any(feature = "pure_text", feature = "map_ops")
))]
pub(in crate::resident) fn find(
    map: &MapValue,
    selector: &str,
    candidate_work: u64,
    meter: &mut impl Budget,
) -> Result<usize, ResidentKernelError> {
    find_with_compare(map, candidate_work, meter, |candidate| {
        let ValueData::String(candidate) = candidate else {
            return Err(ResidentKernelError::InvalidInput);
        };
        Ok(selector.cmp(candidate.as_ref()))
    })
}

pub(super) fn find_with_compare(
    map: &MapValue,
    candidate_work: u64,
    meter: &mut impl Budget,
    mut compare: impl FnMut(&ValueData) -> Result<core::cmp::Ordering, ResidentKernelError>,
) -> Result<usize, ResidentKernelError> {
    let mut largest_key_work = candidate_work;
    let mut key_bytes = 0u64;
    for entry in map.entries() {
        meter.compute(64)?;
        let ValueData::String(text) = entry.key().data() else {
            return Err(ResidentKernelError::InvalidInput);
        };
        let bytes = super::super::budget::checked_u64(text.len())?;
        largest_key_work = largest_key_work.max(
            bytes
                .checked_add(8)
                .ok_or(ResidentKernelError::InvalidShape)?,
        );
        key_bytes = key_bytes
            .checked_add(bytes)
            .ok_or(ResidentKernelError::InvalidShape)?;
        meter.nodes(1)?;
    }
    let comparisons = map
        .entries()
        .len()
        .checked_ilog2()
        .map_or(0, |height| u64::from(height) + 1);
    meter.comparison(
        largest_key_work
            .checked_mul(comparisons)
            .ok_or(ResidentKernelError::InvalidShape)?,
    )?;
    // Keep the maintained conservative selector-storage claim, even though
    // neither route copies canonical map keys to perform this binary search.
    meter.selector_bytes(key_bytes)?;
    meter.admit()?;
    let mut lower = 0usize;
    let mut upper = map.entries().len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        match compare(map.entries()[middle].key().data())? {
            core::cmp::Ordering::Less => upper = middle,
            core::cmp::Ordering::Greater => lower = middle + 1,
            core::cmp::Ordering::Equal => return Ok(middle),
        }
    }
    Err(ResidentKernelError::InvalidInput)
}
