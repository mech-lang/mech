//! Shared pre-discard representability, not resource pricing or execution.
use super::{Error, add, invalid, mul};
use core::mem::size_of;
use mech_core::{
    Value, ValueData,
    snapshot::{
        CanonicalKeyValue, MapEntryDraft, MapEntryValue, ValueDataDraft, ValueFootprint,
        prospective_string_value_footprint,
    },
};

// The fixed proof reads/constructs one canonical ValueFootprint and fourteen
// scalar layout facts (A/P/parameters, wrapper/data, Map/Set, two encodings,
// capacity/producer, drafts/scratch/total). Fund their complete structural
// arithmetic, plus one canonical-owner bound and seven maintained scratch
// facts (count, host count, two index widths, frame width, product, fixed sum),
// with the existing 8*metadata_bytes+256 rule, no new coefficient.
pub(super) const CERTIFICATION_WORK: u64 =
    8 * (size_of::<ValueFootprint>() + size_of::<[u64; 22]>()) as u64 + 256;

/// A pre-discard representability certificate, NOT a reservation or a guessed
/// distinct count. Each successful append spends >=128 compute and >=its byte
/// length before discarding its yield. Consequently A<=W/128, P<=W and D<=A.
/// These are upper bounds only; no language shape is instantiated with them.
/// Public canonical layouts and the canonical String prospective calculator
/// cover every underlying fixed/packed/draft allocation and encoded length.
pub(super) fn certify(
    work: u64,
    parameters: usize,
    scratch: u64,
    owners: u64,
) -> Result<(), Error> {
    let count = work / 128;
    let payload = work;
    let matrix = prospective_string_value_footprint(parameters, true, count, payload)
        .map_err(|_| invalid("histogram policy has no String layout certificate"))?;
    let wrapper = add(size_of::<Value>() as u64, mul(parameters as u64, 8)?)?;
    let data = size_of::<ValueData>() as u64;
    // These are precisely encoding.rs's schema-directed Map/Set containers
    // plus their respective String/U64 child footprints, not serialized-size
    // substitutions for owned allocations.
    let map = add(
        add(wrapper, data)?,
        add(
            mul(
                count,
                add(size_of::<MapEntryValue>() as u64, mul(data, 2)?)?,
            )?,
            payload,
        )?,
    )?;
    let set = add(
        add(wrapper, data)?,
        add(
            mul(count, add(size_of::<CanonicalKeyValue>() as u64, data)?)?,
            payload,
        )?,
    )?;
    let encoded_map = add(add(mul(count, 16)?, payload)?, 8)?;
    let encoded_set = add(add(mul(count, 8)?, payload)?, 8)?;
    // A materialized builder's doubling policy has capacity <=max(4,2*A).
    // Include actual native/canonical draft and index element layouts. The
    // bound deliberately proves the sum representable, which implies every
    // individual allocation is <=isize::MAX, including empty-container slack.
    let capacity = mul(count, 2)?.max(4);
    let producer = add(mul(capacity, size_of::<ValueDataDraft>() as u64)?, payload)?;
    let drafts = add(
        mul(
            capacity,
            add(
                add(
                    size_of::<MapEntryDraft>() as u64,
                    mul(size_of::<ValueDataDraft>() as u64, 3)?,
                )?,
                add(mul(data, 2)?, size_of::<CanonicalKeyValue>() as u64)?,
            )?,
        )?,
        payload,
    )?;
    // Obtained from the selected maintained factory's exact scratch layout,
    // including private StringRadixFrame and fixed bucket/bitset arrays. Never
    // guess that private representation from an index-buffer-only formula.
    let total = [
        matrix.retained_bytes,
        matrix.encoded_bytes,
        map,
        set,
        encoded_map,
        encoded_set,
        producer,
        drafts,
        scratch,
        owners,
    ]
    .into_iter()
    .try_fold(0, add)?;
    if total > isize::MAX as u64 || usize::try_from(total).is_err() {
        return Err(invalid("histogram policy has no host layout certificate"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_certificate_is_finite_checked_and_never_assigns_distinct_count() {
        certify(0, 1, 0, 0).unwrap();
        if usize::BITS == 64 {
            certify(1_000_000_000, 1, 1_000_000_000, 1_048_576).unwrap();
        }
        assert!(certify(u64::MAX, 1, 0, 0).is_err());
        assert!(certify(128, usize::MAX, 0, 0).is_err());
        assert!(certify(0, 1, u64::MAX, 0).is_err());
        assert!(certify(0, 1, 0, u64::MAX).is_err());
    }
    #[test]
    fn policy_certificate_accepts_the_last_representable_bound_and_rejects_next() {
        let (mut low, mut high) = (0, u64::MAX);
        while high - low > 1 {
            let middle = low + (high - low) / 2;
            if certify(middle, 1, 0, 0).is_ok() {
                low = middle;
            } else {
                high = middle;
            }
        }
        certify(low, 1, 0, 0).unwrap();
        assert_eq!(high, low + 1);
        assert!(certify(high, 1, 0, 0).is_err());
    }
}
