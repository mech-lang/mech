use super::*;
use mech_core::ManagedMemoryBudget;
use std::collections::BTreeMap;

fn limits() -> MemoryBudgetLimits {
    MemoryBudgetLimits {
        max_output_elements: Some(1), max_output_bytes: Some(16 * 1024 * 1024),
        max_temporary_bytes: Some(32 * 1024 * 1024), max_cloned_bytes: Some(16 * 1024 * 1024),
        max_retained_nodes: Some(1_000_000), max_compute_work: Some(1_000_000_000_000),
        max_comparison_work: Some(1_000_000_000), ..Default::default()
    }
}
fn new_work(memory: &ManagedMemoryBudget) -> NativeLibraryWorkBudget {
    NativeLibraryWorkBudget::new(1_000_000_000_000_000, 1_000_000_000_000, memory).unwrap()
}
fn body() -> BodyPrice { BodyPrice { scalar_compute: 2048, range_compute: 512, reads: 2, writes: 1 } }
fn packed(key: &[u8]) -> (u64, u64) {
    let word = key.iter().fold(0u128, |word, byte| {
        assert!(byte.is_ascii());
        (word << 7) | u128::from(*byte)
    });
    (word as u64, (word >> 64) as u64)
}
fn encoded(key: &[u8], domain: Domain) -> (u64, u64) {
    let (encode, _) = domain.tables();
    let word = key.iter().fold(0u128, |word, &byte| {
        assert!(domain.contains(byte));
        (word << domain.bits()) | u128::from(encode[usize::from(byte)])
    });
    (word as u64, (word >> 64) as u64)
}
// Test model of the emitted operations. Caller-supplied values deliberately
// include replacement, zero and MAX; no accumulation is part of the owner API.
fn begin_key(map: &mut PackedMap, key: &[u8]) -> Option<u64> {
    assert_eq!(key.len() as u64, map.width());
    if map.frame.fuel == 0 { map.refill().unwrap(); }
    map.frame.fuel -= 1;
    map.check_call(6).unwrap();
    map.check_call(3).unwrap();
    let (low, high) = encoded(key, map.domain);
    map.frame.key_low = low;
    map.frame.key_high = high;
    map.frame.entry_pointer = 0;
    map.frame.entry_known = 0;
    let index = PackedMap::probe(&map.slots, low, high).unwrap();
    map.frame.entry_known = 1;
    if map.slots[index].occupied == 1 {
        map.frame.entry_pointer = (&mut map.slots[index] as *mut Slot) as u64;
        Some(map.slots[index].value)
    } else { None }
}
fn put(map: &mut PackedMap, key: &[u8], value: u64) {
    let previous = begin_key(map, key);
    map.check_call(5).unwrap();
    if previous.is_some() {
        let updates = map.frame.updates.checked_add(1).unwrap();
        // SAFETY: begin_key located a live entry in this exclusively owned
        // table; there has been no growth or callback since taking its address.
        let slot = unsafe { &mut *(map.frame.entry_pointer as *mut Slot) };
        slot.value = value;
        map.frame.updates = updates;
    } else {
        let (low, high) = encoded(key, map.domain);
        map.insert_missing(low, high, value).unwrap();
    }
}
fn table(map: &PackedMap) -> Vec<(u64, u64, u64, u64)> {
    map.slots.iter().map(|s| (s.low, s.high, s.value, s.occupied)).collect()
}

