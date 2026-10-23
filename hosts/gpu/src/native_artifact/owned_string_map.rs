//! Retained standard-library String/U64 map storage, not a counting algorithm.
//! Callers supply every value. No traversal, increment or source-site authority
//! lives here; integrating this owner into a native region requires a separate
//! ownership/representation proof.
use hashbrown::{DefaultHashBuilder, HashMap};
use mech_core::{ManagedMemoryReservation, MemoryRuntimeError, ResidentKernelError};
use mech_engine::native_artifact::NativeLibraryWorkBudget;
use std::hash::BuildHasher;

#[derive(Clone, Copy, Debug)]
pub(super) struct Limits {
    pub entries: u64,
    pub retained_bytes: u64,
    pub temporary_bytes: u64,
}
#[derive(Debug)]
pub(super) enum Error {
    Memory(MemoryRuntimeError),
    Work(ResidentKernelError),
    Limit {
        required: u64,
        limit: u64,
        field: &'static str,
    },
    Overflow,
    Allocation,
    Publication(&'static str),
    Binding(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory(error) => write!(out, "owned map memory: {error:?}"),
            Self::Work(error) => write!(out, "owned map work: {error:?}"),
            Self::Limit {
                required,
                limit,
                field,
            } => write!(out, "owned map {field}: {required} exceeds {limit}"),
            Self::Overflow => out.write_str("owned map size arithmetic overflow"),
            Self::Allocation => out.write_str("owned map allocation failed"),
            Self::Publication(reason) => write!(out, "owned map publication: {reason}"),
            Self::Binding(reason) => write!(out, "owned map binding: {reason}"),
        }
    }
}

#[path = "owned_string_map_publication.rs"]
mod publication;
pub(super) use publication::Geometry as PublicationGeometry;
impl std::error::Error for Error {}
impl From<MemoryRuntimeError> for Error {
    fn from(error: MemoryRuntimeError) -> Self {
        Self::Memory(error)
    }
}
impl From<ResidentKernelError> for Error {
    fn from(error: ResidentKernelError) -> Self {
        Self::Work(error)
    }
}
fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::Overflow)
}
fn mul(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_mul(b).ok_or(Error::Overflow)
}
fn check(required: u64, limit: u64, field: &'static str) -> Result<(), Error> {
    if required > limit {
        Err(Error::Limit {
            required,
            limit,
            field,
        })
    } else {
        Ok(())
    }
}

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

pub(super) struct OwnedStringMap<S = DefaultHashBuilder> {
    // Actual allocations retire before their corresponding reservations.
    values: HashMap<Box<str>, u64, S>,
    table: ManagedMemoryReservation,
    keys: ManagedMemoryReservation,
    metadata: ManagedMemoryReservation,
    work: NativeLibraryWorkBudget,
    limits: Limits,
}
impl OwnedStringMap {
    pub(super) fn new(work: NativeLibraryWorkBudget, limits: Limits) -> Result<Self, Error> {
        Self::with_hasher_fn(work, limits, DefaultHashBuilder::default)
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
    fn with_hasher_fn(
        work: NativeLibraryWorkBudget,
        limits: Limits,
        hasher: impl FnOnce() -> S,
    ) -> Result<Self, Error> {
        let bytes = size_of::<Self>() as u64;
        check(bytes, limits.retained_bytes, "retained bytes")?;
        check(bytes, limits.temporary_bytes, "temporary bytes")?;
        work.charge(add(mul(bytes, 8)?, 64)?, 0)?;
        let memory = work.memory_budget();
        let metadata = memory.reserve_capacity(bytes)?;
        Ok(Self {
            values: HashMap::with_hasher(hasher()),
            table: memory.reserve_capacity(0)?,
            keys: memory.reserve_capacity(0)?,
            metadata,
            work,
            limits,
        })
    }
    pub(super) fn len(&self) -> usize {
        self.values.len()
    }
    pub(super) fn capacity(&self) -> usize {
        self.values.capacity()
    }
    pub(super) fn key_bytes(&self) -> u64 {
        self.keys.capacity_bytes()
    }
    pub(super) fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
    }
    pub(super) fn retained_bytes(&self) -> Result<u64, Error> {
        add(
            add(self.table.capacity_bytes(), self.keys.capacity_bytes())?,
            self.metadata.capacity_bytes(),
        )
    }
    // Bound a full collision chain before lookup. Raw String equality can
    // inspect at most all stored key bytes plus fixed bucket/header work.
    fn lookup_work(&self, key: &str, calls: u64) -> Result<(), Error> {
        let probes = add(mul(self.values.capacity() as u64, 4)?, 32)?;
        let bytes = add(add(self.keys.capacity_bytes(), key.len() as u64)?, probes)?;
        self.work
            .charge(mul(mul(bytes, 64)?, calls)?, mul(bytes, calls)?)?;
        Ok(())
    }
    pub(super) fn get(&self, key: &str) -> Result<Option<u64>, Error> {
        self.lookup_work(key, 1)?;
        Ok(self.values.get(key).copied())
    }
    /// Insert/replace a caller-provided value. In particular this contains no
    /// accumulator arithmetic, user function, or loop over input windows.
    pub(super) fn insert(&mut self, key: &str, value: u64) -> Result<Option<u64>, Error> {
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
        let memory = self.work.memory_budget();
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
        Ok(self.values.insert(key, value))
    }
    pub(super) fn entries(&self) -> Result<impl ExactSizeIterator<Item = (&str, u64)>, Error> {
        self.work.charge(
            add(
                mul(self.values.capacity() as u64, 32)?,
                mul(self.keys.capacity_bytes(), 8)?,
            )?,
            0,
        )?;
        Ok(self
            .values
            .iter()
            .map(|(key, value)| (key.as_ref(), *value)))
    }
}

#[cfg(test)]
#[path = "owned_string_map_tests.rs"]
mod tests;
