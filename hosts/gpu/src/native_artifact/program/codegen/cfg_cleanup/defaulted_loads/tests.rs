use super::*;
use cranelift_codegen::ir::{AbiParam, MemFlags};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::Linkage;

#[test]
fn default_path_does_not_read_the_original_address() {
    let mut module =
        JITModule::new(JITBuilder::new(cranelift_module::default_libcall_names()).unwrap());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let yes = b.create_block();
    let no = b.create_block();
    let join = b.create_block();
    b.append_block_params_for_function_params(entry);
    let result = b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    let [base, index, length] = b.block_params(entry).try_into().unwrap();
    let bounded = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    b.ins().brif(bounded, yes, &[], no, &[]);
    b.switch_to_block(yes);
    let bytes = b.ins().imul_imm(index, 4);
    let address = b.ins().iadd(base, bytes);
    let word = b.ins().uload32(MemFlags::new(), address, 0);
    b.ins().jump(join, &[word.into()]);
    b.switch_to_block(no);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(join, &[zero.into()]);
    b.switch_to_block(join);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();

    assert_eq!(fold(&mut module, &mut context).unwrap(), 1);
    let id = module
        .declare_function("defaulted_word", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let read: unsafe extern "C" fn(*const u32, u64, u64) -> u64 =
        unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    let words = [0xfedc_ba98, 19];
    unsafe {
        assert_eq!(read(words.as_ptr(), 0, 2), 0xfedc_ba98);
        assert_eq!(read(words.as_ptr(), 1, 2), 19);
        assert_eq!(read(words.as_ptr(), 2, 2), 0);
        assert_eq!(read(std::ptr::null(), u64::MAX, 0), 0);
    }
}

fn immutable_default(
    width: u8,
    address_bias: i64,
    length_word: usize,
) -> (JITModule, *const u8, (usize, usize), Vec<MemFlags>) {
    let mut module =
        JITModule::new(JITBuilder::new(cranelift_module::default_libcall_names()).unwrap());
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    let yes = b.create_block();
    let no = b.create_block();
    let join = b.create_block();
    b.append_block_params_for_function_params(entry);
    let result = b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    let [base, index, length] = b.block_params(entry).try_into().unwrap();
    let bounded = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    b.ins().brif(bounded, yes, &[], no, &[]);
    b.switch_to_block(yes);
    // The real source translates its zero-based default selector through
    // wrapping one-based selection. The cancellation must remain exact.
    let one_based = b.ins().iadd_imm(index, 1);
    let ordinal = b.ins().iadd_imm(one_based, address_bias - 1);
    let bytes = b.ins().imul_imm(ordinal, i64::from(width));
    let address = b.ins().iadd(base, bytes);
    let word = if width == 1 {
        b.ins().uload8(types::I64, MemFlags::new(), address, 0)
    } else {
        b.ins().uload32(MemFlags::new(), address, 0)
    };
    let load = b.func.dfg.value_def(word).unwrap_inst();
    b.ins().jump(join, &[word.into()]);
    b.switch_to_block(no);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(join, &[zero.into()]);
    b.switch_to_block(join);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    let packet = BTreeMap::from([(base, (load, 0)), (length, (load, 1))]);
    let views = [View {
        array: 0,
        pointer_word: 0,
        length_word,
        width,
        literal: None,
    }];
    let counts = fold_immutable(&mut module, &mut context, &packet, &views, false).unwrap();
    let flags = context
        .func
        .layout
        .blocks()
        .flat_map(|block| context.func.layout.block_insts(block))
        .filter_map(|inst| match context.func.dfg.insts[inst] {
            Data::Load { flags, .. } => Some(flags),
            _ => None,
        })
        .collect();
    let id = module
        .declare_function("immutable_default", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let address = module.get_finalized_function(id);
    (module, address, counts, flags)
}

#[test]
fn immutable_default_addresses_keep_narrow_values_null_and_wrapping_boundaries() {
    for width in [1, 4] {
        let (_module, address, counts, flags) = immutable_default(width, 0, 1);
        assert_eq!(counts, (1, 1));
        assert_eq!(flags.len(), 1);
        assert!(flags[0].readonly() && flags[0].notrap() && !flags[0].can_move());
        let read: unsafe extern "C" fn(*const u8, u64, u64) -> u64 =
            unsafe { std::mem::transmute(address) };
        let bytes = [0xfe_u8, 0x80, 0x44];
        let words = [0xfedc_ba98_u32, 0x8000_0000, 0x4444];
        let (base, expected) = if width == 1 {
            (bytes.as_ptr(), [0xfe, 0x80])
        } else {
            (words.as_ptr().cast::<u8>(), [0xfedc_ba98, 0x8000_0000])
        };
        unsafe {
            assert_eq!(read(base, 0, 2), expected[0]);
            assert_eq!(read(base, 1, 2), expected[1]);
            for index in [2, 3, u64::MAX - 1, u64::MAX] {
                assert_eq!(read(base, index, 2), 0);
            }
            assert_eq!(read(std::ptr::null(), 0, 0), 0);
            assert_eq!(read(std::ptr::null(), u64::MAX, 0), 0);
        }
    }
}

#[test]
fn unknown_length_or_shifted_address_never_gains_memory_motion_flags() {
    for width in [1, 4] {
        for (bias, length_word) in [(0, 0), (1, 1), (-1, 1)] {
            let (_module, _, counts, flags) = immutable_default(width, bias, length_word);
            assert_eq!(counts, (0, 0));
            assert!(flags.iter().all(|f| !f.readonly() && !f.can_move()));
        }
    }
}

fn byte_read(
    b: &mut FunctionBuilder<'_>,
    base: Value,
    index: Value,
    length: Value,
) -> (Value, Inst) {
    let yes = b.create_block();
    let no = b.create_block();
    let join = b.create_block();
    let result = b.append_block_param(join, types::I64);
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, length);
    b.ins().brif(valid, yes, &[], no, &[]);
    b.switch_to_block(yes);
    let address = b.ins().iadd(base, index);
    let loaded = b.ins().uload8(types::I64, MemFlags::new(), address, 0);
    let load = b.func.dfg.value_def(loaded).unwrap_inst();
    b.ins().jump(join, &[loaded.into()]);
    b.switch_to_block(no);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(join, &[zero.into()]);
    b.switch_to_block(join);
    (result, load)
}

fn shared_loop(mode: u8, rematerialize: bool) -> (JITModule, *const u8, usize) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed")],
            cranelift_module::default_libcall_names(),
        )
        .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [base, seed, length, choose, iterations] = b.block_params(entry).try_into().unwrap();
    let header = b.create_block();
    let index = b.append_block_param(header, types::I64);
    let remaining = b.append_block_param(header, types::I64);
    let sum = b.append_block_param(header, types::I64);
    let body = b.create_block();
    let exit = b.create_block();
    let zero = b.ins().iconst(types::I64, 0);
    b.ins()
        .jump(header, &[seed.into(), iterations.into(), zero.into()]);
    b.switch_to_block(header);
    b.ins().brif(remaining, body, &[], exit, &[]);
    b.switch_to_block(body);
    let first = b.create_block();
    let skipped = b.create_block();
    let merge = b.create_block();
    let copied_index = b.append_block_param(merge, types::I64);
    let first_byte = b.append_block_param(merge, types::I64);
    b.ins().brif(choose, first, &[], skipped, &[]);
    b.switch_to_block(first);
    let (byte, load) = byte_read(&mut b, base, if mode == 2 { seed } else { index }, length);
    b.ins().jump(merge, &[index.into(), byte.into()]);
    b.switch_to_block(skipped);
    let alternate = if mode == 1 {
        b.ins().iadd_imm(index, 1)
    } else {
        index
    };
    b.ins().jump(merge, &[alternate.into(), zero.into()]);
    b.switch_to_block(merge);
    let copy = b.create_block();
    let twice_copied = b.append_block_param(copy, types::I64);
    b.ins().jump(copy, &[copied_index.into()]);
    b.switch_to_block(copy);
    let (second, _) = byte_read(&mut b, base, twice_copied, length);
    let pair = b.ins().iadd(first_byte, second);
    let total = b.ins().iadd(sum, pair);
    let next_index = b.ins().iadd_imm(index, 1);
    let next_remaining = b.ins().iadd_imm(remaining, -1);
    b.ins().jump(
        header,
        &[next_index.into(), next_remaining.into(), total.into()],
    );
    b.switch_to_block(exit);
    b.ins().return_(&[sum]);
    b.seal_all_blocks();
    b.finalize();
    let packet = BTreeMap::from([(base, (load, 0)), (length, (load, 1))]);
    let views = [View {
        array: 0,
        pointer_word: 0,
        length_word: 1,
        width: 1,
        literal: None,
    }];
    assert_eq!(
        fold_immutable(&mut module, &mut context, &packet, &views, rematerialize).unwrap(),
        (2, 2)
    );
    let loads = context
        .func
        .layout
        .blocks()
        .flat_map(|block| context.func.layout.block_insts(block))
        .filter(|&inst| {
            matches!(context.func.dfg.insts[inst], Data::Load { flags, .. } if flags.readonly())
        })
        .count();
    let id = module
        .declare_function(
            "shared_default_loop",
            Linkage::Local,
            &context.func.signature,
        )
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let address = module.get_finalized_function(id);
    (module, address, loads)
}

