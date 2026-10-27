//! Optional bounded memo of complete-key lookups, backed by the ordinary map.
//! Every hash slot and source update remains. Growth refreshes all addresses.
use super::*;

// At most 32 KiB of auxiliary pointer storage. Eligibility follows the full
// source-derived packed key range, independent of observed keys or queries.
const MAX_BITS: u64 = 12;
pub(in crate::native_artifact) fn lookup_cache_entries(
    domain: Domain,
    width: u64,
) -> Option<usize> {
    let bits = width.checked_mul(u64::from(domain.bits()))?;
    (bits > 0 && bits <= MAX_BITS).then(|| 1usize << bits)
}

pub(super) struct LookupCache {
    entries: Vec<u64>,
    // The actual backing retires before its same-account reservation.
    _backing: ManagedMemoryReservation,
}
impl LookupCache {
    pub(super) fn address(&self) -> u64 {
        self.entries.as_ptr() as u64
    }
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(super) fn clear(&mut self) {
        self.entries.fill(0);
    }
    #[cfg(test)]
    pub(super) fn get(&self, key: u64) -> u64 {
        self.entries[key as usize]
    }
    pub(super) fn insert(&mut self, key: u64, address: u64) {
        // Constructors bound the complete encoding range. All inserted keys
        // have passed valid_key; a live allocation's entry pointer is nonzero.
        assert_ne!(address, 0);
        self.entries[key as usize] = address;
    }
}

impl PackedMap {
    pub(super) fn try_lookup_cache(&mut self) -> Result<(), Error> {
        let Some(count) = lookup_cache_entries(self.domain, self.width) else {
            return Ok(());
        };
        let bytes = mul(count as u64, 8)?;
        if add(self.work.memory_budget().used_bytes(), bytes)?
            > cap(self.limits.max_temporary_bytes)?
        {
            return Ok(());
        }
        // Optional acceleration cannot turn a refused auxiliary reservation
        // into a source failure. The complete original hash lookup remains.
        let Ok(backing) = self.metadata.budget().reserve_capacity(bytes) else {
            return Ok(());
        };
        let compute = add(mul(bytes, 32)?, 128)?;
        let admitted = self.work.with_unscoped_debit(|remaining| {
            if compute > remaining.0 / 4 {
                Ok::<_, Error>((0, 0, false))
            } else {
                Ok((compute, 0, true))
            }
        })?;
        if admitted != Some(true) {
            return Ok(());
        }
        let mut entries = Vec::new();
        if entries.try_reserve_exact(count).is_err() || entries.capacity() != count {
            return Ok(());
        }
        entries.resize(count, 0);
        let cache = LookupCache {
            entries,
            _backing: backing,
        };
        self.frame.lookup_cache = cache.address();
        self.lookup_cache = Some(cache);
        Ok(())
    }
}