#[test]
fn packed_maps_share_private_claims_without_claiming_the_same_caller_twice() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let base = (size_of::<PackedMap>() + size_of::<Frame>()) as u64 + 3;
    let work = new_work(&memory.with_limits(u64::MAX, 2 * base + 2));
    let mut left = PackedMap::new(work.clone(), limits(), 1, body(), 64).unwrap();
    let mut right = PackedMap::new(work.clone(), limits(), 1, body(), 64).unwrap();
    // outside_nodes is descriptive local geometry, not a new shared claim.
    assert_eq!(memory.retained_nodes(), 2 * base);
    assert_eq!(memory.retained_snapshot_nodes(), 0);
    put(&mut left, b"a", 1);
    put(&mut right, b"b", 2);
    assert_eq!(memory.retained_nodes(), 2 * base + 2);
    assert_eq!(begin_key(&mut left, b"c"), None);
    let old = table(&left);
    let key = packed(b"c");
    assert!(matches!(left.insert_missing(key.0, key.1, 3), Err(Error::Memory(_))));
    assert_eq!(table(&left), old);
    assert_eq!((left.frame.len, left.updates()), (1, 1));
    assert_eq!(memory.retained_nodes(), 2 * base + 2);
    drop(right);
    left.insert_missing(key.0, key.1, 3).unwrap();
    assert_eq!(memory.retained_nodes(), base + 2);
    drop((left, work));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn packed_rehash_admits_simultaneous_old_and_new_nodes_before_table_mutation() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let base = (size_of::<PackedMap>() + size_of::<Frame>()) as u64 + 3;
    let work = new_work(&memory.with_limits(u64::MAX, base + 8));
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    for key in 0..7 { put(&mut map, &[key], key as u64); }
    let old = table(&map);
    let bytes = memory.used_bytes();
    assert_eq!(begin_key(&mut map, &[7]), None);
    let key = packed(&[7]);
    assert!(matches!(map.insert_missing(key.0, key.1, 7), Err(Error::Memory(_))));
    assert_eq!(table(&map), old);
    assert_eq!((map.frame.capacity, map.frame.len, map.updates()), (8, 7, 7));
    assert_eq!(memory.used_bytes(), bytes);
    assert_eq!(memory.retained_nodes(), base + 7);
    drop((map, work));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn packed_conversion_holds_both_owners_and_releases_all_claims_on_denial() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let base = (size_of::<PackedMap>() + size_of::<Frame>()) as u64 + 3;
    let work = new_work(&memory.with_limits(u64::MAX, base + 3));
    let work_bytes = memory.used_bytes();
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    put(&mut map, b"a", 1);
    put(&mut map, b"b", 2);
    assert!(matches!(map.finish(), Err(Error::Memory(_))));
    assert_eq!(memory.used_bytes(), work_bytes);
    assert_eq!(memory.retained_nodes(), 0);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn successful_packed_conversion_keeps_only_the_returned_owner_claim() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    for key in 0..16 { put(&mut map, &[key], key as u64); }
    let (output, _, _) = map.finish().unwrap();
    assert_eq!(memory.retained_nodes(), 17);
    assert_eq!(memory.retained_snapshot_nodes(), 0);
    drop((output, work));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn packed_finite_domains_publish_every_original_key_and_preserve_replacements() {
    for alphabet in [b"ACGT".as_slice(), b"xyz", b"\0a", b"\x01\x02\x04\x08"] {
        let mut bitmap = [0u64; 2];
        for &byte in alphabet { bitmap[usize::from(byte / 64)] |= 1 << (byte % 64); }
        let domain = Domain::from_alphabet(bitmap).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = new_work(&memory);
        let mut map = PackedMap::new_encoded(work.clone(), limits(), 4, body(), 0, domain).unwrap();
        let mut expected = BTreeMap::new();
        for i in 0..alphabet.len().pow(4) {
            let mut rank = i;
            let key: [u8; 4] = core::array::from_fn(|_| {
                let byte = alphabet[rank % alphabet.len()]; rank /= alphabet.len(); byte
            });
            put(&mut map, &key, i as u64);
            let replacement = if i % 2 == 0 { 0 } else { u64::MAX };
            put(&mut map, &key, replacement);
            expected.insert(std::str::from_utf8(&key).unwrap().to_string(), replacement);
        }
        let count = expected.len() as u64;
        let (published, updates, traversals) = map.finish().unwrap();
        assert_eq!((updates, traversals), (count * 2, count * 2));
        let actual = published.entries().unwrap().map(|(key, value)| (key.to_string(), value))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(actual, expected);
        drop(published); drop(work);
        assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
    }
}

