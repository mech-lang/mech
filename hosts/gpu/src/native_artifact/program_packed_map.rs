//! Private fixed-width ASCII String/U64 storage for generated map operations.
//! Generated Mech code owns iteration, branches, lookup and replacement values.
//! Rust owns admission, distinct-key insertion, growth and final publication.
use super::owned_string_map::{Error, Limits, OwnedStringMap};
use super::program_key_domain::Domain;
use mech_core::{
    ManagedMemoryReservation, ManagedNodeReservation, MemoryBudgetLimits, Value, ValueData,
};
use mech_engine::native_artifact::NativeLibraryWorkBudget;

pub(super) const MAX_WIDTH: u64 = 18;
pub(super) const BATCH: u64 = 1024;
pub(super) const HASH_FIRST: u64 = 0xbf58476d1ce4e5b9;
pub(super) const HASH_SECOND: u64 = 0x94d049bb133111eb;
pub(super) const GROUP: usize = 8;
pub(super) const EMPTY: u8 = 0x80;

// Explicit ABI: generated code never reads hashbrown's private representation.
// Every field is a word; all pointers originate exclusively in this owner.
#[repr(C)]
pub(super) struct Frame {
    pub slots: u64,
    pub capacity: u64,
    pub len: u64,
    pub fuel: u64,
    pub updates: u64,
    pub key_low: u64,
    pub key_high: u64,
    pub entry_pointer: u64,
    pub entry_known: u64,
    pub source_ptr: u64,
    pub source_len: u64,
    pub width: u64,
    pub previous_start: u64,
    pub low_mask: u64,
    pub high_mask: u64,
    pub range_call_allowed: u64,
    pub read_call_allowed: u64,
    pub write_call_allowed: u64,
    pub controls: u64,
    pub lookup_cache: u64,
}
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const FRAME_WORDS: usize = 20;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const SLOTS: usize = 0;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const CAPACITY: usize = 1;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const LENGTH: usize = 2;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const FUEL: usize = 3;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const UPDATES: usize = 4;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const KEY_LOW: usize = 5;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const KEY_HIGH: usize = 6;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const ENTRY_POINTER: usize = 7;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const ENTRY_KNOWN: usize = 8;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const SOURCE_PTR: usize = 9;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const SOURCE_LEN: usize = 10;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const WIDTH: usize = 11;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const PREVIOUS_START: usize = 12;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const LOW_MASK: usize = 13;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const HIGH_MASK: usize = 14;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const RANGE_CALL_ALLOWED: usize = 15;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const READ_CALL_ALLOWED: usize = 16;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const WRITE_CALL_ALLOWED: usize = 17;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const CONTROLS: usize = 18;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const LOOKUP_CACHE: usize = 19;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Slot {
    pub low: u64,
    pub high: u64,
    pub value: u64,
}
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const SLOT_WORDS: usize = 3;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const SLOT_LOW: usize = 0;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const SLOT_HIGH: usize = 1;
#[cfg(any(test, feature = "canonical-native-program"))]
pub(super) const SLOT_VALUE: usize = 2;

#[path = "program_packed_slots.rs"]
mod payload;
#[cfg(test)]
pub(super) use payload::NarrowSlot;
use payload::Slots;
pub(super) use payload::{narrow, slot_bytes, value_offset};

#[path = "program_frozen_packed_map.rs"]
mod frozen;
pub(super) use frozen::FrozenPackedMap;

#[path = "program_packed_lookup_cache.rs"]
mod lookup;
use lookup::LookupCache;
pub(super) use lookup::lookup_cache_entries;

/// Counts bound one complete acyclic source-body traversal, including branches
/// not taken. These are descriptor-derived counts, never user-supplied fuel.
#[derive(Clone, Copy, Debug)]
pub(super) struct BodyPrice {
    pub scalar_compute: u64,
    pub range_compute: u64,
    pub reads: u64,
    pub writes: u64,
}
#[derive(Clone, Copy, Debug)]
struct Price {
    compute: u64,
    comparison: u64,
}

