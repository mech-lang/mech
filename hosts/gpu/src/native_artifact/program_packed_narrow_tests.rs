use super::*;

#[test]
fn narrow_payload_selection_uses_proved_domain_for_every_supported_width() {
    for (domain, expected) in [
        (boundary_domain(b"a"), 16),
        (boundary_domain(b"abc"), 16),
        (boundary_domain(b"abcdefgh"), 16),
        (boundary_domain(b"abcdefghi"), 24),
        (boundary_domain(b"ABDH"), 24),
        (Domain::ascii(), 24),
    ] {
        assert_eq!(slot_bytes(domain), expected);
        for width in [0, 1, 9, 16, 17, 18] {
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let work = new_work(&memory);
            let mut map =
                PackedMap::new_encoded(work.clone(), limits(), width, body(), 0, domain).unwrap();
            assert_eq!(map.slots.is_narrow(), expected == 16);
            assert_eq!(map.backing.capacity_bytes(), (expected as u64 + 1) * 8 + 7);
            assert_eq!(map.frame.slots, map.slots.address());
            let first = map.slots.entry_address(0);
            assert_eq!(map.slots.entry_address(1) - first, expected as u64);
            let key = vec![b'a'; width as usize];
            // ASCII fallback includes this character even for the sparse
            // source alphabet. The narrow cases all explicitly contain it.
            put(&mut map, &key, u64::MAX);
            assert_eq!(begin_key(&mut map, &key), Some(u64::MAX));
            put(&mut map, &key, 0);
            assert_eq!(begin_key(&mut map, &key), Some(0));
            let (published, updates, _) = map.finish().unwrap();
            assert_eq!(updates, 2);
            assert_eq!(
                published.entries().unwrap().collect::<Vec<_>>(),
                vec![(std::str::from_utf8(&key).unwrap(), 0)]
            );
            drop((published, work));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn narrow_initial_backing_obeys_exact_and_one_short_byte_limits() {
    let domain = boundary_domain(b"abcdefgh");
    for short in [1, 0] {
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = new_work(&memory);
        let before = (memory.used_bytes(), memory.retained_nodes());
        let metadata = (size_of::<PackedMap>() + size_of::<Frame>()) as u64;
        let exact = before.0 + metadata + table_bytes(domain, 8).unwrap();
        let mut caps = limits();
        caps.max_temporary_bytes = Some(exact - short);
        let result = PackedMap::new_encoded(work.clone(), caps, 1, body(), 0, domain);
        if short == 1 {
            assert!(matches!(
                result,
                Err(Error::Limit {
                    field: "temporary bytes",
                    ..
                })
            ));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), before);
        } else {
            let map = result.unwrap();
            assert_eq!(memory.used_bytes(), exact);
            assert_eq!(map.backing.capacity_bytes(), 143);
            drop(map);
        }
        drop(work);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn narrow_growth_admits_simultaneous_old_and_new_byte_and_node_claims() {
    let domain = boundary_domain(b"abcdefgh");
    for (dimension, short) in [("bytes", 1u64), ("bytes", 0), ("nodes", 1), ("nodes", 0)] {
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let metadata = (size_of::<PackedMap>() + size_of::<Frame>()) as u64;
        let base_nodes = metadata + 4;
        let view = if dimension == "nodes" {
            memory.with_limits(u64::MAX, base_nodes + 17 - short)
        } else {
            memory.clone()
        };
        let work = new_work(&view);
        let work_bytes = memory.used_bytes();
        let mut caps = limits();
        // This owner now also retains the optional complete-key pointer
        // cache. Include its actual geometry in both exact peak boundaries;
        // the one-byte-short case must still refuse before table mutation.
        let cache_bytes = lookup_cache_entries(domain, 1).unwrap() as u64 * 8;
        if dimension == "bytes" {
            caps.max_temporary_bytes = Some(
                work_bytes
                    + metadata
                    + cache_bytes
                    + table_bytes(domain, 8).unwrap()
                    + table_bytes(domain, 16).unwrap()
                    - short,
            );
        }
        let mut map = PackedMap::new_encoded(work.clone(), caps, 1, body(), 0, domain).unwrap();
        assert_eq!(
            map.lookup_cache.as_ref().unwrap().len() as u64 * 8,
            cache_bytes
        );
        for (index, &key) in b"abcdefg".iter().enumerate() {
            put(
                &mut map,
                &[key],
                if index == 0 { u64::MAX } else { index as u64 },
            );
        }
        assert_eq!(begin_key(&mut map, b"h"), None);
        let old = (
            table(&map),
            map.frame.slots,
            map.frame.controls,
            map.frame.capacity,
            map.frame.len,
            map.frame.updates,
            map.frame.entry_pointer,
        );
        let usage = (memory.used_bytes(), memory.retained_nodes());
        let (low, high) = encoded(b"h", domain);
        let result = map.insert_missing(low, high, 0);
        if short == 1 {
            if dimension == "bytes" {
                assert!(matches!(
                    result,
                    Err(Error::Limit {
                        field: "temporary bytes",
                        ..
                    })
                ));
            } else {
                assert!(matches!(result, Err(Error::Memory(_))));
            }
            assert_eq!(
                (
                    table(&map),
                    map.frame.slots,
                    map.frame.controls,
                    map.frame.capacity,
                    map.frame.len,
                    map.frame.updates,
                    map.frame.entry_pointer
                ),
                old
            );
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), usage);
        } else {
            result.unwrap();
            assert_eq!(
                (map.frame.capacity, map.frame.len, map.updates()),
                (16, 8, 8)
            );
            assert_ne!(map.frame.slots, old.1);
            assert_eq!((map.frame.entry_pointer - map.frame.slots) % 16, 0);
            assert!(map.frame.entry_pointer < map.frame.slots + 16 * 16);
            assert_eq!(
                memory.used_bytes(),
                work_bytes + metadata + cache_bytes + table_bytes(domain, 16).unwrap()
            );
            assert_eq!(memory.retained_nodes(), base_nodes + 8);
            assert_eq!(begin_key(&mut map, b"a"), Some(u64::MAX));
            assert_eq!(begin_key(&mut map, b"h"), Some(0));
            assert_eq!(&map.controls[..7], &map.controls[16..]);
        }
        drop((map, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

#[test]
fn narrow_collision_groups_wrap_grow_and_compare_complete_keys() {
    let domain = boundary_domain(b"abcdefgh");
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let mut map = PackedMap::new_encoded(work.clone(), limits(), 18, body(), 0, domain).unwrap();
    let mut candidates: [Vec<Vec<u8>>; 128] = std::array::from_fn(|_| Vec::new());
    for mut index in 0..65536u64 {
        let mut key = vec![b'a'; 18];
        for byte in key.iter_mut().rev() {
            *byte = b'a' + (index & 7) as u8;
            index >>= 3;
        }
        let (low, high) = encoded(&key, domain);
        assert_eq!(high, 0);
        let mixed = hash(low, high);
        if mixed & 15 == 15 {
            candidates[usize::from(fingerprint(mixed))].push(key);
        }
    }
    let keys = candidates
        .into_iter()
        .find(|keys| keys.len() >= 10)
        .expect("bounded collision fixture");
    let mut expected = BTreeMap::new();
    for (index, key) in keys[..9].iter().enumerate() {
        let count = if index == 0 { u64::MAX } else { index as u64 };
        put(&mut map, key, count);
        expected.insert(std::str::from_utf8(key).unwrap().to_owned(), count);
    }
    assert_eq!(
        (map.frame.capacity, map.frame.len, map.updates()),
        (16, 9, 9)
    );
    assert_eq!(&map.controls[..7], &map.controls[16..]);
    for (key, value) in &expected {
        assert_eq!(begin_key(&mut map, key.as_bytes()), Some(*value));
    }
    assert_eq!(begin_key(&mut map, &keys[9]), None);
    for capacity in [8, 16, 32] {
        let full = Slots::new(domain, capacity).unwrap();
        let controls = vec![fingerprint(hash(1, 0)); capacity + GROUP - 1];
        assert!(PackedMap::probe(&full, &controls, 1, 0).is_err());
    }
    let (published, updates, _) = map.finish().unwrap();
    assert_eq!(updates, 9);
    assert_eq!(
        published
            .entries()
            .unwrap()
            .map(|(key, value)| (key.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
        expected
    );
    drop((published, work));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn wide_fallback_preserves_high_key_words_and_narrow_rejects_high_word_forgery() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    let work = new_work(&memory);
    let mut map = PackedMap::new(work.clone(), limits(), 18, body(), 0).unwrap();
    let first = vec![b'a'; 18];
    let mut second = first.clone();
    second[0] = b'b';
    let (a, b) = (packed(&first), packed(&second));
    assert_eq!(a.0, b.0);
    assert_ne!(a.1, b.1);
    put(&mut map, &first, u64::MAX);
    put(&mut map, &second, 0);
    assert_eq!(begin_key(&mut map, &first), Some(u64::MAX));
    assert_eq!(begin_key(&mut map, &second), Some(0));
    let (published, updates, _) = map.finish().unwrap();
    assert_eq!((published.len(), updates), (2, 2));
    drop(published);
    let domain = boundary_domain(b"abc");
    let mut map = PackedMap::new_encoded(work.clone(), limits(), 18, body(), 0, domain).unwrap();
    assert_eq!(begin_key(&mut map, &first), None);
    let low = map.frame.key_low;
    map.frame.key_high = 1;
    let before = (
        table(&map),
        map.updates(),
        memory.used_bytes(),
        memory.retained_nodes(),
        work.used(),
    );
    assert!(map.insert_missing(low, 1, 8).is_err());
    assert_eq!(
        (
            table(&map),
            map.updates(),
            memory.used_bytes(),
            memory.retained_nodes(),
            work.used()
        ),
        before
    );
    drop((map, work));
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}
