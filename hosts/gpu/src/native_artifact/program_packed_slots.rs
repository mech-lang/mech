//! Explicit payload layouts shared by runtime storage and generated probes.
//! The source-derived packing domain decides the layout for every width this
//! entry accepts; runtime input values never select a different payload ABI.
use super::{Domain, Error, MAX_WIDTH, Slot};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub(in crate::native_artifact) struct NarrowSlot {
    pub low: u64,
    pub value: u64,
}

pub(in crate::native_artifact) fn narrow(domain: Domain) -> bool {
    u64::from(domain.packing().bits()) * MAX_WIDTH <= u64::from(u64::BITS)
}
pub(in crate::native_artifact) fn slot_bytes(domain: Domain) -> usize {
    if narrow(domain) {
        size_of::<NarrowSlot>()
    } else {
        size_of::<Slot>()
    }
}
#[cfg(any(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact) fn value_offset(domain: Domain) -> usize {
    if narrow(domain) {
        core::mem::offset_of!(NarrowSlot, value)
    } else {
        core::mem::offset_of!(Slot, value)
    }
}

// One active vector and its backing allocation. The logical Slot returned by
// get/iter has a zero high word for narrow storage; no count bits are narrowed.
pub(super) enum Slots {
    Narrow(Vec<NarrowSlot>),
    Wide(Vec<Slot>),
}
impl Slots {
    pub fn new(domain: Domain, count: usize) -> Result<Self, Error> {
        fn zeroed<T: Copy + Default>(count: usize) -> Result<Vec<T>, Error> {
            let mut slots = Vec::new();
            slots
                .try_reserve_exact(count)
                .map_err(|_| Error::Allocation)?;
            if slots.capacity() != count {
                return Err(Error::Allocation);
            }
            slots.resize(count, T::default());
            Ok(slots)
        }
        if narrow(domain) {
            Ok(Self::Narrow(zeroed(count)?))
        } else {
            Ok(Self::Wide(zeroed(count)?))
        }
    }
    pub fn is_narrow(&self) -> bool {
        matches!(self, Self::Narrow(_))
    }
    pub fn len(&self) -> usize {
        match self {
            Self::Narrow(slots) => slots.len(),
            Self::Wide(slots) => slots.len(),
        }
    }
    pub fn address(&self) -> u64 {
        match self {
            Self::Narrow(slots) => slots.as_ptr() as u64,
            Self::Wide(slots) => slots.as_ptr() as u64,
        }
    }
    pub fn address_mut(&mut self) -> u64 {
        match self {
            Self::Narrow(slots) => slots.as_mut_ptr() as u64,
            Self::Wide(slots) => slots.as_mut_ptr() as u64,
        }
    }
    pub fn entry_address(&mut self, index: usize) -> u64 {
        match self {
            Self::Narrow(slots) => (&mut slots[index] as *mut NarrowSlot) as u64,
            Self::Wide(slots) => (&mut slots[index] as *mut Slot) as u64,
        }
    }
    pub fn get(&self, index: usize) -> Slot {
        match self {
            Self::Narrow(slots) => Slot {
                low: slots[index].low,
                high: 0,
                value: slots[index].value,
            },
            Self::Wide(slots) => slots[index],
        }
    }
    // Callers validate full packed-key membership before this nonfallible
    // commit. For a narrow domain that proof necessarily establishes high=0.
    pub fn put(&mut self, index: usize, slot: Slot) {
        match self {
            Self::Narrow(slots) => {
                debug_assert_eq!(slot.high, 0);
                slots[index] = NarrowSlot {
                    low: slot.low,
                    value: slot.value,
                };
            }
            Self::Wide(slots) => slots[index] = slot,
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = Slot> + '_ {
        (0..self.len()).map(|index| self.get(index))
    }
}