#[path = "program_packed_fuel.rs"]
mod fuel;

pub(super) struct PackedMap {
    // Actual allocations retire before their reservations. The boxed frame
    // stays put when the containing closed-loop owner is moved into its Box.
    slots: Slots,
    controls: Vec<u8>,
    lookup_cache: Option<LookupCache>,
    frame: Box<Frame>,
    backing: ManagedMemoryReservation,
    metadata: ManagedMemoryReservation,
    nodes: ManagedNodeReservation,
    work: NativeLibraryWorkBudget,
    limits: MemoryBudgetLimits,
    outside_nodes: u64,
    width: u64,
    domain: Domain,
    decode: [u8; 128],
    all_codes_valid: bool,
    body: BodyPrice,
    issued: u64,
    funded: u64,
}

fn add(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_add(b).ok_or(Error::Overflow)
}
fn mul(a: u64, b: u64) -> Result<u64, Error> {
    a.checked_mul(b).ok_or(Error::Overflow)
}
fn cap(limit: Option<u64>) -> Result<u64, Error> {
    limit.ok_or(Error::Binding("unbounded packed-map dimension"))
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
fn finite(limits: MemoryBudgetLimits) -> Result<(), Error> {
    for limit in [
        limits.max_output_elements,
        limits.max_output_bytes,
        limits.max_temporary_bytes,
        limits.max_cloned_bytes,
        limits.max_retained_nodes,
        limits.max_compute_work,
        limits.max_comparison_work,
    ] {
        cap(limit)?;
    }
    Ok(())
}

/// Emit this exact wrapping mix in generated lookup. The table also bounds
/// complete collisions; hashing quality confers no work-budget exemption.
pub(super) fn hash(low: u64, high: u64) -> u64 {
    // Fold a complete unsigned product, rather than serially mixing two
    // truncated products. Both key words participate, even in the wide layout.
    let product = u128::from(low ^ HASH_FIRST) * u128::from(high ^ HASH_SECOND);
    product as u64 ^ (product >> 64) as u64
}
fn fingerprint(hash: u64) -> u8 {
    (hash >> 57) as u8
}
fn table_bytes(domain: Domain, capacity: u64) -> Result<u64, Error> {
    add(
        mul(capacity, slot_bytes(domain) as u64)?,
        add(capacity, (GROUP - 1) as u64)?,
    )
}
fn empty_controls(capacity: usize) -> Result<Vec<u8>, Error> {
    let count = capacity.checked_add(GROUP - 1).ok_or(Error::Overflow)?;
    let mut controls = Vec::new();
    controls
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    if controls.capacity() != count {
        return Err(Error::Allocation);
    }
    controls.resize(count, EMPTY);
    Ok(controls)
}
fn set_control(controls: &mut [u8], capacity: usize, index: usize, byte: u8) {
    controls[index] = byte;
    // Any unaligned eight-byte load starting below capacity remains in bounds.
    if index < GROUP - 1 {
        controls[capacity + index] = byte;
    }
}

impl PackedMap {
    /// The caller proves an empty canonical seed, one fixed-width ASCII key
    /// projection, sole ownership and no observable intermediate map versions.
    #[cfg(test)]
    pub(super) fn new(
        work: NativeLibraryWorkBudget,
        limits: MemoryBudgetLimits,
        width: u64,
        body: BodyPrice,
        outside_nodes: u64,
    ) -> Result<Self, Error> {
        Self::new_encoded(work, limits, width, body, outside_nodes, Domain::ascii())
    }

    pub(super) fn new_encoded(
        work: NativeLibraryWorkBudget,
        limits: MemoryBudgetLimits,
        width: u64,
        body: BodyPrice,
        outside_nodes: u64,
        domain: Domain,
    ) -> Result<Self, Error> {
        finite(limits)?;
        if width > MAX_WIDTH || body.writes == 0 {
            return Err(Error::Binding("unsupported packed-map source proof"));
        }
        let bytes = (size_of::<Self>() + size_of::<Frame>()) as u64;
        check(
            add(work.memory_budget().used_bytes(), bytes)?,
            cap(limits.max_temporary_bytes)?,
            "temporary bytes",
        )?;
        check(
            add(outside_nodes, bytes)?,
            cap(limits.max_retained_nodes)?,
            "retained nodes",
        )?;
        work.charge(add(mul(bytes, 8)?, 128)?, 0)?;
        let memory = work.memory_budget().with_limits(
            cap(limits.max_temporary_bytes)?,
            cap(limits.max_retained_nodes)?,
        );
        // Only this owner's real private geometry is claimed. outside_nodes
        // describes caller state already covered by its own shared claims.
        let nodes = memory.reserve_nodes(add(bytes, 4)?)?;
        let metadata = memory.reserve_capacity(bytes)?;
        let backing = memory.reserve_capacity(0)?;
        let domain = domain.packing();
        let (_, decode) = domain.tables();
        // Derive this from the actual decoder, not alphabet cardinality: even
        // a noninjective bit window must not certify an unused symbol code.
        // The immutable proof fits this owner's size-funded metadata claim.
        let all_codes_valid = decode[..1usize << domain.bits()]
            .iter()
            .all(|&byte| byte != u8::MAX);
        let key_bits = width * u64::from(domain.bits());
        let mut owner = Self {
            slots: Slots::new(domain, 0)?,
            controls: Vec::new(),
            lookup_cache: None,
            frame: Box::new(Frame {
                slots: 0,
                capacity: 0,
                len: 0,
                fuel: 0,
                updates: 0,
                key_low: 0,
                key_high: 0,
                entry_pointer: 0,
                entry_known: 0,
                source_ptr: 0,
                source_len: 0,
                width,
                previous_start: u64::MAX,
                low_mask: if key_bits < 64 {
                    (1u64 << key_bits) - 1
                } else {
                    u64::MAX
                },
                high_mask: if key_bits > 64 {
                    (1u64 << (key_bits - 64)) - 1
                } else {
                    0
                },
                range_call_allowed: 0,
                read_call_allowed: 0,
                write_call_allowed: 0,
                controls: 0,
                lookup_cache: 0,
            }),
            backing,
            metadata,
            nodes,
            work,
            limits,
            outside_nodes,
            width,
            domain,
            decode,
            all_codes_valid,
            body,
            issued: 0,
            funded: 0,
        };
        // Even an empty table has a finite, generated-probe-compatible shape.
        owner.allocate_initial()?;
        owner.try_lookup_cache()?;
        let flags = owner.call_flags(owner.frame.capacity, 0)?;
        owner.set_call_flags(flags);
        Ok(owner)
    }

    pub(super) fn frame(&mut self) -> *mut Frame {
        &mut *self.frame
    }
    pub(super) fn calls_allowed(&self) -> bool {
        self.frame.range_call_allowed != 0
            && self.frame.read_call_allowed != 0
            && self.frame.write_call_allowed != 0
    }
    pub(super) fn width(&self) -> u64 {
        self.width
    }
    pub(super) fn updates(&self) -> u64 {
        self.frame.updates
    }
    pub(super) fn funded(&self) -> u64 {
        self.funded
    }
    pub(super) fn remaining(&self) -> u64 {
        self.frame.fuel
    }
    pub(super) fn uses_lookup_cache(&self) -> bool {
        self.lookup_cache.is_some()
    }

    fn validate_frame(&self) -> Result<(), Error> {
        if self.frame.slots != self.slots.address()
            || self.slots.is_narrow() != narrow(self.domain)
            || self.frame.controls != self.controls.as_ptr() as u64
            || self.frame.capacity != self.slots.len() as u64
            || !self.frame.capacity.is_power_of_two()
            || self.frame.capacity < 8
            || self.controls.len()
                != self
                    .slots
                    .len()
                    .checked_add(GROUP - 1)
                    .ok_or(Error::Overflow)?
            || self.controls[..GROUP - 1] != self.controls[self.slots.len()..]
            || self.frame.len > self.frame.capacity / 8 * 7
            || self.frame.fuel > self.issued
            || self.frame.entry_known > 1
            || self.frame.width != self.width
            || self.frame.lookup_cache != self.lookup_cache.as_ref().map_or(0, LookupCache::address)
        {
            return Err(Error::Binding("invalid private packed-map frame"));
        }
        Ok(())
    }

    fn allocate_initial(&mut self) -> Result<(), Error> {
        let bytes = table_bytes(self.domain, GROUP as u64)?;
        check(
            add(self.work.memory_budget().used_bytes(), bytes)?,
            cap(self.limits.max_temporary_bytes)?,
            "temporary bytes",
        )?;
        self.work.charge(add(mul(bytes, 32)?, 128)?, 0)?;
        let pending = self.metadata.budget().reserve_capacity(bytes)?;
        let slots = Slots::new(self.domain, GROUP)?;
        let controls = empty_controls(GROUP)?;
        self.slots = slots;
        self.controls = controls;
        self.backing = pending;
        self.frame.slots = self.slots.address_mut();
        self.frame.capacity = 8;
        self.frame.controls = self.controls.as_mut_ptr() as u64;
        Ok(())
    }

    // Keep the original String-map full-chain accounting conservative even
    // though private packed comparisons inspect just two words. Key payload
    // growth and capacity growth both increase every remaining body's bound.
    fn read_price(&self, capacity: u64, count: u64) -> Result<Price, Error> {
        let bytes = add(
            add(mul(count, self.width)?, self.width)?,
            add(mul(capacity, 4)?, 32)?,
        )?;
        Ok(Price {
            compute: add(mul(bytes, 64)?, 256)?,
            comparison: bytes,
        })
    }
    fn write_price(&self, capacity: u64, count: u64) -> Result<Price, Error> {
        let read = self.read_price(capacity, count)?;
        Ok(Price {
            compute: add(mul(read.compute, 3)?, 512)?,
            comparison: mul(read.comparison, 3)?,
        })
    }
    fn price(&self, capacity: u64, count: u64) -> Result<Price, Error> {
        let read = self.read_price(capacity, count)?;
        let write = self.write_price(capacity, count)?;
        Ok(Price {
            compute: add(
                add(
                    add(self.body.scalar_compute, self.body.range_compute)?,
                    add(mul(self.width, 64)?, 512)?,
                )?,
                add(
                    mul(read.compute, self.body.reads)?,
                    mul(write.compute, self.body.writes)?,
                )?,
            )?,
            comparison: add(
                mul(read.comparison, self.body.reads)?,
                mul(write.comparison, self.body.writes)?,
            )?,
        })
    }

    fn call_flags(&self, capacity: u64, count: u64) -> Result<[u64; 3], Error> {
        let compute = cap(self.limits.max_compute_work)?;
        let comparison = cap(self.limits.max_comparison_work)?;
        let read = self.read_price(capacity, count)?;
        let write = self.write_price(capacity, count)?;
        Ok([
            u64::from(self.body.range_compute <= compute),
            u64::from(read.compute <= compute && read.comparison <= comparison),
            u64::from(write.compute <= compute && write.comparison <= comparison),
        ])
    }
    fn set_call_flags(&mut self, flags: [u64; 3]) {
        self.frame.range_call_allowed = flags[0];
        self.frame.read_call_allowed = flags[1];
        self.frame.write_call_allowed = flags[2];
    }
    fn check_call_price(&self, price: Price) -> Result<(), Error> {
        use mech_core::{
            MemoryBudgetDimension as Dimension, ResidentKernelError, ResidentResourcePhase,
        };
        for (required, limit, dimension) in [
            (
                price.compute,
                cap(self.limits.max_compute_work)?,
                Dimension::ComputeWork,
            ),
            (
                price.comparison,
                cap(self.limits.max_comparison_work)?,
                Dimension::ComparisonWork,
            ),
        ] {
            if required > limit {
                return Err(Error::Work(ResidentKernelError::ResourceLimit {
                    phase: ResidentResourcePhase::Admission,
                    dimension,
                    required,
                    limit,
                }));
            }
        }
        Ok(())
    }
    /// Invoked only at an actually executed source operation whose generated
    /// cap flag is false. Root work is already prepaid; this does not debit it
    /// again or reject a later operation on an untaken branch.
    pub(crate) fn check_call(&self, operation: u64) -> Result<(), Error> {
        self.validate_frame()?;
        let price = match operation {
            6 => Price {
                compute: self.body.range_compute,
                comparison: 0,
            },
            3 | 4 => self.read_price(self.frame.capacity, self.frame.len)?,
            5 => self.write_price(self.frame.capacity, self.frame.len)?,
            _ => return Err(Error::Binding("unknown packed-map source call")),
        };
        self.check_call_price(price)
    }

    /// No root renewal, refund or independent allowance. Generated code must
    /// decrement fuel once at the header, before any body instruction or probe.
    pub(super) fn refill(&mut self) -> Result<u64, Error> {
        self.validate_frame()?;
        if self.frame.fuel != 0 {
            return Err(Error::Binding("packed-map fuel was renewed"));
        }
        // Pricing is pure scalar arithmetic. Retain any error until the
        // original remaining-allowance check succeeds, without holding the
        // shared worker mutex while calculating the invariant price.
        let prepared = self
            .price(self.frame.capacity, self.frame.len)
            .map(fuel::Prepared::new);
        let full = prepared
            .as_ref()
            .ok()
            .and_then(fuel::Prepared::full_debit)
            .zip(self.funded.checked_add(BATCH));
        let credited = if let Some((price, funded)) = full {
            self.work
                .try_charge_funded_batch(price.compute, price.comparison)?
                .then_some((BATCH, funded))
        } else {
            None
        };
        let (count, funded) = if let Some(credited) = credited {
            credited
        } else {
            self.work
                .with_unscoped_debit(|remaining| prepared?.select(remaining, self.funded))?
                .ok_or(Error::Binding("packed-map entered a foreign work scope"))?
        };
        self.frame.fuel = count;
        self.issued = count;
        self.funded = funded;
        self.frame.entry_known = 0;
        self.frame.entry_pointer = 0;
        Ok(count)
    }

    fn valid_key(&self, low: u64, high: u64) -> bool {
        let mut key = (u128::from(high) << 64) | u128::from(low);
        let bits = self.domain.bits();
        if key >> (self.width * u64::from(bits)) != 0 {
            return false;
        }
        // Width remains checked for every key. With a complete decoder there
        // are no forbidden per-symbol codes left to inspect; sparse decoders
        // retain the original walk, including singleton/nonzero alphabets.
        if self.all_codes_valid {
            return true;
        }
        let mask = (1u128 << bits) - 1;
        for _ in 0..self.width {
            if self.decode[(key & mask) as usize] == u8::MAX {
                return false;
            }
            key >>= bits;
        }
        true
    }
    fn probe(slots: &Slots, controls: &[u8], low: u64, high: u64) -> Result<usize, Error> {
        Self::probe_debits(slots, controls, low, high, &mut |_, _| Ok(()))
    }
    fn probe_debits(
        slots: &Slots,
        controls: &[u8],
        low: u64,
        high: u64,
        charge: &mut impl FnMut(u64, u64) -> Result<(), mech_core::ResidentKernelError>,
    ) -> Result<usize, Error> {
        if slots.len() == 0 {
            return Err(Error::Binding("empty packed table"));
        }
        if slots.len() < GROUP
            || !slots.len().is_power_of_two()
            || controls.len() != slots.len().checked_add(GROUP - 1).ok_or(Error::Overflow)?
        {
            return Err(Error::Binding("invalid packed control shape"));
        }
        let hash = hash(low, high);
        let tag = fingerprint(hash);
        let group_mask = slots.len() - GROUP;
        let mut index = hash as usize & group_mask;
        // No deletion/tombstones: insertion uses the first empty lane in this
        // same linear sequence. A power-of-two capacity is a multiple of eight;
        // group-aligned starts visit each bucket exactly once without wrapping
        // within a group. Fingerprints never replace full-key equality.
        for _ in 0..slots.len() / GROUP {
            // Frozen operations pay for each actual bounded group. Existing
            // mutation callers already prepay their whole-body envelope.
            charge(1024, 16)?;
            let group = &controls[index..index + GROUP];
            for (lane, &control) in group.iter().enumerate() {
                if control == tag {
                    let at = index + lane;
                    let entry = slots.get(at);
                    if entry.low == low && entry.high == high {
                        return Ok(at);
                    }
                } else if control > EMPTY {
                    return Err(Error::Binding("invalid packed-map control"));
                }
            }
            if let Some(lane) = group.iter().position(|&control| control == EMPTY) {
                return Ok(index + lane);
            }
            index = (index + GROUP) & group_mask;
        }
        Err(Error::Binding("packed-map probe exhausted complete table"))
    }

    fn geometry(&self, count: u64) -> Result<(), Error> {
        let payload = mul(count, self.width)?;
        let output = add(
            (size_of::<Value>() + size_of::<ValueData>()) as u64,
            add(
                mul(
                    count,
                    (size_of::<mech_core::snapshot::MapEntryValue>() + 2 * size_of::<ValueData>())
                        as u64,
                )?,
                payload,
            )?,
        )?;
        check(1, cap(self.limits.max_output_elements)?, "output elements")?;
        check(output, cap(self.limits.max_output_bytes)?, "output bytes")?;
        // Retain the source insert's finite clone cap, although packed storage
        // does not construct a String until canonical publication.
        check(
            self.width,
            cap(self.limits.max_cloned_bytes)?,
            "cloned bytes",
        )?;
        // Admit the post-insert owner too: generated replacements do not run
        // another geometry check after the new entry becomes retained.
        let live_nodes = add(
            add(self.outside_nodes, self.metadata.capacity_bytes())?,
            add(count, 4)?,
        )?;
        check(
            add(live_nodes, add(mul(count, 2)?, 2)?)?,
            cap(self.limits.max_retained_nodes)?,
            "retained nodes",
        )?;
        check(
            self.work.memory_budget().used_bytes(),
            cap(self.limits.max_temporary_bytes)?,
            "temporary bytes",
        )?;
        Ok(())
    }

    /// Slow path only for a source insertion whose emitted lookup found no
    /// entry. The supplied value is already computed by the generated caller.
    /// A refusal cannot change the table, frame counters or cached entry.
    pub(super) fn insert_missing(&mut self, low: u64, high: u64, value: u64) -> Result<(), Error> {
        self.validate_frame()?;
        if self.issued == 0
            || self.frame.fuel >= self.issued
            || self.frame.entry_known != 1
            || self.frame.entry_pointer != 0
            || self.frame.key_low != low
            || self.frame.key_high != high
            || !self.valid_key(low, high)
        {
            return Err(Error::Binding(
                "packed-map insertion lacks current missing-key proof",
            ));
        }
        let count = add(self.frame.len, 1)?;
        let updates = add(self.frame.updates, 1)?;
        self.geometry(count)?;
        let old_capacity = self.frame.capacity;
        let growth = self.frame.len == old_capacity / 8 * 7;
        let capacity = if growth {
            mul(old_capacity, 2)?
        } else {
            old_capacity
        };
        let table_bytes = if growth {
            table_bytes(self.domain, capacity)?
        } else {
            0
        };
        if table_bytes > isize::MAX as u64 {
            return Err(Error::Overflow);
        }
        check(
            add(self.work.memory_budget().used_bytes(), table_bytes)?,
            cap(self.limits.max_temporary_bytes)?,
            "temporary bytes",
        )?;
        // A bounded full old-table scan and worst-case full-chain reinsertion
        // cover even adversarial collisions. This fee includes the extra
        // defensive lookup and repeat probe after growth.
        let probes = add(
            add(old_capacity, capacity)?,
            if growth {
                mul(old_capacity, capacity)?
            } else {
                0
            },
        )?;
        let recheck = if growth {
            mul(mul(old_capacity, self.width)?, 32)?
        } else {
            0
        };
        let cache_work = self.lookup_cache.as_ref().map_or(Ok(0), |cache| {
            if growth {
                add(mul(cache.len() as u64, 32)?, mul(capacity, 64)?)
            } else {
                Ok(32)
            }
        })?;
        let extra = Price {
            compute: add(
                add(
                    add(add(mul(probes, 128)?, recheck)?, mul(table_bytes, 32)?)?,
                    add(mul(self.width, 32)?, 256)?,
                )?,
                cache_work,
            )?,
            comparison: mul(probes, 2)?,
        };
        let write = self.write_price(old_capacity, self.frame.len)?;
        self.check_call_price(Price {
            compute: add(write.compute, extra.compute)?,
            comparison: add(write.comparison, extra.comparison)?,
        })?;
        let old = self.price(old_capacity, self.frame.len)?;
        let next = self.price(capacity, count)?;
        let flags = self.call_flags(capacity, count)?;
        // Include the in-progress traversal: its header already consumed one
        // fuel. This conservatively funds all its remaining source operations.
        let pending = add(self.frame.fuel, 1)?;
        let delta_compute = next
            .compute
            .checked_sub(old.compute)
            .ok_or(Error::Overflow)?;
        let delta_comparison = next
            .comparison
            .checked_sub(old.comparison)
            .ok_or(Error::Overflow)?;
        self.work.charge(
            add(extra.compute, mul(delta_compute, pending)?)?,
            add(extra.comparison, mul(delta_comparison, pending)?)?,
        )?;
        let old_index = Self::probe(&self.slots, &self.controls, low, high)?;
        if self.controls[old_index] != EMPTY {
            return Err(Error::Binding("forged packed-map missing-key proof"));
        }
        let memory = self.metadata.budget();
        let new_entry = memory.reserve_nodes(1)?;
        // A replacement table temporarily duplicates all occupied slots and
        // both slot/control containers. Admission precedes allocation and
        // remains until the old table retires; failures release pending claims.
        let _rehash = memory.reserve_nodes(if growth { add(self.frame.len, 2)? } else { 0 })?;
        let index;
        let mut replacement = None;
        if growth {
            let reservation = memory.reserve_capacity(table_bytes)?;
            let count = usize::try_from(capacity).map_err(|_| Error::Overflow)?;
            let mut slots = Slots::new(self.domain, count)?;
            let mut controls = empty_controls(count)?;
            for (old_index, slot) in self.slots.iter().enumerate() {
                let control = self.controls[old_index];
                if control == EMPTY {
                    continue;
                }
                if control != fingerprint(hash(slot.low, slot.high))
                    || !self.valid_key(slot.low, slot.high)
                {
                    return Err(Error::Binding("invalid retained packed-map entry"));
                }
                let at = Self::probe(&slots, &controls, slot.low, slot.high)?;
                if controls[at] != EMPTY {
                    return Err(Error::Binding("duplicate retained packed-map entry"));
                }
                slots.put(at, slot);
                set_control(&mut controls, count, at, control);
            }
            index = Self::probe(&slots, &controls, low, high)?;
            replacement = Some((slots, controls, reservation));
        } else {
            index = old_index;
        }
        self.nodes.merge(new_entry)?;
        // All fallible work completed before replacing the retained table.
        if let Some((slots, controls, reservation)) = replacement {
            self.slots = slots;
            self.controls = controls;
            self.backing = reservation;
            self.frame.slots = self.slots.address_mut();
            self.frame.controls = self.controls.as_mut_ptr() as u64;
            self.frame.capacity = capacity;
        }
        self.slots.put(index, Slot { low, high, value });
        set_control(
            &mut self.controls,
            self.slots.len(),
            index,
            fingerprint(hash(low, high)),
        );
        self.frame.len = count;
        self.frame.updates = updates;
        self.frame.entry_pointer = self.slots.entry_address(index);
        if let Some(cache) = &mut self.lookup_cache {
            if growth {
                cache.clear();
                for at in 0..self.slots.len() {
                    if self.controls[at] != EMPTY {
                        let low = self.slots.get(at).low;
                        let address = self.slots.entry_address(at);
                        cache.insert(low, address);
                    }
                }
            } else {
                cache.insert(low, self.frame.entry_pointer);
            }
        }
        self.set_call_flags(flags);
        Ok(())
    }

    /// Import every retained String/value into the existing publication owner.
    /// This computes no source update and omits no unqueried entry.
    pub(super) fn finish(self) -> Result<(OwnedStringMap, u64, u64), Error> {
        let work = self.work.clone();
        work.with_call_limits(
            cap(self.limits.max_compute_work)?,
            cap(self.limits.max_comparison_work)?,
            || self.finish_scoped(),
        )?
    }
    // One conversion boundary has one finite call allowance. Creating a fresh
    // allowance per imported entry would let a large table bypass that cap.
    fn finish_scoped(self) -> Result<(OwnedStringMap, u64, u64), Error> {
        self.validate_frame()?;
        let traversals = self
            .funded
            .checked_sub(self.frame.fuel)
            .ok_or(Error::Overflow)?;
        if self.frame.updates > mul(traversals, self.body.writes)? {
            return Err(Error::Binding(
                "packed-map updates exceed source write sites",
            ));
        }
        self.work.charge(
            add(
                mul(self.frame.capacity, 64)?,
                mul(self.frame.len, self.width * 32)?,
            )?,
            0,
        )?;
        let remaining = cap(self.limits.max_temporary_bytes)?
            .checked_sub(self.work.memory_budget().used_bytes())
            .ok_or(Error::Overflow)?;
        let mut output = OwnedStringMap::new_with_memory(
            self.work.clone(),
            Limits {
                entries: self.frame.len,
                retained_bytes: remaining,
                temporary_bytes: remaining,
            },
            self.metadata.budget(),
        )?;
        let mut count = 0;
        let bits = self.domain.bits();
        let mask = (1u128 << bits) - 1;
        for (index, slot) in self.slots.iter().enumerate() {
            let control = self.controls[index];
            if control == EMPTY {
                continue;
            }
            if control != fingerprint(hash(slot.low, slot.high))
                || !self.valid_key(slot.low, slot.high)
            {
                return Err(Error::Binding("invalid packed-map publication entry"));
            }
            let mut key = (u128::from(slot.high) << 64) | u128::from(slot.low);
            let mut bytes = [0u8; MAX_WIDTH as usize];
            for index in (0..self.width as usize).rev() {
                bytes[index] = self.decode[(key & mask) as usize];
                key >>= bits;
            }
            let text = std::str::from_utf8(&bytes[..self.width as usize])
                .map_err(|_| Error::Binding("non-ASCII packed-map publication"))?;
            output.insert(text, slot.value)?;
            count = add(count, 1)?;
        }
        if count != self.frame.len || output.len() as u64 != count {
            return Err(Error::Binding("packed-map publication count mismatch"));
        }
        Ok((output, self.frame.updates, traversals))
    }
}

#[cfg(test)]
#[path = "program_packed_map_tests.rs"]
mod tests;
