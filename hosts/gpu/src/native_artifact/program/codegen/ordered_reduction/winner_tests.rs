use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn unchanged_vector_minimum_skips_index_reconstruction_and_preserves_order() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut functions = Vec::new();
    for defer in [false, true] {
        let mut context = module.make_context();
        context.set_disasm(true);
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I32)];
        let mut builder = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [input, positions, output] = b.block_params(entry)[..] else {
            unreachable!()
        };
        let costs = b.ins().load(types::I32X4, MemFlags::new(), input, 0);
        let best = b.ins().load(types::I32, MemFlags::new(), input, 16);
        let positions = b.ins().load(types::I32X4, MemFlags::new(), positions, 0);
        let missing = b.ins().iconst(types::I32, -1);
        let (minimum, winner) = super::winner::emit(&mut b, costs, positions, best, missing, defer);
        b.ins().store(MemFlags::new(), minimum, output, 0);
        b.ins().store(MemFlags::new(), winner, output, 4);
        b.ins().return_(&[winner]);
        b.seal_all_blocks();
        b.finalize();
        let id = module
            .declare_function(
                &format!("strict_vector_winner_{defer}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        if defer {
            let assembly = context.compiled_code().unwrap().vcode.as_ref().unwrap();
            assert!(
                assembly.split_whitespace().any(|op| {
                    op.starts_with("b.") || ["cbz", "cbnz", "tbz", "tbnz"].contains(&op)
                }),
                "winner reconstruction must retain a conditional route: {assembly}"
            );
        }
        functions.push(id);
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u32, *const u32, *mut u32) -> u32;
    let calls = functions
        .into_iter()
        .map(|id| {
            // SAFETY: the live finalized module owns the exact declared C entry.
            unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) }
        })
        .collect::<Vec<_>>();
    let mut seed = 0x68f7_2d9a_0123_4567u64;
    for sample in 0..4096 {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u32
        };
        let mut best = next();
        let mut raw = std::array::from_fn::<_, 4, _>(|_| next());
        let mut positions = std::array::from_fn::<_, 4, _>(|_| next() % 65535);
        match sample % 8 {
            0 => {
                best = u32::MAX;
                raw = [u32::MAX; 4];
            }
            1 => {
                best = 0;
                raw = [0; 4];
            }
            2 => {
                best = 0x8000_0000;
                raw = [0x8000_0000, 0x8000_0001, u32::MAX, 0x8000_0000];
            }
            3 => {
                best = 0x8000_0000;
                raw = [0x7fff_ffff, 0x8000_0000, 0x7fff_ffff, u32::MAX];
                positions = [103, 1, 2, 60];
            }
            4 => {
                best = 8;
                raw = [7, 7, 8, 7];
                positions = [21, 18, 0, 3];
            }
            5 => {
                best = u32::MAX;
                raw = [u32::MAX - 1, u32::MAX, u32::MAX, u32::MAX - 1];
            }
            _ => {}
        }
        let mut input = [0u32; 5];
        for lane in 0..4 {
            input[lane] = raw[lane].min(best);
            if raw[lane] >= best {
                positions[lane] = u32::MAX;
            }
        }
        input[4] = best;
        let minimum = *input[..4].iter().min().unwrap();
        let winner = if minimum < best {
            (0..4)
                .filter(|&lane| input[lane] == minimum)
                .map(|lane| positions[lane])
                .min()
                .unwrap()
        } else {
            u32::MAX
        };
        let original = (input, positions);
        for call in &calls {
            let mut output = [0xa5a5_a5a5; 5];
            // SAFETY: both immutable inputs cover every declared read, the
            // disjoint output covers both writes, and all storage stays live.
            let actual = unsafe {
                call(
                    input.as_ptr(),
                    positions.as_ptr(),
                    output.as_mut_ptr().add(1),
                )
            };
            assert_eq!(actual, winner, "sample {sample}");
            assert_eq!(
                output,
                [0xa5a5_a5a5, minimum, winner, 0xa5a5_a5a5, 0xa5a5_a5a5]
            );
            assert_eq!((input, positions), original);
        }
    }
}

