use super::*;
use cranelift_module::default_libcall_names;
use std::collections::BTreeMap;

struct Kernel {
    entry: unsafe extern "C" fn(*mut Frame) -> u64,
    _module: cranelift_jit::JITModule,
}
impl Kernel {
    fn new(vectorized: bool) -> Self {
        Self::funded(vectorized, Funding::Metered)
    }
    fn funded(vectorized: bool, funding: Funding) -> Self {
        Self::classified(vectorized, funding, false)
    }
    fn classified(vectorized: bool, funding: Funding, classified: bool) -> Self {
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed")],
            default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let (id, _, _) = define(
            &mut module,
            "borrowed_utf8_histogram",
            vectorized,
            funding,
            true,
            classified,
        )
        .unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: the live module owns the complete one-pointer leaf ABI.
        let entry = unsafe {
            std::mem::transmute::<*const u8, unsafe extern "C" fn(*mut Frame) -> u64>(
                module.get_finalized_function(id),
            )
        };
        Self {
            entry,
            _module: module,
        }
    }
}
fn reference<'a>(strings: impl IntoIterator<Item = &'a str>) -> BTreeMap<&'a str, u64> {
    let mut expected = BTreeMap::new();
    for text in strings {
        *expected.entry(text).or_default() += 1;
    }
    expected
}
fn verify<'a>(frame: &Frame, slots: &[Slot], texts: &[&'a str]) {
    let expected = reference(texts[..frame.completed as usize].iter().copied());
    let mut actual = BTreeMap::new();
    for slot in slots.iter().filter(|s| s.row_plus_one != 0) {
        assert!(slot.row_plus_one <= frame.completed);
        let key = texts[slot.row_plus_one as usize - 1];
        assert!(actual.insert(key, slot.count).is_none());
    }
    assert_eq!(actual, expected);
    assert_eq!(frame.distinct as usize, expected.len());
    assert_eq!(
        frame.payload,
        expected.keys().map(|s| s.len() as u64).sum::<u64>()
    );
}
fn frame(cells: &[Cell], slots: &mut [Slot]) -> Frame {
    Frame {
        cells: cells.as_ptr() as u64,
        count: cells.len() as u64,
        slots: slots.as_mut_ptr() as u64,
        capacity: slots.len() as u64,
        limit: slots.len() as u64,
        fuel: u64::MAX,
        comparison: u64::MAX,
        completed: 0,
        distinct: 0,
        payload: 0,
        representatives: 0,
    }
}
fn cells(texts: &[&str]) -> Vec<Cell> {
    texts
        .iter()
        .map(|s| Cell {
            address: s.as_ptr() as u64,
            bytes: s.len() as u64,
        })
        .collect()
}

#[test]
fn generated_histogram_scalar_and_wide_reads_preserve_utf8_counts_and_every_tail() {
    let scalar = Kernel::new(false);
    let wide = Kernel::new(true);
    let prepaid = Kernel::funded(true, Funding::Prepaid);
    let mut owned = vec![
        "".to_owned(),
        "a\0".into(),
        "a".into(),
        "東京".into(),
        "é".into(),
    ];
    // Exercise every short/tail byte length, including the new exact two-byte
    // equality chunk. These lengths are independent of the benchmark fixture.
    owned.extend((0..=97).map(|n| "a".repeat(n)));
    for n in 0..70 {
        owned.push(format!("{}é\0{}", "x".repeat(n), "y".repeat(70 - n)));
    }
    for count in [0, 1, 2, 3, 7, 16, 31, 64, 255, 1024] {
        let texts = (0..count)
            .map(|i| owned[(i * 37 + i / 7) % owned.len()].as_str())
            .collect::<Vec<_>>();
        let cells = cells(&texts);
        for kernel in [&scalar, &wide, &prepaid] {
            let guard = Slot {
                hash: 0x123456789,
                count: 17,
                row_plus_one: 19,
            };
            let mut bank = vec![Slot::default(); 258];
            bank[0] = guard;
            bank[257] = guard;
            let mut f = frame(&cells, &mut bank[1..257]);
            // SAFETY: both disjoint banks and every string owner remain live.
            assert_eq!(unsafe { (kernel.entry)(&mut f) }, COMPLETE);
            assert_eq!(f.completed, count as u64);
            verify(&f, &bank[1..257], &texts);
            assert_eq!(bank[0], guard);
            assert_eq!(bank[257], guard);
        }
    }
}

#[test]
fn refused_native_histogram_keeps_only_the_complete_prefix_and_can_resume() {
    for vectorized in [false, true] {
        let kernel = Kernel::new(vectorized);
        let owned = (0..32)
            .map(|i| format!("shared-long-prefix-{}-東京", i % 9))
            .collect::<Vec<_>>();
        let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
        let cells = cells(&texts);
        for fuel in [0, 1, 15, 16, 17, 31, 32, 63, 127, 255, 511, 1023] {
            for comparison in [0, 1, 3, 4, 7, 8, 15, 16, 31, 127, u64::MAX] {
                let mut slots = vec![Slot::default(); 16];
                let mut f = frame(&cells, &mut slots);
                f.fuel = fuel;
                f.comparison = comparison;
                // SAFETY: exact aligned owned banks, and no aliases to writes.
                let status = unsafe { (kernel.entry)(&mut f) };
                assert!(matches!(status, COLD | COMPLETE));
                verify(&f, &slots, &texts);
                assert!(f.fuel <= fuel && f.comparison <= comparison);
                f.fuel = u64::MAX;
                f.comparison = u64::MAX;
                assert_eq!(unsafe { (kernel.entry)(&mut f) }, COMPLETE);
                assert_eq!(f.completed, texts.len() as u64);
                verify(&f, &slots, &texts);
            }
        }
    }
}

