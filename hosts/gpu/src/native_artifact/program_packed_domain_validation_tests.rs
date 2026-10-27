use super::*;

fn original_validation(map: &PackedMap, mut key: u128) -> bool {
    let bits = map.domain.bits();
    if key >> (map.width * u64::from(bits)) != 0 {
        return false;
    }
    let mask = (1u128 << bits) - 1;
    for _ in 0..map.width {
        if map.decode[(key & mask) as usize] == u8::MAX {
            return false;
        }
        key >>= bits;
    }
    true
}

#[test]
fn complete_and_sparse_decoders_match_original_validation_at_every_width() {
    let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
    // Full ASCII, complete compact, sparse compact, singleton/nonzero and
    // noninjective fallback domains exercise distinct decoder inventories.
    let domains = [
        Domain::ascii(),
        boundary_domain(b"pqrs"),
        boundary_domain(b"abc"),
        boundary_domain(b"a"),
        boundary_domain(&[1, 2, 4, 8]),
        boundary_domain(b"ABCDEFGHI"),
    ];
    for domain in domains {
        for width in 0..=MAX_WIDTH {
            let work = new_work(&memory);
            let map =
                PackedMap::new_encoded(work.clone(), limits(), width, body(), 0, domain).unwrap();
            let bits = u64::from(map.domain.bits());
            let code_count = 1usize << bits;
            assert_eq!(
                map.all_codes_valid,
                map.decode[..code_count].iter().all(|&byte| byte != u8::MAX)
            );
            let used = bits * width;
            let mask = (1u128 << used) - 1;
            let mut cases = vec![0, mask, u128::MAX, 1u128 << used];
            // Each possible symbol in every position, including leading codes
            // and both sides of the 64-bit word boundary.
            for position in 0..width {
                for code in 0..code_count {
                    cases.push((code as u128) << (position * bits));
                }
            }
            let mut random = 0x1d8e4e27c47d124fu128;
            for _ in 0..128 {
                random ^= random << 13;
                random ^= random >> 7;
                random ^= random << 17;
                cases.push(random);
                cases.push(random & mask);
            }
            for key in cases {
                assert_eq!(
                    map.valid_key(key as u64, (key >> 64) as u64),
                    original_validation(&map, key),
                    "domain={:?}, width={width}, key={key:#034x}",
                    map.domain
                );
            }
            drop((map, work));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn complete_decoder_still_rejects_excess_key_bits_before_any_mutation() {
    for domain in [Domain::ascii(), boundary_domain(b"pqrs")] {
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let work = new_work(&memory);
        let mut map = PackedMap::new_encoded(work.clone(), limits(), 2, body(), 0, domain).unwrap();
        assert!(map.all_codes_valid);
        let first = if domain == Domain::ascii() {
            b'\0'
        } else {
            b'p'
        };
        assert_eq!(begin_key(&mut map, &[first, first]), None);
        let invalid = 1u128 << (2 * u64::from(map.domain.bits()));
        map.frame.key_low = invalid as u64;
        map.frame.key_high = (invalid >> 64) as u64;
        let before = (
            table(&map),
            map.updates(),
            map.frame.len,
            memory.used_bytes(),
            memory.retained_nodes(),
            work.used(),
        );
        assert!(
            map.insert_missing(invalid as u64, (invalid >> 64) as u64, 9)
                .is_err()
        );
        assert_eq!(
            (
                table(&map),
                map.updates(),
                map.frame.len,
                memory.used_bytes(),
                memory.retained_nodes(),
                work.used()
            ),
            before
        );
        drop((map, work));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
