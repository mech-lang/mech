use crate::native_artifact::program::runtime::maps::string::*;
use mech_core::ManagedMemoryBudget;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::hash::{BuildHasher, Hasher};
use std::rc::Rc;

fn limits() -> Limits {
    Limits {
        entries: 4096,
        retained_bytes: 4 * 1024 * 1024,
        temporary_bytes: 8 * 1024 * 1024,
    }
}
fn work(memory: &ManagedMemoryBudget) -> NativeLibraryWorkBudget {
    NativeLibraryWorkBudget::new(1_000_000_000, 100_000_000, memory).unwrap()
}

#[test]
fn owned_map_keeps_arbitrary_values_unicode_and_empty_keys_and_releases_storage() {
    let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
    let work = work(&memory);
    let base = memory.used_bytes();
    let mut map = OwnedStringMap::new(work.clone(), limits()).unwrap();
    assert_eq!(map.capacity(), 0);
    assert_eq!(map.get("missing").unwrap(), None);
    for (key, value) in [("", 7), ("é\0🙂", u64::MAX), ("ordinary", 91)] {
        assert_eq!(map.insert(key, value).unwrap(), None);
        assert_eq!(map.get(key).unwrap(), Some(value));
    }
    // Values are caller supplied, including replacement by a smaller value.
    assert_eq!(map.insert("é\0🙂", 3).unwrap(), Some(u64::MAX));
    assert_eq!(map.insert("é\0🙂", 3).unwrap(), Some(3));
    assert_eq!(map.len(), 3);
    let actual = map
        .entries()
        .unwrap()
        .map(|(key, value)| (key.to_owned(), value))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual,
        BTreeMap::from([("".into(), 7), ("ordinary".into(), 91), ("é\0🙂".into(), 3)])
    );
    assert_eq!(memory.used_bytes() - base, map.retained_bytes().unwrap());
    assert!(work.used().0 > 0 && work.used().1 > 0);
    drop(map);
    assert_eq!(memory.used_bytes(), base);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[derive(Clone)]
struct Collisions(Rc<Cell<u64>>);
struct CollisionHasher(Rc<Cell<u64>>);
impl BuildHasher for Collisions {
    type Hasher = CollisionHasher;
    fn build_hasher(&self) -> Self::Hasher {
        CollisionHasher(self.0.clone())
    }
}
impl Hasher for CollisionHasher {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.set(self.0.get() + bytes.len() as u64);
    }
}

#[test]
fn owned_map_uses_library_hashing_under_complete_collisions_and_repeated_growth() {
    let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
    let work = work(&memory);
    let calls = Rc::new(Cell::new(0));
    let mut map = OwnedStringMap::with_hasher(work, limits(), Collisions(calls.clone())).unwrap();
    let mut previous = 0;
    let mut grows = 0;
    for index in 0..256 {
        let key = format!("prefix-{index:04}");
        assert_eq!(map.insert(&key, index * 97).unwrap(), None);
        if map.capacity() != previous {
            grows += 1;
            previous = map.capacity();
        }
        assert_eq!(map.get(&key).unwrap(), Some(index * 97));
    }
    assert!(grows >= 5);
    assert!(calls.get() > 0);
    assert_eq!(map.get("prefix-0255").unwrap(), Some(255 * 97));
    assert_eq!(map.get("not-present").unwrap(), None);
    assert_eq!(map.len(), 256);
}

#[test]
fn owned_map_denials_precede_content_capacity_or_key_allocation_and_retry_succeeds() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let work = work(&memory);
    let mut map = OwnedStringMap::new(work, limits()).unwrap();
    let base = memory.used_bytes();
    let pressure = memory
        .reserve_capacity(memory.limit_bytes() - base)
        .unwrap();
    assert!(matches!(map.insert("first", 77), Err(Error::Memory(_))));
    assert_eq!(map.len(), 0);
    assert_eq!(map.capacity(), 0);
    assert_eq!(memory.used_bytes(), memory.limit_bytes());
    drop(pressure);
    assert_eq!(memory.used_bytes(), base);
    assert_eq!(map.insert("first", 77).unwrap(), None);
    let stable = memory.used_bytes();
    let capacity = map.capacity();
    map.limits.temporary_bytes = map.retained_bytes().unwrap();
    assert!(matches!(
        map.insert("new", 99),
        Err(Error::Limit {
            field: "temporary bytes",
            ..
        })
    ));
    assert_eq!(map.get("first").unwrap(), Some(77));
    assert_eq!(map.get("new").unwrap(), None);
    assert_eq!(map.capacity(), capacity);
    assert_eq!(memory.used_bytes(), stable);
    map.limits = limits();
    assert_eq!(map.insert("new", 99).unwrap(), None);
}

#[test]
fn owned_map_checks_entry_and_work_caps_without_restricting_replacements() {
    let memory = ManagedMemoryBudget::new(1024 * 1024);
    let work = work(&memory);
    let mut small = limits();
    small.entries = 1;
    let mut map = OwnedStringMap::new(work, small).unwrap();
    map.insert("present", 8).unwrap();
    assert!(matches!(
        map.insert("other", 9),
        Err(Error::Limit {
            field: "entries",
            ..
        })
    ));
    assert_eq!(map.insert("present", 4).unwrap(), Some(8));
    assert_eq!(map.get("present").unwrap(), Some(4));
    let setup = size_of::<OwnedStringMap>() as u64 * 8 + 64;
    let tight = NativeLibraryWorkBudget::new(setup + 100, 100, &memory).unwrap();
    let mut empty = OwnedStringMap::new(tight, limits()).unwrap();
    assert!(matches!(empty.insert("anything", 1), Err(Error::Work(_))));
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.capacity(), 0);
}

