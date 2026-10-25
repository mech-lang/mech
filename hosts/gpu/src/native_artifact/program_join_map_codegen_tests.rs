use super::*;
use cranelift_module::default_libcall_names;

type Entry = unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64;
struct Compiled {
    entry: Entry,
    _module: cranelift_jit::JITModule,
}
impl Compiled {
    fn new(vectorized: bool) -> Self {
        let catalog = mech_stdlib::source_catalog();
        let mut compiler = mech_runtime::RuntimeBuilder::new()
            .function_catalog(catalog.clone())
            .build_compiler()
            .unwrap();
        let product = compiler.compile_canonical_source(
            "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} | piece <- seed<string>], \"\")\n"
        ).unwrap();
        let plan = mech_engine::native_artifact::lower_native_program_artifact(
            product.artifact(),
            &catalog,
        )
        .unwrap();
        let regions = plan.string_map_regions();
        assert_eq!(regions.len(), 1);
        let builder = cranelift_jit::JITBuilder::with_flags(
            &[("opt_level", "speed"), ("preserve_frame_pointers", "true")],
            default_libcall_names(),
        )
        .unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let (id, _, _) = define_impl(&mut module, &regions[0], vectorized).unwrap();
        module.finalize_definitions().unwrap();
        let entry =
            unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
        Self {
            entry,
            _module: module,
        }
    }

    fn check(
        &self,
        source: &[u8],
        offsets: Option<&[u32]>,
        first: usize,
        rows: usize,
        table: &[u16; 128],
        active: bool,
        alignment: usize,
    ) {
        assert!(source.is_ascii());
        let flat = if let Some(offsets) = offsets {
            let mut bytes = Vec::new();
            for index in first..first + rows {
                let start = if index == 0 {
                    0
                } else {
                    offsets[index - 1] as usize + 1
                };
                let lf = offsets[index] as usize;
                assert_eq!(source[lf], b'\n');
                let end = lf - usize::from(lf > start && source[lf - 1] == b'\r');
                bytes.extend_from_slice(&source[start..end]);
            }
            bytes
        } else {
            assert_eq!((first, rows), (0, 1));
            source.to_vec()
        };
        let mut expected = flat.clone();
        let processed = if active {
            let missing = flat
                .iter()
                .position(|key| table[*key as usize] >= 128)
                .unwrap_or(flat.len());
            for index in 0..missing {
                expected[index] = table[flat[index] as usize] as u8;
            }
            missing
        } else {
            0
        };
        let lead = 17 + alignment;
        let mut output = vec![0xca; lead + flat.len() + 19];
        let original = source.to_vec();
        let mut geometry = [
            source.as_ptr() as u64,
            source.len() as u64,
            offsets.map_or(0, |v| v.as_ptr() as u64),
            first as u64,
            rows as u64,
            offsets.map_or(0, |v| v.len() as u64),
            0,
        ];
        let before = geometry;
        // The old u16 table remains the independent scalar oracle. The shared
        // owner preparation may select either representation, preserving every
        // ASCII value, missing-key outcome and valid zero.
        let prepared = crate::native_artifact::program_ascii_map::test_table(table);
        // SAFETY: immutable ASCII data, real LF allocation, fixed initialized
        // table and distinct output extent all outlive this synchronous call.
        // Guard bytes make row/tail overwrites visible. The kernel must fill
        // the raw suffix after a missing key, not just report its prefix.
        let actual = unsafe {
            (self.entry)(
                geometry.as_mut_ptr() as u64,
                flat.len() as u64,
                prepared.as_ptr() as u64,
                output.as_mut_ptr().add(lead) as u64,
                u64::from(active),
            )
        };
        assert_eq!(actual, processed as u64);
        assert_eq!(geometry[..6], before[..6]);
        assert_eq!(geometry[6], flat.len() as u64);
        assert_eq!(&output[lead..lead + flat.len()], expected.as_slice());
        assert!(output[..lead].iter().all(|v| *v == 0xca));
        assert!(output[lead + flat.len()..].iter().all(|v| *v == 0xca));
        assert_eq!(source, original);
    }
}

fn implementations() -> [Compiled; 2] {
    [
        Compiled::new(false),
        Compiled::new(cfg!(all(target_arch = "aarch64", target_endian = "little"))),
    ]
}

#[test]
fn joined_transform_matches_every_ascii_key_short_tail_alignment_and_inactive_copy() {
    let implementations = implementations();
    let table = core::array::from_fn(|i| ((i * 37 + 11) % 128) as u16);
    for length in (0..=81).chain([127, 128, 129, 255, 256, 257, 4096]) {
        let source = (0..length).map(|i| (i % 128) as u8).collect::<Vec<_>>();
        for c in &implementations {
            for active in [false, true] {
                for alignment in 0..8 {
                    c.check(&source, None, 0, 1, &table, active, alignment);
                }
            }
        }
    }
    for length in 0..=70 {
        let a = "a".repeat(length);
        let source = format!("skip\n{a}\r\n\n{a}\nlast\r");
        let offsets = source
            .bytes()
            .enumerate()
            .filter_map(|(i, b)| (b == b'\n').then_some(i as u32))
            .collect::<Vec<_>>();
        for first in 0..offsets.len() {
            for rows in 1..=offsets.len() - first {
                for c in &implementations {
                    c.check(
                        source.as_bytes(),
                        Some(&offsets),
                        first,
                        rows,
                        &table,
                        true,
                        length % 8,
                    );
                }
            }
        }
    }
}

