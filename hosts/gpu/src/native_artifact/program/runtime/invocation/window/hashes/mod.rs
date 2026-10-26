//! A standard-library-table accumulator, not a workload implementation.
//! Source-window iteration lives in the emitted object. Every callback looks
//! up and updates one count, including keys never used by a later query.
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
use hashbrown::HashMap;
use mech_core::{
    ManagedMemoryReservation, ManagedNodeReservation, MemoryBudgetDimension as Dimension,
};
use mech_engine::native_artifact::{NativeLibraryWorkBudget, NativeMatchOrigin};

pub(in crate::native_artifact::program::runtime::invocation::window) const MAX_WIDTH: u64 =
    (u128::BITS / 7) as u64;

pub(in crate::native_artifact::program::runtime) struct WindowHashes {
    // Drop actual owners before releasing the corresponding reservations.
    counts: HashMap<u128, u64>,
    backing: ManagedMemoryReservation,
    _metadata: ManagedMemoryReservation,
    entries: ManagedNodeReservation,
    metadata_nodes: ManagedNodeReservation,
    length: usize,
    width: u64,
    accepted: u64,
    updated: u64,
    prepaid_updates: u64,
    prepaid_capacity: u64,
    collection: u32,
    origin: NativeMatchOrigin,
    work: NativeLibraryWorkBudget,
    temporary_limit: u64,
    retained_limit: u64,
    error: Option<Error>,
}

fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b)
        .ok_or_else(|| invalid("hash window accounting overflow"))
}
fn mul(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_mul(b)
        .ok_or_else(|| invalid("hash window accounting overflow"))
}

// Bound hashbrown 0.15.5's power-of-two buckets, <=7/8 load, control group,
// and alignment. Check the actual allocation size after every fallible growth.
// The old table remains charged while the entire replacement is admitted.
fn allocation_bound(capacity: usize) -> Result<u64, Error> {
    let buckets = capacity
        .checked_mul(8)
        .and_then(|n| n.checked_add(6))
        .map(|n| (n / 7).max(4))
        .and_then(usize::checked_next_power_of_two)
        .ok_or_else(|| invalid("hash window capacity overflow"))?;
    add(
        mul(buckets as u64, (size_of::<(u128, u64)>() + 1) as u64)?,
        64,
    )
}

impl WindowHashes {
    pub(in crate::native_artifact::program::runtime::invocation::window) fn new(
        length: usize,
        width: u64,
        accepted: u64,
        collection: u32,
        origin: NativeMatchOrigin,
        work: NativeLibraryWorkBudget,
        temporary_limit: u64,
        retained_limit: u64,
    ) -> Result<Self, Error> {
        let expected = if width > 0 && width <= length as u64 {
            length as u64 - width + 1
        } else {
            0
        };
        if width > MAX_WIDTH || accepted != expected {
            return Err(invalid("hash window is outside its source proof"));
        }
        let metadata = size_of::<Self>() as u64;
        crate::native_artifact::program::runtime::collections::check(
            origin,
            Dimension::TemporaryBytes,
            metadata,
            Some(temporary_limit),
        )?;
        let nodes = add(metadata, 2)?;
        crate::native_artifact::program::runtime::collections::check(
            origin,
            Dimension::RetainedNodes,
            nodes,
            Some(retained_limit),
        )?;
        let memory = work.memory_budget();
        Ok(Self {
            counts: HashMap::new(),
            backing: memory.reserve_capacity(0)?,
            _metadata: memory.reserve_capacity(metadata)?,
            entries: memory.reserve_nodes(0)?,
            metadata_nodes: memory.reserve_nodes(nodes)?,
            length,
            width,
            accepted,
            updated: 0,
            prepaid_updates: 0,
            prepaid_capacity: 0,
            collection,
            origin,
            work,
            temporary_limit,
            retained_limit,
            error: None,
        })
    }

    fn charge(&self, compute: u64, comparison: u64) -> Result<(), Error> {
        self.work
            .charge(compute, comparison)
            .map_err(|error| Error::Collection {
                origin: self.origin,
                error,
            })
    }

    // Fund at most 1024 starts on the SAME account. If the table grows,
    // prepay the capacity delta for every remaining start BEFORE its lookup.
    // No renewal/refund, favourable-hash assumption or discounted final work.
    fn fund_updates(&mut self) -> Result<(), Error> {
        let capacity = self.counts.capacity() as u64;
        if self.prepaid_updates == 0 {
            let count = (self.accepted - self.updated).min(1024);
            let probes = add(mul(capacity, 4)?, 32)?;
            let compute = add(mul(probes, 64)?, mul(self.width, 32)?)?;
            self.charge(mul(count, compute)?, mul(count, probes)?)?;
            self.prepaid_updates = count;
            self.prepaid_capacity = capacity;
        } else if capacity > self.prepaid_capacity {
            let delta = mul(capacity - self.prepaid_capacity, 4)?;
            self.charge(
                mul(mul(self.prepaid_updates, delta)?, 64)?,
                mul(self.prepaid_updates, delta)?,
            )?;
            self.prepaid_capacity = capacity;
        }
        self.prepaid_updates -= 1;
        Ok(())
    }

