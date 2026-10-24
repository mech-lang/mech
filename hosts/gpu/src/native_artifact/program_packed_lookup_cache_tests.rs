use super::*;

#[test]
fn direct_lookup_cache_bounds_the_complete_key_range() {
    for alphabet in [&b"ab"[..], &b"ACGT"[..], &b"abcdefg"[..], &b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"[..]] {
        let domain = boundary_domain(alphabet).packing();
        for width in 0..=18 {
            let bits = width * u64::from(domain.bits());
            assert_eq!(lookup_cache_entries(domain, width),
                (bits > 0 && bits <= 12).then(|| 1usize << bits));
        }
    }
    assert_eq!(lookup_cache_entries(Domain::ascii(), 1), Some(128));
    assert_eq!(lookup_cache_entries(Domain::ascii(), 2), None);
}

#[test]
fn direct_lookup_cache_refreshes_growth_and_keeps_zero_and_max_values() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let domain = boundary_domain(b"ACGT");
    let mut map = PackedMap::new_encoded(work.clone(), limits(), 3, body(), 0, domain).unwrap();
    assert!(map.uses_lookup_cache());
    assert_eq!(map.lookup_cache.as_ref().unwrap().len(), 64);
    assert_eq!(map.frame.lookup_cache, map.lookup_cache.as_ref().unwrap().address());
    let mut keys = Vec::new();
    for rank in 0..64usize {
        let key = [b"ACGT"[rank / 16], b"ACGT"[(rank / 4) % 4], b"ACGT"[rank % 4]];
        let value = if rank % 2 == 0 { 0 } else { u64::MAX };
        put(&mut map, &key, value);
        keys.push((key, value));
        for &(key, expected) in &keys {
            let (low, high) = encoded(&key, domain);
            let at = PackedMap::probe(&map.slots, &map.controls, low, high).unwrap();
            assert_ne!(map.controls[at], EMPTY);
            assert_eq!(map.lookup_cache.as_ref().unwrap().get(low), map.slots.entry_address(at));
            assert_eq!(map.slots.get(at).value, expected);
        }
    }
    assert!(map.frame.capacity > 64, "original hash-table growth still runs");
    let (published, updates, traversals) = map.finish().unwrap();
    assert_eq!((updates, traversals), (64, 64));
    assert_eq!(published.entries().unwrap().count(), 64);
    drop(published);
    drop(work);
    assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
}

#[test]
fn direct_lookup_cache_declines_unavailable_auxiliary_space_and_foreign_pointers() {
    let domain = boundary_domain(b"ACGT");
    let measure = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let measured = PackedMap::new_encoded(new_work(&measure), limits(), 6, body(), 0, domain).unwrap();
    assert!(measured.uses_lookup_cache());
    let minimum = measure.used_bytes() - 4096 * 8;
    drop(measured);
    assert_eq!(measure.used_bytes(), 0);
    let memory = ManagedMemoryBudget::new(minimum);
    let mut restricted = limits();
    restricted.max_temporary_bytes = Some(minimum);
    let mut map = PackedMap::new_encoded(new_work(&memory), restricted, 6, body(), 0, domain).unwrap();
    assert!(!map.uses_lookup_cache());
    assert_eq!(map.frame.lookup_cache, 0);
    map.validate_frame().unwrap();
    map.frame.lookup_cache = 8;
    assert!(map.validate_frame().is_err());
    map.frame.lookup_cache = 0;
    drop(map);
    assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
}

#[test]
fn direct_lookup_cache_preserves_the_complete_frozen_map() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let domain = boundary_domain(b"ab");
    let mut map = PackedMap::new_encoded(work.clone(), limits(), 1, body(), 0, domain).unwrap();
    put(&mut map, b"a", 0);
    put(&mut map, b"b", u64::MAX);
    let frozen = map.freeze().unwrap();
    assert_eq!(frozen.lookup("a").unwrap(), Some(0));
    assert_eq!(frozen.lookup("b").unwrap(), Some(u64::MAX));
    assert_eq!(frozen.lookup("c").unwrap(), None);
    assert_eq!(frozen.lookup("é").unwrap(), None);
    drop(frozen);
    drop(work);
    assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
}
