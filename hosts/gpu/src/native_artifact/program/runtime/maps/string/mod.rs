//! Retained standard-library String/U64 map storage, not a counting algorithm.
//! Callers supply every value. No traversal, increment or source-site authority
//! lives here; integrating this owner into a native region requires a separate
//! ownership/representation proof.
use crate::native_artifact::program::runtime::maps::admission::{Error, Limits, add, check, mul};
use hashbrown::{DefaultHashBuilder, HashMap};
use mech_core::{ManagedMemoryReservation, ManagedNodeReservation};
use mech_engine::native_artifact::NativeLibraryWorkBudget;
use std::hash::BuildHasher;

mod borrowed_values;
mod publication;
use borrowed_values::BorrowedStringValues;
pub(in crate::native_artifact) use crate::native_artifact::program::runtime::maps::string::publication::{
    Geometry as PublicationGeometry,
    logical_map_bytes,
    logical_map_limits,
    string_u64_schema,
};
// hashbrown 0.15.5: power-of-two buckets, <=7/8 load, one control byte per
// bucket, a trailing control group and alignment. Use concrete entry width.
fn allocation_bound(capacity: usize) -> Result<u64, Error> {
    let buckets = capacity
        .checked_mul(8)
        .and_then(|n| n.checked_add(6))
        .map(|n| (n / 7).max(4))
        .and_then(usize::checked_next_power_of_two)
        .ok_or(Error::Overflow)?;
    let bytes = add(
        mul(buckets as u64, (size_of::<(Box<str>, u64)>() + 1) as u64)?,
        64,
    )?;
    if bytes > isize::MAX as u64 {
        return Err(Error::Overflow);
    }
    Ok(bytes)
}