    fn update(&mut self, index: u64, key: u128) -> Result<(), Error> {
        if self.error.is_some() || index != self.updated || index >= self.accepted {
            return Err(invalid(
                "hash window update outside sequential admitted starts",
            ));
        }
        let start = usize::try_from(index).map_err(|_| invalid("hash window start"))?;
        let _end = start
            .checked_add(self.width as usize)
            .filter(|end| *end <= self.length)
            .ok_or_else(|| invalid("hash window read outside admitted source"))?;
        if key >> (self.width * 7) != 0 {
            return Err(invalid("hash window packed key exceeds admitted width"));
        }
        self.fund_updates()?;
        if let Some(count) = self.counts.get_mut(&key) {
            *count = count
                .checked_add(1)
                .ok_or_else(|| invalid("hash window count overflow"))?;
        } else {
            let growth = self.counts.len() == self.counts.capacity();
            // The private table persists even for query-only consumers, which
            // never publish a canonical Map. Retain one claim per entry and
            // admit duplicate entries plus the replacement table on rehash.
            let rehash_nodes = if growth {
                add(self.counts.len() as u64, 1)?
            } else {
                0
            };
            crate::native_artifact::program::runtime::collections::check(
                self.origin,
                Dimension::RetainedNodes,
                add(
                    add(add(self.metadata_nodes.nodes(), self.entries.nodes())?, 1)?,
                    rehash_nodes,
                )?,
                Some(self.retained_limit),
            )?;
            let memory = self.work.memory_budget();
            let entry = memory.reserve_nodes(1)?;
            let rehash = memory.reserve_nodes(rehash_nodes)?;
            if growth {
                let target = self
                    .counts
                    .capacity()
                    .checked_mul(2)
                    .map(|n| n.max(4))
                    .ok_or_else(|| invalid("hash window growth overflow"))?;
                let bound = allocation_bound(target)?;
                let peak = add(
                    add(self.backing.capacity_bytes(), bound)?,
                    self._metadata.capacity_bytes(),
                )?;
                crate::native_artifact::program::runtime::collections::check(
                    self.origin,
                    Dimension::TemporaryBytes,
                    peak,
                    Some(self.temporary_limit),
                )?;
                self.charge(
                    add(
                        mul(bound, 32)?,
                        mul(mul(target as u64, target as u64)?, 64)?,
                    )?,
                    0,
                )?;
                let mut pending = self.work.memory_budget().reserve_capacity(bound)?;
                self.counts
                    .try_reserve(target - self.counts.len())
                    .map_err(|_| invalid("hash window library allocation failed"))?;
                let actual = self.counts.allocation_size() as u64;
                if actual > bound {
                    return Err(invalid("hash window library exceeded admitted allocation"));
                }
                pending.resize_capacity(actual)?;
                self.backing = pending;
            }
            drop(rehash);
            // The second lookup/insert and any complete bucket scan were also
            // included in the conservative prepayment above.
            self.counts.insert(key, 1);
            self.entries.merge(entry)?;
        }
        self.updated += 1;
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime::invocation::window) fn validate(
        &self,
        collection: u32,
        accepted: u64,
        completed: u64,
    ) -> Result<(), Error> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        if self.collection != collection
            || self.accepted != accepted
            || self.prepaid_updates != 0
            || self.updated != accepted
            || completed != accepted
        {
            return Err(invalid(
                "hash window did not perform every admitted table update",
            ));
        }
        Ok(())
    }

    pub(in crate::native_artifact::program::runtime::invocation::window) fn completed_updates(
        &self,
    ) -> u64 {
        self.updated
    }

    pub(in crate::native_artifact::program::runtime::invocation::window) fn count(
        &self,
        query: &[u8],
    ) -> Result<u64, Error> {
        self.charge(
            add(mul(self.counts.capacity() as u64, 64)?, 1024)?,
            add(mul(self.counts.capacity() as u64, 4)?, 32)?,
        )?;
        if query.len() as u64 != self.width {
            return Ok(0);
        }
        Ok(pack(query)
            .and_then(|key| self.counts.get(&key).copied())
            .unwrap_or(0))
    }

    pub(in crate::native_artifact::program::runtime) fn entries(
        &self,
    ) -> impl Iterator<Item = (u128, u64)> + '_ {
        self.counts.iter().map(|(key, count)| (*key, *count))
    }
}

fn pack(bytes: &[u8]) -> Option<u128> {
    if bytes.len() > MAX_WIDTH as usize {
        return None;
    }
    bytes.iter().try_fold(0u128, |key, byte| {
        byte.is_ascii().then_some((key << 7) | u128::from(*byte))
    })
}

pub(in crate::native_artifact::program::runtime::invocation::window) unsafe extern "C" fn update_batch(
    owner: *mut WindowHashes,
    start: u64,
    keys: *const u64,
    count: u64,
) -> u64 {
    // SAFETY: only our emitted code receives this owner/function pair. Its
    // descriptor-admitted stack slot contains count initialized low/high pairs,
    // remains live for the synchronous call, and does not alias the owner.
    let owner = unsafe { &mut *owner };
    if count == 0
        || count > 128
        || start != owner.updated
        || start
            .checked_add(count)
            .is_none_or(|end| end > owner.accepted)
    {
        owner.error = Some(invalid("hash window batch outside admitted starts"));
        return 0;
    }
    for offset in 0..count {
        // No Rust u128 alignment or native-endian pair-layout assumption.
        let low = unsafe { keys.add((offset * 2) as usize).read() };
        let high = unsafe { keys.add((offset * 2 + 1) as usize).read() };
        let key = (u128::from(high) << 64) | u128::from(low);
        if let Err(error) = owner.update(start + offset, key) {
            owner.error = Some(error);
            return 0;
        }
    }
    1
}

#[cfg(test)]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::runtime::invocation::window::hashes) mod tests;