#[test]
fn shared_default_reads_preserve_current_loop_values_and_divergent_paths() {
    for (mode, rematerialize) in (0..=2).flat_map(|mode| [(mode, false), (mode, true)]) {
        let (_module, address, loads) = shared_loop(mode, rematerialize);
        assert_eq!(loads, if mode == 0 { 1 } else { 2 }, "mode {mode}");
        let read: unsafe extern "C" fn(*const u8, u64, u64, u64, u64) -> u64 =
            unsafe { std::mem::transmute(address) };
        let bytes = [0xfe_u8, 0x80, 19, 0, 7];
        for seed in [0, 1, 4, 5, u64::MAX - 1, u64::MAX] {
            for choose in [0, 1] {
                for iterations in [0, 1, 2, 7] {
                    let byte = |index: u64| bytes.get(index as usize).copied().unwrap_or(0) as u64;
                    let expected = (0..iterations).fold(0_u64, |sum, step| {
                        let index = seed.wrapping_add(step);
                        let first = if choose != 0 {
                            byte(if mode == 2 { seed } else { index })
                        } else {
                            0
                        };
                        let second = byte(if mode == 1 && choose == 0 {
                            index.wrapping_add(1)
                        } else {
                            index
                        });
                        sum.wrapping_add(first + second)
                    });
                    assert_eq!(
                        unsafe {
                            read(bytes.as_ptr(), seed, bytes.len() as u64, choose, iterations)
                        },
                        expected,
                        "mode {mode}, seed {seed}, choose {choose}, iterations {iterations}",
                    );
                    assert_eq!(
                        unsafe { read(std::ptr::null(), seed, 0, choose, iterations) },
                        0
                    );
                }
            }
        }
    }
}
