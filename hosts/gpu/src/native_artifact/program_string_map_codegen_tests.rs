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
            "+> text\n+> map\ntext/join([(entries<{string:string}>){piece} | piece <- seed<string>], \"\")\n",
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
        // SAFETY: define_impl emits this exact private five-word signature;
        // this test owner retains the generated code for every call.
        let entry =
            unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
        Self {
            entry,
            _module: module,
        }
    }

    fn check(&self, source: &[u8], table: &[u16; 128], alignment: usize, in_place: bool) {
        assert!(source.is_ascii());
        assert!(!source.windows(2).any(|pair| pair == b"\r\n"));
        let offset = 16 + alignment;
        let mut input = vec![0xda; offset + source.len() + 17];
        input[offset..offset + source.len()].copy_from_slice(source);
        let original_input = input.clone();
        let mut output = if in_place {
            input.clone()
        } else {
            vec![0xcb; input.len()]
        };
        let original_output = output.clone();
        let prepared = crate::native_artifact::program_ascii_map::test_table(table);
        let first_missing = source
            .iter()
            .position(|byte| table[*byte as usize] >= 128)
            .unwrap_or(source.len());
        // SAFETY: all source bytes satisfy the admitted byte/grapheme guard;
        // the initialized 128-cell table and actual source/output extents
        // outlive this call. In-place operation has one exclusive Vec owner;
        // no Rust slice reference aliases the raw writable output loan.
        let actual = unsafe {
            let source_pointer = if in_place {
                output.as_ptr().add(offset)
            } else {
                input.as_ptr().add(offset)
            };
            (self.entry)(
                source_pointer as u64,
                source.len() as u64,
                prepared.as_ptr() as u64,
                output.as_mut_ptr().add(offset) as u64,
                0,
            )
        };
        assert_eq!(actual, first_missing as u64);
        for index in 0..first_missing {
            assert_eq!(output[offset + index], table[source[index] as usize] as u8);
        }
        assert_eq!(&output[..offset], &original_output[..offset]);
        assert_eq!(
            &output[offset + first_missing..],
            &original_output[offset + first_missing..]
        );
        assert_eq!(input, original_input);
    }
}

#[test]
fn byte_map_batches_match_scalar_for_every_ascii_key_tail_and_alignment() {
    let scalar = Compiled::new(false);
    let vector = Compiled::new(cfg!(all(target_arch = "aarch64", target_endian = "little")));
    let table = std::array::from_fn(|index| ((index * 37 + 11) % 128) as u16);
    for length in (0..=129).chain([255, 256, 257, 1023, 4096]) {
        let source = (0..length)
            .map(|index| (index % 128) as u8)
            .collect::<Vec<_>>();
        for alignment in 0..8 {
            for in_place in [false, true] {
                scalar.check(&source, &table, alignment, in_place);
                vector.check(&source, &table, alignment, in_place);
            }
        }
    }
}

#[test]
fn byte_map_batches_keep_the_exact_first_failure_for_all_unsigned_markers() {
    let scalar = Compiled::new(false);
    let vector = Compiled::new(cfg!(all(target_arch = "aarch64", target_endian = "little")));
    for length in [1, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 129] {
        for missing in 0..length {
            let mut source = vec![b'A'; length];
            source[missing] = b'B';
            for marker in [128, 129, 255, 256, 32767, 32768, u16::MAX] {
                let mut table = std::array::from_fn(|index| index as u16);
                table[b'B' as usize] = marker;
                for in_place in [false, true] {
                    scalar.check(&source, &table, missing % 8, in_place);
                    vector.check(&source, &table, missing % 8, in_place);
                }
            }
        }
    }
}

#[test]
fn compact_maps_preserve_all_ascii_domains_zero_missing_keys_tails_and_aliasing() {
    let scalar = Compiled::new(false);
    let vector = Compiled::new(cfg!(all(target_arch = "aarch64", target_endian = "little")));
    for mask in [0, 32, 64, 96] {
        for domain in [0, 32, 64, 96] {
            let table = core::array::from_fn(|key| {
                if key as u8 & mask == domain & mask && key & 31 != 11 {
                    ((key & 31) * 7 % 128) as u16
                } else {
                    256
                }
            });
            let prepared = crate::native_artifact::program_ascii_map::test_table(&table);
            assert_eq!(prepared[crate::native_artifact::program_ascii_map::KIND], 1);
            let filler = (0..128)
                .find(|&key| table[key] < 128 && !matches!(key, 10 | 13))
                .unwrap() as u8;
            for length in [1, 7, 8, 15, 16, 17, 31, 32, 33, 60, 65] {
                for key in 0..128 {
                    let mut source = vec![filler; length];
                    source[length / 2] = key;
                    for in_place in [false, true] {
                        scalar.check(&source, &table, length % 8, in_place);
                        vector.check(&source, &table, length % 8, in_place);
                    }
                }
            }
        }
    }
}