#[test]
fn packed_layout_matches_generated_word_offsets() {
    assert_eq!(size_of::<Frame>(), FRAME_WORDS * 8);
    assert_eq!(size_of::<Slot>(), SLOT_WORDS * 8);
    assert_eq!(std::mem::offset_of!(Frame, slots), SLOTS * 8);
    assert_eq!(std::mem::offset_of!(Frame, capacity), CAPACITY * 8);
    assert_eq!(std::mem::offset_of!(Frame, len), LENGTH * 8);
    assert_eq!(std::mem::offset_of!(Frame, fuel), FUEL * 8);
    assert_eq!(std::mem::offset_of!(Frame, updates), UPDATES * 8);
    assert_eq!(std::mem::offset_of!(Frame, key_low), KEY_LOW * 8);
    assert_eq!(std::mem::offset_of!(Frame, key_high), KEY_HIGH * 8);
    assert_eq!(std::mem::offset_of!(Frame, entry_pointer), ENTRY_POINTER * 8);
    assert_eq!(std::mem::offset_of!(Frame, entry_known), ENTRY_KNOWN * 8);
    assert_eq!(std::mem::offset_of!(Frame, source_ptr), SOURCE_PTR * 8);
    assert_eq!(std::mem::offset_of!(Frame, source_len), SOURCE_LEN * 8);
    assert_eq!(std::mem::offset_of!(Frame, width), WIDTH * 8);
    assert_eq!(std::mem::offset_of!(Frame, previous_start), PREVIOUS_START * 8);
    assert_eq!(std::mem::offset_of!(Frame, low_mask), LOW_MASK * 8);
    assert_eq!(std::mem::offset_of!(Frame, high_mask), HIGH_MASK * 8);
    assert_eq!(std::mem::offset_of!(Frame, range_call_allowed), RANGE_CALL_ALLOWED * 8);
    assert_eq!(std::mem::offset_of!(Frame, read_call_allowed), READ_CALL_ALLOWED * 8);
    assert_eq!(std::mem::offset_of!(Frame, write_call_allowed), WRITE_CALL_ALLOWED * 8);
    assert_eq!(std::mem::offset_of!(Slot, low), SLOT_LOW * 8);
    assert_eq!(std::mem::offset_of!(Slot, high), SLOT_HIGH * 8);
    assert_eq!(std::mem::offset_of!(Slot, value), SLOT_VALUE * 8);
    assert_eq!(std::mem::offset_of!(Slot, occupied), SLOT_OCCUPIED * 8);
}

#[test]
fn packed_map_keeps_all_ascii_keys_values_growth_and_updates() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let base = memory.used_bytes();
    let mut map = PackedMap::new(work.clone(), limits(), 2, body(), 64).unwrap();
    let mut expected = BTreeMap::new();
    let mut updates = 0;
    for index in 0..256u64 {
        let key = [(index / 128) as u8, (index % 128) as u8];
        let value = if index == 0 { u64::MAX } else { index * 17 };
        put(&mut map, &key, value);
        expected.insert(String::from_utf8(key.to_vec()).unwrap(), value);
        updates += 1;
    }
    assert!(map.frame.capacity >= 512);
    for index in 0..256u64 {
        let key = [(index / 128) as u8, (index % 128) as u8];
        let value = if index % 2 == 0 { 0 } else { u64::MAX - index };
        put(&mut map, &key, value);
        expected.insert(String::from_utf8(key.to_vec()).unwrap(), value);
        updates += 1;
    }
    assert_eq!(map.updates(), updates);
    let (published, completed, traversals) = map.finish().unwrap();
    assert_eq!((completed, traversals), (updates, updates));
    let actual = published.entries().unwrap().map(|(k, v)| (k.to_owned(), v)).collect::<BTreeMap<_, _>>();
    assert_eq!(actual, expected);
    drop(published);
    assert_eq!(memory.used_bytes(), base);
    drop(work);
    assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
}