#[test]
fn full_banks_terminate_and_malformed_headers_never_commit_a_row() {
    let kernel = Kernel::new(true);
    let texts = ["a", "a", "b", "b", "c", "d", "a"];
    let cells = cells(&texts);
    for capacity in [1, 2, 4] {
        let mut slots = vec![Slot::default(); capacity];
        let mut f = frame(&cells, &mut slots);
        // SAFETY: capacity is truthful even when insufficient for all keys.
        assert_eq!(
            unsafe { (kernel.entry)(&mut f) },
            if capacity < 4 { FULL } else { COMPLETE }
        );
        verify(&f, &slots, &texts);
    }
    for bad in [0, 3, u64::MAX] {
        let mut slots = vec![Slot::default(); 4];
        let mut f = frame(&cells, &mut slots);
        f.capacity = bad;
        assert_eq!(unsafe { (kernel.entry)(&mut f) }, INVALID);
        assert_eq!(f.completed, 0);
        assert!(slots.iter().all(|s| s.row_plus_one == 0));
    }
}

#[test]
fn exact_pass_bound_covers_metered_work_and_prepaid_output_at_every_occupancy() {
    let metered = Kernel::new(true);
    let prepaid = Kernel::funded(true, Funding::Prepaid);
    for count in [0, 1, 2, 3, 47, 48, 49, 64, 65, 127, 257, 1024] {
        let owned = (0..count)
            .map(|i| format!("{}é\0{}", "x".repeat(i % 71), i % 89))
            .collect::<Vec<_>>();
        let texts = owned.iter().map(String::as_str).collect::<Vec<_>>();
        let cells = cells(&texts);
        let bytes = texts.iter().map(|s| s.len() as u64).sum::<u64>();
        for capacity in [4, 16, 64, 256, 2048] {
            let (fuel, comparison) =
                crate::native_artifact::program::runtime::maps::utf8_histogram::pass_work(
                    count as u64,
                    bytes,
                    capacity,
                )
                .unwrap();
            let mut prior = None;
            for kernel in [&metered, &prepaid] {
                let mut slots = vec![Slot::default(); capacity as usize];
                let mut f = frame(&cells, &mut slots);
                f.limit =
                    crate::native_artifact::program::runtime::maps::utf8_histogram::bank_limit(
                        capacity as usize,
                    );
                f.fuel = fuel;
                f.comparison = comparison;
                // SAFETY: this full native pass is funded from its complete
                // checked input-byte/probe bound, with genuine disjoint owners.
                let status = unsafe { (kernel.entry)(&mut f) };
                assert!(matches!(status, COMPLETE | FULL));
                verify(&f, &slots, &texts);
                assert!(f.fuel <= fuel && f.comparison <= comparison);
                let result = (status, f.completed, f.distinct, f.payload, slots);
                if let Some(prior) = &prior {
                    assert_eq!(&result, prior);
                }
                prior = Some(result);
            }
        }
    }
    use crate::native_artifact::program::runtime::maps::utf8_histogram::{
        bank_limit, maximum_capacity, pass_work,
    };
    for count in 0..4096 {
        let cap = maximum_capacity(count).unwrap();
        assert!(bank_limit(cap) >= count as u64);
    }
    assert!(maximum_capacity(usize::MAX).is_none());
    assert!(pass_work(u64::MAX, 1, 4).is_none());
    assert!(pass_work(1, u64::MAX, 4).is_none());
}

#[test]
fn native_string_classes_record_first_equal_rows_across_full_replay_unicode_and_empty_inputs() {
    for funding in [Funding::Metered, Funding::Prepaid] {
        let kernel = Kernel::classified(true, funding, true);
        let names = (0..97)
            .map(|n| format!("{}é\0東京", "x".repeat(n)))
            .collect::<Vec<_>>();
        for texts in [
            vec![],
            vec!["", "", "é", "", "é"],
            names.iter().map(String::as_str).cycle().take(500).collect(),
        ] {
            let cells = cells(&texts);
            let mut representatives = vec![u64::MAX; texts.len()];
            let mut capacity = 4;
            loop {
                let mut slots = vec![Slot::default(); capacity];
                let mut frame = frame(&cells, &mut slots);
                frame.limit =
                    crate::native_artifact::program::runtime::maps::utf8_histogram::bank_limit(
                        capacity,
                    );
                frame.representatives = representatives.as_mut_ptr() as u64;
                let status = unsafe { (kernel.entry)(&mut frame) };
                verify(&frame, &slots, &texts);
                for row in 0..frame.completed as usize {
                    let first = texts.iter().position(|text| *text == texts[row]).unwrap();
                    assert_eq!(representatives[row], first as u64);
                }
                if status == COMPLETE {
                    break;
                }
                assert_eq!(status, FULL);
                capacity *= 2;
            }
        }
    }
}