pub(in crate::native_artifact) struct OwnedStringMap<S = DefaultHashBuilder> {
    // Actual allocations retire before their corresponding reservations.
    values: HashMap<Box<str>, u64, S>,
    borrowed: Option<Box<[BorrowedStringValues; 1]>>,
    table: ManagedMemoryReservation,
    keys: ManagedMemoryReservation,
    metadata: ManagedMemoryReservation,
    nodes: ManagedNodeReservation,
    work: NativeLibraryWorkBudget,
    limits: Limits,
}
impl OwnedStringMap {
    pub(in crate::native_artifact) fn try_with_borrowed_keys(
        source: &mech_core::Value,
        keys: &mech_core::Value,
        schemas: &mech_core::SchemaTable,
        work: NativeLibraryWorkBudget,
        limits: Limits,
        memory: mech_core::ManagedMemoryBudget,
    ) -> Result<Option<Self>, Error> {
        let mut owner = Self::new_with_memory(work.clone(), limits, memory.clone())?;
        let fixed = owner.retained_bytes()?;
        let remaining = Limits {
            entries: limits.entries,
            retained_bytes: limits
                .retained_bytes
                .checked_sub(fixed)
                .ok_or(Error::Overflow)?,
            temporary_bytes: limits
                .temporary_bytes
                .checked_sub(fixed)
                .ok_or(Error::Overflow)?,
        };
        match BorrowedStringValues::new_with_keys(
            source,
            keys,
            schemas,
            work.clone(),
            memory.clone(),
            remaining,
        ) {
            Ok(bank) => {
                // Keep an ordinary map's inline representation small. The bank's
                // metadata claim funds its boxed body; separately admit the
                // transient stack/heap coexistence before the fallible allocation.
                let bytes = size_of::<BorrowedStringValues>() as u64;
                check(
                    add(owner.retained_bytes()?, add(bank.retained_bytes()?, bytes)?)?,
                    limits.temporary_bytes,
                    "temporary bytes",
                )?;
                work.charge(add(mul(bytes, 8)?, 128)?, 0)?;
                let staging = memory.reserve_capacity(bytes)?;
                let mut one = Vec::new();
                one.try_reserve_exact(1).map_err(|_| Error::Allocation)?;
                if one.capacity() != 1 {
                    return Err(Error::Allocation);
                }
                one.push(bank);
                let bank: Box<[BorrowedStringValues; 1]> = one
                    .into_boxed_slice()
                    .try_into()
                    .map_err(|_| Error::Allocation)?;
                drop(staging);
                owner.borrowed = Some(bank);
                Ok(Some(owner))
            }
            // A different key set is an optional representation miss. It must
            // preserve the original seed and ordinary owned import.
            Err(Error::Binding(
                "borrowed dictionary key sets differ"
                | "borrowed dictionary ordered key sets differ",
            )) => Ok(None),
            Err(error) => Err(error),
        }
    }
    #[cfg(test)]
    pub(in crate::native_artifact) fn new(
        work: NativeLibraryWorkBudget,
        limits: Limits,
    ) -> Result<Self, Error> {
        Self::with_hasher_fn(work, limits, DefaultHashBuilder::default)
    }
    pub(in crate::native_artifact) fn new_with_memory(
        work: NativeLibraryWorkBudget,
        limits: Limits,
        memory: mech_core::ManagedMemoryBudget,
    ) -> Result<Self, Error> {
        #[cfg(test)]
        let hasher = tests::publication_hasher;
        #[cfg(not(test))]
        let hasher = DefaultHashBuilder::default;
        Self::with_hasher_memory_fn(work, limits, memory, hasher)
    }
}
impl<S: BuildHasher> OwnedStringMap<S> {
    #[cfg(test)]
    fn with_hasher(
        work: NativeLibraryWorkBudget,
        limits: Limits,
        hasher: S,
    ) -> Result<Self, Error> {
        Self::with_hasher_fn(work, limits, || hasher)
    }
    #[cfg(test)]
    fn with_hasher_fn(
        work: NativeLibraryWorkBudget,
        limits: Limits,
        hasher: impl FnOnce() -> S,
    ) -> Result<Self, Error> {
        let memory = work.memory_budget();
        Self::with_hasher_memory_fn(work, limits, memory, hasher)
    }
    fn with_hasher_memory_fn(
        work: NativeLibraryWorkBudget,
        limits: Limits,
        memory: mech_core::ManagedMemoryBudget,
        hasher: impl FnOnce() -> S,
    ) -> Result<Self, Error> {
        if memory != work.memory_budget() {
            return Err(Error::Binding("foreign map memory account"));
        }
        let admitted = work.memory_budget();
        let memory = admitted.with_limits(memory.limit_bytes(), memory.limit_nodes());
        let bytes = size_of::<Self>() as u64;
        check(bytes, limits.retained_bytes, "retained bytes")?;
        check(bytes, limits.temporary_bytes, "temporary bytes")?;
        work.charge(add(mul(bytes, 8)?, 64)?, 0)?;
        let nodes = memory.reserve_nodes(1)?;
        let metadata = memory.reserve_capacity(bytes)?;
        Ok(Self {
            values: HashMap::with_hasher(hasher()),
            borrowed: None,
            table: memory.reserve_capacity(0)?,
            keys: memory.reserve_capacity(0)?,
            metadata,
            nodes,
            work,
            limits,
        })
    }
    fn borrowed(&self) -> Option<&BorrowedStringValues> {
        self.borrowed.as_deref().map(|one| &one[0])
    }
    pub(in crate::native_artifact) fn len(&self) -> usize {
        self.borrowed()
            .map_or_else(|| self.values.len(), BorrowedStringValues::len)
    }
    #[cfg(test)]
    pub(in crate::native_artifact) fn capacity(&self) -> usize {
        self.values.capacity()
    }
    pub(in crate::native_artifact) fn key_bytes(&self) -> u64 {
        self.borrowed().map_or_else(
            || self.keys.capacity_bytes(),
            BorrowedStringValues::key_bytes,
        )
    }
    pub(in crate::native_artifact) fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
    }
    pub(in crate::native_artifact) fn retained_bytes(&self) -> Result<u64, Error> {
        let owned = add(
            add(self.table.capacity_bytes(), self.keys.capacity_bytes())?,
            self.metadata.capacity_bytes(),
        )?;
        add(
            owned,
            self.borrowed()
                .map_or(Ok(0), BorrowedStringValues::retained_bytes)?,
        )
    }
    pub(in crate::native_artifact) fn retained_nodes(&self) -> u64 {
        self.nodes.nodes()
            + self
                .borrowed()
                .map_or(0, BorrowedStringValues::retained_nodes)
    }
    pub(in crate::native_artifact) fn has_borrowed_keys(&self) -> bool {
        self.borrowed.is_some()
    }
    pub(in crate::native_artifact) fn replace_borrowed(
        &mut self,
        source: &mech_core::Value,
        index: usize,
        text: &str,
        value: u64,
    ) -> Result<u64, Error> {
        self.borrowed
            .as_mut()
            .ok_or(Error::Binding("ordinal write without borrowed bank"))?[0]
            .replace(source, index, text, value)
    }
    // Bound a full collision chain before lookup. Raw String equality can
    // inspect at most all stored key bytes plus fixed bucket/header work.
    pub(in crate::native_artifact) fn lookup_work(
        &self,
        key: &str,
        calls: u64,
    ) -> Result<(), Error> {
        let probes = add(mul(self.values.capacity() as u64, 4)?, 32)?;
        let bytes = add(add(self.keys.capacity_bytes(), key.len() as u64)?, probes)?;
        self.work
            .charge(mul(mul(bytes, 64)?, calls)?, mul(bytes, calls)?)?;
        Ok(())
    }
    pub(in crate::native_artifact) fn get(&self, key: &str) -> Result<Option<u64>, Error> {
        if self.borrowed.is_some() {
            return Err(Error::Binding("unproved lookup of borrowed dictionary"));
        }
        self.lookup_work(key, 1)?;
        Ok(self.values.get(key).copied())
    }
    /// Insert/replace a caller-provided value. In particular this contains no
    /// accumulator arithmetic, user function, or loop over input windows.
    pub(in crate::native_artifact) fn insert(
        &mut self,
        key: &str,
        value: u64,
    ) -> Result<Option<u64>, Error> {
        if self.borrowed.is_some() {
            return Err(Error::Binding("unproved write of borrowed dictionary"));
        }
        self.lookup_work(key, 2)?;
        if let Some(current) = self.values.get_mut(key) {
            return Ok(Some(core::mem::replace(current, value)));
        }
        let entries = add(self.values.len() as u64, 1)?;
        check(entries, self.limits.entries, "entries")?;
        let key_bytes = key.len() as u64;
        let retained = self.retained_bytes()?;
        let growth = self.values.len() == self.values.capacity();
        let target = if growth {
            self.values
                .capacity()
                .checked_mul(2)
                .map(|n| n.max(4))
                .ok_or(Error::Overflow)?
        } else {
            0
        };
        let table_bytes = if growth { allocation_bound(target)? } else { 0 };
        let next = add(
            add(
                retained - self.table.capacity_bytes(),
                if growth {
                    table_bytes
                } else {
                    self.table.capacity_bytes()
                },
            )?,
            key_bytes,
        )?;
        check(next, self.limits.retained_bytes, "retained bytes")?;
        // Keep old table, full replacement and both possible String buffers
        // simultaneously funded until every fallible allocation has succeeded.
        let peak = add(add(retained, table_bytes)?, mul(key_bytes, 3)?)?;
        check(peak, self.limits.temporary_bytes, "temporary bytes")?;
        let growth_work = if growth {
            add(
                add(
                    mul(table_bytes, 32)?,
                    mul(mul(target as u64, target as u64)?, 64)?,
                )?,
                mul(self.keys.capacity_bytes(), 64)?,
            )?
        } else {
            0
        };
        self.work
            .charge(add(growth_work, add(mul(key_bytes, 32)?, 128)?)?, 0)?;
        let memory = self.metadata.budget();
        let new_entry = memory.reserve_nodes(1)?;
        // Key construction and replacement table storage coexist with the
        // original map. Claims retire on every failure, never after allocating
        // first and checking a global census that another worker can change.
        let _staging = memory.reserve_nodes(add(
            2,
            if growth {
                add(self.values.len() as u64, 1)?
            } else {
                0
            },
        )?)?;
        let mut new_key = memory.reserve_capacity(mul(key_bytes, 3)?)?;
        let mut new_table = memory.reserve_capacity(table_bytes)?;
        // Key allocation happens before table growth, so allocator failure
        // cannot leave a successful resize behind a failed entry operation.
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(key.len())
            .map_err(|_| Error::Allocation)?;
        if bytes.capacity() as u64 > mul(key_bytes, 2)? {
            return Err(Error::Allocation);
        }
        bytes.extend_from_slice(key.as_bytes());
        let key = String::from_utf8(bytes)
            .map_err(|_| Error::Allocation)?
            .into_boxed_str();
        if growth {
            self.values
                .try_reserve(target - self.values.len())
                .map_err(|_| Error::Allocation)?;
            let actual = self.values.allocation_size() as u64;
            // This is a pinned library layout assertion, not permission to
            // allocate first and raise a limit after observing its size.
            assert!(
                actual <= table_bytes,
                "pinned hashbrown allocation exceeded its admitted layout"
            );
            new_table.resize_capacity(actual)?;
            self.table = new_table;
        }
        // Same-account split/merge arithmetic was checked before allocation.
        new_key.resize_capacity(key_bytes)?;
        self.keys.merge_capacity(new_key)?;
        self.nodes.merge(new_entry)?;
        Ok(self.values.insert(key, value))
    }
    pub(in crate::native_artifact) fn entries(
        &self,
    ) -> Result<impl ExactSizeIterator<Item = (&str, u64)>, Error> {
        if let Some(bank) = self.borrowed() {
            return Ok(Entries::Borrowed(bank.entries()?));
        }
        self.work.charge(
            add(
                mul(self.values.capacity() as u64, 32)?,
                mul(self.keys.capacity_bytes(), 8)?,
            )?,
            0,
        )?;
        Ok(Entries::Owned(
            self.values
                .iter()
                .map(|(key, value)| (key.as_ref(), *value)),
        ))
    }
}

// Both views borrow stable keys and allocate no iterator storage.
enum Entries<A, B> {
    Owned(A),
    Borrowed(B),
}
impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for Entries<A, B> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match self {
            Self::Owned(a) => a.next(),
            Self::Borrowed(b) => b.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Owned(a) => a.size_hint(),
            Self::Borrowed(b) => b.size_hint(),
        }
    }
}
impl<T, A: ExactSizeIterator<Item = T>, B: ExactSizeIterator<Item = T>> ExactSizeIterator
    for Entries<A, B>
{
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(in crate::native_artifact) use tests::with_fixed_publication_hash_seed;

pub(in crate::native_artifact) mod frozen;