#[test]
fn packed_map_preserves_empty_nul_and_maximum_width_keys() {
    for key in [Vec::new(), vec![0], vec![0; 18], vec![127; 18], b"arbitrary ascii !!".to_vec()] {
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = new_work(&memory);
        let mut map = PackedMap::new(work.clone(), limits(), key.len() as u64, body(), 0).unwrap();
        put(&mut map, &key, u64::MAX);
        put(&mut map, &key, 0);
        put(&mut map, &key, 9);
        let (published, updates, traversals) = map.finish().unwrap();
        assert_eq!((updates, traversals), (3, 3));
        let actual = published.entries().unwrap().collect::<Vec<_>>();
        assert_eq!(actual, [(std::str::from_utf8(&key).unwrap(), 9)]);
        drop(published);
        drop(work);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn packed_map_refill_cannot_renew_or_bypass_root_call_or_foreign_limits() {
    for mode in 0..5 {
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = new_work(&memory);
        let mut policy = limits();
        if mode == 1 { policy.max_comparison_work = Some(0); }
        let mut map = PackedMap::new(work.clone(), policy, 1, body(), 0).unwrap();
        let used = work.used();
        match mode {
            0 => {
                let price = map.price(map.frame.capacity, 0).unwrap();
                work.restrict(used.0 + price.compute - 1, used.1 + price.comparison).unwrap();
                assert!(map.refill().is_err());
                assert_eq!(work.used(), used);
            }
            1 => {
                map.refill().unwrap();
                assert_eq!((map.frame.range_call_allowed, map.frame.read_call_allowed,
                    map.frame.write_call_allowed), (1, 0, 0));
                let funded = work.used();
                assert!(funded.0 > used.0 && funded.1 > used.1);
                assert!(map.check_call(6).is_ok());
                assert!(matches!(map.check_call(3), Err(Error::Work(
                    mech_core::ResidentKernelError::ResourceLimit {
                        dimension: mech_core::MemoryBudgetDimension::ComparisonWork, limit: 0, .. }))));
                assert_eq!(work.used(), funded);
            }
            2 => {
                map.refill().unwrap();
                let funded = map.funded();
                let fuel = map.remaining();
                let after = work.used();
                assert!(map.refill().is_err());
                assert_eq!((map.funded(), map.remaining(), work.used()), (funded, fuel, after));
            }
            3 => {
                let foreign = new_work(&memory);
                assert!(foreign.with_scope(|| map.refill()).unwrap().is_err());
                assert_eq!(work.used(), used);
            }
            _ => {
                let price = map.price(map.frame.capacity, 0).unwrap();
                work.restrict(used.0 + price.compute, used.1 + price.comparison - 1).unwrap();
                assert!(map.refill().is_err());
                assert_eq!(work.used(), used);
            }
        }
        drop(map);
        drop(work);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn packed_map_handles_deliberate_probe_collisions_and_bounds_full_tables() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let mut map = PackedMap::new(work.clone(), limits(), 2, body(), 0).unwrap();
    let keys = (0..16384u64).map(|index| [(index / 128) as u8, (index % 128) as u8])
        .filter(|key| { let (low, high) = packed(key); hash(low, high) & 7 == 0 })
        .take(7).collect::<Vec<_>>();
    assert_eq!(keys.len(), 7);
    for (index, key) in keys.iter().enumerate() { put(&mut map, key, index as u64); }
    assert_eq!((map.frame.capacity, map.frame.len), (8, 7));
    for (index, key) in keys.iter().enumerate() {
        let (low, high) = packed(key);
        let at = PackedMap::probe(&map.slots, low, high).unwrap();
        assert_eq!((map.slots[at].occupied, map.slots[at].value), (1, index as u64));
    }
    let full = vec![Slot { low: 0, high: 0, value: 0, occupied: 1 }; 8];
    assert!(PackedMap::probe(&full, 1, 0).is_err());
    drop(map);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn packed_map_refuses_unbounded_calls_invalid_width_and_update_wrap() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let base = memory.used_bytes();
    let before = work.used();
    assert!(PackedMap::new(work.clone(), limits(), MAX_WIDTH + 1, body(), 0).is_err());
    let mut unbounded = limits();
    unbounded.max_comparison_work = None;
    assert!(PackedMap::new(work.clone(), unbounded, 1, body(), 0).is_err());
    assert_eq!((memory.used_bytes(), work.used()), (base, before));
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    assert_eq!(begin_key(&mut map, b"a"), None);
    map.frame.updates = u64::MAX;
    let before = (table(&map), work.used(), memory.used_bytes());
    let (low, high) = packed(b"a");
    assert!(matches!(map.insert_missing(low, high, 1), Err(Error::Overflow)));
    assert_eq!((table(&map), work.used(), memory.used_bytes()), before);
    assert_eq!((map.frame.len, map.frame.updates), (0, u64::MAX));
    drop(map);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn packed_map_missing_insert_rejects_forgery_and_denial_without_mutation() {
    for mode in 0..6 {
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = new_work(&memory);
        let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
        for key in b"abcdefg" { put(&mut map, &[*key], 91); }
        assert_eq!((map.frame.len, map.frame.capacity), (7, 8));
        assert_eq!(begin_key(&mut map, b"h"), None);
        let (low, high) = packed(b"h");
        let used = work.used();
        match mode {
            0 => { map.frame.entry_known = 0; }
            1 => { map.frame.key_low = low + 1; }
            2 => { work.restrict(used.0, used.1).unwrap(); }
            3 => { map.limits.max_cloned_bytes = Some(0); }
            4 => {
                map.limits.max_temporary_bytes = Some(memory.used_bytes() + 16 * size_of::<Slot>() as u64 - 1);
            }
            _ => { map.limits.max_output_bytes = Some(0); }
        }
        let before = (table(&map), map.frame.slots, map.frame.capacity, map.frame.len,
            map.frame.updates, map.frame.fuel, memory.used_bytes());
        assert!(map.insert_missing(low, high, 0).is_err());
        let after = (table(&map), map.frame.slots, map.frame.capacity, map.frame.len,
            map.frame.updates, map.frame.fuel, memory.used_bytes());
        assert_eq!(after, before);
        drop(map);
        drop(work);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn packed_map_growth_refreshes_entry_and_prepays_remaining_body_delta() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    for key in b"abcdefg" { put(&mut map, &[*key], 0); }
    let old_slots = map.frame.slots;
    assert_eq!(begin_key(&mut map, b"h"), None);
    let old_price = map.price(map.frame.capacity, map.frame.len).unwrap();
    let next_price = map.price(16, 8).unwrap();
    let pending = map.remaining() + 1;
    let before = work.used();
    let (low, high) = packed(b"h");
    map.insert_missing(low, high, 27).unwrap();
    let after = work.used();
    assert_eq!((map.frame.capacity, map.frame.len, map.frame.updates), (16, 8, 8));
    assert_ne!(map.frame.slots, old_slots);
    assert!(map.frame.entry_pointer >= map.frame.slots);
    assert!(map.frame.entry_pointer < map.frame.slots + 16 * size_of::<Slot>() as u64);
    assert_eq!((map.frame.entry_pointer - map.frame.slots) % size_of::<Slot>() as u64, 0);
    assert!(after.0 - before.0 >= (next_price.compute - old_price.compute) * pending);
    assert!(after.1 - before.1 >= (next_price.comparison - old_price.comparison) * pending);
    let at = PackedMap::probe(&map.slots, low, high).unwrap();
    assert_eq!((map.slots[at].occupied, map.slots[at].value), (1, 27));
    drop(map);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn packed_map_call_flags_defer_untaken_calls_and_refresh_after_insertion() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let mut policy = limits();
    policy.max_compute_work = Some(0);
    let mut empty = PackedMap::new(work.clone(), policy, 1, body(), 0).unwrap();
    // No source library call has executed merely because the body is funded.
    empty.refill().unwrap();
    assert_eq!((empty.frame.range_call_allowed, empty.frame.read_call_allowed,
        empty.frame.write_call_allowed), (0, 0, 0));
    let before = work.used();
    assert!(matches!(empty.check_call(6), Err(Error::Work(
        mech_core::ResidentKernelError::ResourceLimit {
            dimension: mech_core::MemoryBudgetDimension::ComputeWork, required: 512, limit: 0, .. }))));
    assert_eq!(work.used(), before);
    drop(empty);

    // The actual fresh insertion fits its cap; the more expensive FUTURE
    // replacement must become disallowed without rejecting that insertion.
    let mut policy = limits();
    policy.max_comparison_work = Some(278); // old write246 + missing helper32
    let mut map = PackedMap::new(work.clone(), policy, 18, body(), 0).unwrap();
    assert_eq!((map.frame.read_call_allowed, map.frame.write_call_allowed), (1, 1));
    put(&mut map, &[b'a'; 18], 0);
    assert_eq!((map.frame.len, map.frame.updates), (1, 1));
    assert_eq!((map.frame.read_call_allowed, map.frame.write_call_allowed), (1, 0));
    let before = (table(&map), work.used());
    assert!(map.check_call(3).is_ok());
    assert!(matches!(map.check_call(5), Err(Error::Work(
        mech_core::ResidentKernelError::ResourceLimit {
            dimension: mech_core::MemoryBudgetDimension::ComparisonWork,
            required: 300, limit: 278, .. }))));
    assert_eq!((table(&map), work.used()), before);
    drop(map);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn packed_map_refill_preserves_compute_and_comparison_headroom() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 0).unwrap();
    let price = map.price(map.frame.capacity, map.frame.len).unwrap();
    let used = work.used();
    work.restrict(used.0 + price.compute * 16, used.1 + price.comparison * 8).unwrap();
    assert_eq!(map.refill().unwrap(), 2);
    assert_eq!(work.used(), (used.0 + price.compute * 2, used.1 + price.comparison * 2));
    drop(map);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn packed_map_new_entry_admits_the_prospective_replacement_geometry() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let mut map = PackedMap::new(work.clone(), limits(), 1, body(), 64).unwrap();
    assert_eq!(begin_key(&mut map, b"a"), None);
    // The former calculation admitted old len+3 plus the new output nodes,
    // missing the new retained node that the next replacement would observe.
    let former = map.outside_nodes + map.metadata.capacity_bytes() + 3 + 2 + 2;
    map.limits.max_retained_nodes = Some(former);
    let (low, high) = packed(b"a");
    assert!(matches!(map.insert_missing(low, high, 3),
        Err(Error::Limit { field: "retained nodes", required, limit })
            if required == former + 1 && limit == former));
    assert_eq!((map.frame.len, map.frame.updates), (0, 0));
    map.limits.max_retained_nodes = Some(former + 1);
    map.insert_missing(low, high, 3).unwrap();
    assert_eq!((map.frame.len, map.frame.updates), (1, 1));
    assert!(map.geometry(1).is_ok());
    drop(map);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}

#[test]
fn packed_map_whole_import_has_one_finite_call_allowance() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let base = memory.used_bytes();
    let mut measured = PackedMap::new(work.clone(), limits(), 2, body(), 0).unwrap();
    for key in 0..16u8 { put(&mut measured, &[0, key], u64::from(key)); }
    let before = work.used();
    let (output, _, _) = measured.finish().unwrap();
    let import_compute = work.used().0 - before.0;
    drop(output);
    assert_eq!(memory.used_bytes(), base);

    let mut limited = PackedMap::new(work.clone(), limits(), 2, body(), 0).unwrap();
    for key in 0..16u8 { put(&mut limited, &[0, key], u64::from(key)); }
    limited.limits.max_compute_work = Some(import_compute - 1);
    assert!(matches!(limited.finish(), Err(Error::Work(
        mech_core::ResidentKernelError::ResourceLimit {
            dimension: mech_core::MemoryBudgetDimension::ComputeWork, .. }))));
    assert_eq!(memory.used_bytes(), base);
    drop(work);
    assert_eq!(memory.used_bytes(), 0);
}