#[test]
fn owned_map_replacement_peak_has_an_exact_and_one_short_boundary() {
    let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
    let mut map = OwnedStringMap::new(work(&memory), limits()).unwrap();
    // Reach an actual full library table, rather than guessing its bucket/load
    // policy or choosing a convenient fixed implementation capacity.
    let mut ordinal = 0;
    loop {
        map.insert(&format!("k{ordinal:03}"), ordinal).unwrap();
        ordinal += 1;
        if map.len() == map.capacity() {
            break;
        }
    }
    let key = "a new Unicode key: 猫";
    let target = map.capacity() * 2;
    let required =
        map.retained_bytes().unwrap() + allocation_bound(target).unwrap() + key.len() as u64 * 3;
    let old_len = map.len();
    let old_capacity = map.capacity();
    let old_bytes = memory.used_bytes();
    map.limits.temporary_bytes = required - 1;
    assert!(
        matches!(map.insert(key, 900), Err(Error::Limit { field: "temporary bytes", required: actual, limit }) if actual == required && limit == required - 1)
    );
    assert_eq!(
        (map.len(), map.capacity(), memory.used_bytes()),
        (old_len, old_capacity, old_bytes)
    );
    map.limits.temporary_bytes = required;
    assert_eq!(map.insert(key, 900).unwrap(), None);
    assert_eq!(map.get(key).unwrap(), Some(900));
    assert!(map.capacity() > old_capacity);
}

#[test]
fn owned_map_denies_work_before_hashing_and_cannot_refill_on_retry() {
    let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
    let calibration = work(&memory);
    let calls = Rc::new(Cell::new(0));
    let mut reference =
        OwnedStringMap::with_hasher(calibration.clone(), limits(), Collisions(calls)).unwrap();
    reference.insert("present", 301).unwrap();
    let setup = calibration.used();
    reference.get("present").unwrap();
    let completed = calibration.used();
    assert!(completed.0 > setup.0 && completed.1 > setup.1);
    for comparison_short in [false, true] {
        let cap = if comparison_short {
            (completed.0, completed.1 - 1)
        } else {
            (completed.0 - 1, completed.1)
        };
        let bounded = NativeLibraryWorkBudget::new(cap.0, cap.1, &memory).unwrap();
        let calls = Rc::new(Cell::new(0));
        let mut map =
            OwnedStringMap::with_hasher(bounded.clone(), limits(), Collisions(calls.clone()))
                .unwrap();
        map.insert("present", 301).unwrap();
        assert_eq!(bounded.used(), setup);
        let hashes = calls.get();
        let bytes = memory.used_bytes();
        for _ in 0..3 {
            assert!(matches!(map.get("present"), Err(Error::Work(_))));
            assert_eq!(calls.get(), hashes);
            assert_eq!(bounded.used(), setup);
            assert_eq!(memory.used_bytes(), bytes);
        }
        assert_eq!(map.values.values().copied().collect::<Vec<_>>(), [301]);
    }
}

#[test]
fn owned_map_capacity_bound_rejects_overflow_without_allocation() {
    assert!(matches!(allocation_bound(usize::MAX), Err(Error::Overflow)));
    assert!(matches!(
        allocation_bound(usize::MAX / 8),
        Err(Error::Overflow)
    ));
}

#[test]
fn owned_map_collision_work_covers_long_shared_prefixes() {
    let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
    let calls = Rc::new(Cell::new(0));
    let mut map = OwnedStringMap::with_hasher(work(&memory), limits(), Collisions(calls)).unwrap();
    let prefix = "é\0".repeat(1024);
    for index in 0..16 {
        let key = format!("{prefix}{index:04}");
        map.insert(&key, 1000 - index).unwrap();
    }
    assert_eq!(map.get(&format!("{prefix}0015")).unwrap(), Some(985));
    assert_eq!(map.get(&format!("{prefix}0016")).unwrap(), None);
    assert_eq!(map.len(), 16);
}

#[test]
fn owned_map_retained_key_limit_is_admitted_before_growth() {
    let memory = ManagedMemoryBudget::new(16 * 1024 * 1024);
    let mut map = OwnedStringMap::new(work(&memory), limits()).unwrap();
    let key = "long-key".repeat(4096);
    let next = map.retained_bytes().unwrap() + allocation_bound(4).unwrap() + key.len() as u64;
    let before = memory.used_bytes();
    map.limits.retained_bytes = next - 1;
    assert!(
        matches!(map.insert(&key, 18), Err(Error::Limit { field: "retained bytes", required, limit }) if required == next && limit == next - 1)
    );
    assert_eq!(
        (map.len(), map.capacity(), memory.used_bytes()),
        (0, 0, before)
    );
    map.limits.retained_bytes = next;
    assert_eq!(map.insert(&key, 18).unwrap(), None);
    assert_eq!(map.get(&key).unwrap(), Some(18));
}
