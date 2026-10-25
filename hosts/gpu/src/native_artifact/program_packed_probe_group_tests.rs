//! The aligned partition covers every slot, including a wrap to group zero.
use super::*;

#[test]
fn aligned_probe_groups_preserve_full_keys_and_complete_bounded_wraparound() {
    const CAPACITY: usize = 64;
    for domain in [Domain::ascii(), boundary_domain(b"ACGT")] {
        let mut slots = Slots::new(domain, CAPACITY).unwrap();
        let mut controls = empty_controls(CAPACITY).unwrap();
        let narrow = narrow(domain);
        let keys = (0..2_000_000u64)
            .filter_map(|low| {
                let high = if narrow {
                    0
                } else {
                    low.rotate_left(37) & ((1 << 62) - 1)
                };
                let mixed = hash(low, high);
                (mixed as usize & (CAPACITY - GROUP) == CAPACITY - GROUP && fingerprint(mixed) == 7)
                    .then_some((low, high))
            })
            .take(CAPACITY + 1)
            .collect::<Vec<_>>();
        assert_eq!(keys.len(), CAPACITY + 1);
        for (ordinal, &(low, high)) in keys[..CAPACITY].iter().enumerate() {
            let mut charges = Vec::new();
            let at = PackedMap::probe_debits(&slots, &controls, low, high, &mut |a, b| {
                charges.push((a, b));
                Ok(())
            })
            .unwrap();
            assert_eq!(at, (CAPACITY - GROUP + ordinal) % CAPACITY);
            assert_eq!(charges, vec![(1024, 16); ordinal / GROUP + 1]);
            let value = if ordinal % 2 == 0 { 0 } else { u64::MAX };
            slots.put(at, Slot { low, high, value });
            set_control(&mut controls, CAPACITY, at, 7);
            for (present, &(a, b)) in keys[..=ordinal].iter().enumerate() {
                let index = PackedMap::probe(&slots, &controls, a, b).unwrap();
                assert_eq!((slots.get(index).low, slots.get(index).high), (a, b));
                assert_eq!(
                    slots.get(index).value,
                    if present % 2 == 0 { 0 } else { u64::MAX }
                );
            }
        }
        let before = controls.clone();
        let mut groups = 0;
        assert!(
            PackedMap::probe_debits(
                &slots,
                &controls,
                keys[CAPACITY].0,
                keys[CAPACITY].1,
                &mut |_, _| {
                    groups += 1;
                    Ok(())
                }
            )
            .is_err()
        );
        assert_eq!(
            groups,
            CAPACITY / GROUP,
            "a full table must stop after its complete partition"
        );
        assert_eq!(controls, before);
        let mut charges = 0;
        assert!(
            PackedMap::probe_debits(&slots, &controls, keys[0].0, keys[0].1, &mut |_, _| {
                charges += 1;
                Err(mech_core::ResidentKernelError::InvalidInput)
            })
            .is_err()
        );
        assert_eq!(
            charges, 1,
            "admission failure precedes the first control read"
        );
    }
}