#[cfg(target_endian = "little")]
#[test]
fn unchanged_vector_choice_keeps_full_item_and_maps_only_strict_winners() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut functions = Vec::new();
    for defer in [false, true] {
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I64)];
        let mut builder = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [input, positions, output] = b.block_params(entry)[..] else {
            unreachable!()
        };
        let costs = b.ins().load(types::I32X4, MemFlags::new(), input, 0);
        let best = b.ins().load(types::I32, MemFlags::new(), input, 16);
        let old_item = b.ins().load(types::I64, MemFlags::new(), input, 24);
        let positions = b.ins().load(types::I32X4, MemFlags::new(), positions, 0);
        let missing = b.ins().iconst(types::I32, -1);
        let (minimum, item) = super::winner::emit_mapped(
            &mut b,
            costs,
            positions,
            best,
            missing,
            defer,
            old_item,
            |b, winner, improved| {
                let changed = (!improved).then(|| {
                    b.ins().icmp_imm(
                        cranelift_codegen::ir::condcodes::IntCC::NotEqual,
                        winner,
                        -1,
                    )
                });
                let index = b.ins().uextend(types::I64, winner);
                let high = b.ins().iconst(types::I64, 0x0123_4567_0000_0000);
                let packed = b.ins().bor(index, high);
                changed.map_or(packed, |changed| b.ins().select(changed, packed, old_item))
            },
        );
        let minimum = b.ins().uextend(types::I64, minimum);
        b.ins().store(MemFlags::new(), minimum, output, 0);
        b.ins().store(MemFlags::new(), item, output, 8);
        b.ins().return_(&[item]);
        b.seal_all_blocks();
        b.finalize();
        let id = module
            .declare_function(
                &format!("strict_vector_item_{defer}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        functions.push(id);
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u32, *const u32, *mut u64) -> u64;
    let calls = functions
        .into_iter()
        .map(|id| {
            // SAFETY: the live JIT owns this exact three-pointer C entry.
            unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) }
        })
        .collect::<Vec<_>>();
    let mut seed = 0xe381_47b0_a526_c9d2u64;
    for sample in 0..4096 {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u32
        };
        let best = match sample % 5 {
            0 => 0,
            1 => u32::MAX,
            2 => 0x8000_0000,
            _ => next(),
        };
        let mut raw = std::array::from_fn::<_, 4, _>(|_| next());
        let mut positions = std::array::from_fn::<_, 4, _>(|_| next() % 65535);
        match sample % 7 {
            0 => raw = [best; 4],
            1 => raw = [best.saturating_sub(1); 4],
            2 => {
                raw = [0, u32::MAX, 0, 0];
                positions = [51, 2, 14, 3];
            }
            _ => {}
        }
        let old_item = (u64::from(next()) << 32) | u64::from(next());
        let mut input = [0u32; 8];
        for lane in 0..4 {
            input[lane] = raw[lane].min(best);
            if raw[lane] >= best {
                positions[lane] = u32::MAX;
            }
        }
        input[4] = best;
        input[6] = old_item as u32;
        input[7] = (old_item >> 32) as u32;
        let minimum = *input[..4].iter().min().unwrap();
        let expected = if minimum < best {
            let winner = (0..4)
                .filter(|&lane| input[lane] == minimum)
                .map(|lane| positions[lane])
                .min()
                .unwrap();
            0x0123_4567_0000_0000 | u64::from(winner)
        } else {
            old_item
        };
        let original = (input, positions);
        for call in &calls {
            let guard = 0xa5a5_a5a5_a5a5_a5a5u64;
            let mut output = [guard; 5];
            // SAFETY: input covers both vectors and the complete old item;
            // initialized disjoint output covers both writes and stays live.
            let actual = unsafe {
                call(
                    input.as_ptr(),
                    positions.as_ptr(),
                    output.as_mut_ptr().add(1),
                )
            };
            assert_eq!(actual, expected, "sample {sample}");
            assert_eq!(output, [guard, u64::from(minimum), expected, guard, guard]);
            assert_eq!((input, positions), original);
        }
    }
}