#[test]
fn joined_transform_keeps_first_missing_zero_outputs_and_fills_every_raw_suffix() {
    let implementations = implementations();
    for length in [1, 7, 8, 15, 16, 17, 31, 32, 33, 59, 60, 61, 63, 64, 65, 129] {
        for missing in 0..length {
            let mut source = vec![b'A'; length];
            source[missing] = b'B';
            for marker in (128..=255).chain([256, 32767, 32768, u16::MAX]) {
                let mut table = [0u16; 128];
                table[b'B' as usize] = marker;
                for c in &implementations {
                    c.check(&source, None, 0, 1, &table, true, missing % 8);
                }
            }
        }
    }
    for source in ["AAAA\nB\r\nAAAA\n", "A\nAAAA\nB\nAAAA\n", "\nB\r\n\n"] {
        let offsets = source
            .bytes()
            .enumerate()
            .filter_map(|(i, b)| (b == b'\n').then_some(i as u32))
            .collect::<Vec<_>>();
        let mut table = [0u16; 128];
        table[b'B' as usize] = 256;
        for c in &implementations {
            for active in [false, true] {
                c.check(
                    source.as_bytes(),
                    Some(&offsets),
                    0,
                    offsets.len(),
                    &table,
                    active,
                    3,
                );
            }
        }
    }
}

#[test]
fn joined_transform_rejects_bad_extents_before_publishing_initialization() {
    let implementations = implementations();
    let source = b"aa\nbb\n";
    let table = crate::native_artifact::program_ascii_map::test_table(&[0u16; 128]);
    let offsets = [2u32, 5];
    for c in &implementations {
        for case in 0..8 {
            let mut output = [0xcau8; 32];
            let mut words = [
                source.as_ptr() as u64,
                source.len() as u64,
                offsets.as_ptr() as u64,
                0,
                2,
                2,
                0,
            ];
            let mut payload = 4;
            let mut active = 1;
            match case {
                0 => words[3] = u64::MAX,
                1 => words[4] = 3,
                2 => words[1] = 3,
                3 => payload = 3,
                4 => payload = 5,
                5 => active = 2,
                6 => {
                    words[2] = 0;
                    words[5] = 0;
                }
                7 => {
                    words[2] = 0;
                    words[4] = 1;
                    words[5] = 1;
                }
                _ => unreachable!(),
            }
            let result = unsafe {
                (c.entry)(
                    words.as_mut_ptr() as u64,
                    payload,
                    table.as_ptr() as u64,
                    output.as_mut_ptr().add(8) as u64,
                    active,
                )
            };
            assert_eq!(result, u64::MAX, "case {case}");
            assert_eq!(words[6], 0);
            assert!(output[..8].iter().all(|v| *v == 0xca));
            assert!(output[8 + payload as usize..].iter().all(|v| *v == 0xca));
        }
    }
}

#[test]
fn joined_byte_table_rejects_every_non_ascii_key_without_publishing() {
    let implementations = implementations();
    for cells in [[0u16; 128], core::array::from_fn(|key| key as u16)] {
        let table = crate::native_artifact::program_ascii_map::test_table(&cells);
        for c in &implementations {
            for length in [1, 15, 16, 17, 31, 32, 33, 63] {
                for invalid in 128..=255 {
                    for at in [0, length / 2, length - 1] {
                        let mut source = vec![b'A'; length];
                        source[at] = invalid;
                        let original = source.clone();
                        let mut output = vec![0xca; length + 16];
                        let mut words = [source.as_ptr() as u64, length as u64, 0, 0, 1, 0, 0];
                        let geometry = words;
                        let result = unsafe {
                            (c.entry)(
                                words.as_mut_ptr() as u64,
                                length as u64,
                                table.as_ptr() as u64,
                                output.as_mut_ptr().add(8) as u64,
                                1,
                            )
                        };
                        assert_eq!(result, u64::MAX);
                        assert_eq!(words, geometry);
                        assert_eq!(source, original);
                        assert!(output[..8].iter().all(|v| *v == 0xca));
                        assert!(output[8 + length..].iter().all(|v| *v == 0xca));
                    }
                }
            }
        }
    }
}

#[test]
fn projected_joined_map_keeps_domain_failures_zero_outputs_and_every_raw_suffix() {
    let implementations = implementations();
    let table = core::array::from_fn(|key| {
        if key & 64 == 64 && key & 31 != 11 {
            ((key & 31) * 7 % 128) as u16
        } else {
            256
        }
    });
    let prepared = crate::native_artifact::program_ascii_map::test_table(&table);
    assert_eq!(prepared[crate::native_artifact::program_ascii_map::KIND], 1);
    for length in [1, 7, 8, 15, 16, 17, 31, 32, 33, 60, 65] {
        for key in 0..128 {
            for at in [0, length / 2, length - 1] {
                let mut source = vec![b'@'; length];
                source[at] = key;
                for c in &implementations {
                    c.check(&source, None, 0, 1, &table, true, at % 8);
                }
            }
        }
    }
    let source = b"@AP\r\n\n@AP\n@";
    let offsets = source
        .iter()
        .enumerate()
        .filter_map(|(i, &b)| (b == b'\n').then_some(i as u32))
        .collect::<Vec<_>>();
    for c in &implementations {
        c.check(source, Some(&offsets), 0, offsets.len(), &table, true, 3);
    }
}
