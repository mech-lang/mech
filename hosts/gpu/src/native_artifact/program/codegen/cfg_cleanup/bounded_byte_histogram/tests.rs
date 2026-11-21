use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
type Run = unsafe extern "C" fn(u64, *const u64, *mut u64) -> u64;
fn fixture(
    module: &JITModule,
    mode: u8,
) -> (
    Context,
    Block,
    BTreeMap<Value, (Inst, usize)>,
    Vec<View>,
    Vec<View>,
) {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fc);
    let entry = b.create_block();
    let header = b.create_block();
    let body = b.create_block();
    let input_check = b.create_block();
    let input_yes = b.create_block();
    let input_no = b.create_block();
    let input_join = b.create_block();
    let count_yes = b.create_block();
    let count_no = b.create_block();
    let count_join = b.create_block();
    let write_check = b.create_block();
    let write = b.create_block();
    let done = b.create_block();
    let errors = [(); 3].map(|()| b.create_block());
    b.append_block_params_for_function_params(entry);
    let at = b.append_block_param(header, types::I64);
    let byte = b.append_block_param(input_join, types::I64);
    let count = b.append_block_param(count_join, types::I64);
    b.switch_to_block(entry);
    let packet = b.block_params(entry)[1];
    let error = b.block_params(entry)[2];
    let mut loads = BTreeMap::new();
    let fields: [Value; 7] = core::array::from_fn(|word| {
        let v = b
            .ins()
            .load(types::I64, MemFlags::new(), packet, (word * 8) as i32);
        let inst = b.func.dfg.value_def(v).unwrap_inst();
        loads.insert(v, (inst, word));
        v
    });
    let [begin, mut end, initial, input, input_len, bins, bin_len] = fields;
    if mode == 4 {
        end = b.ins().iadd_imm(end, 1);
    }
    if mode == 5 {
        let changed = b.ins().iconst(types::I32, 7);
        b.ins().store(MemFlags::new(), changed, bins, 0);
    }
    b.ins().jump(header, &[initial.into()]);
    b.switch_to_block(header);
    let active = b.ins().icmp(IntCC::UnsignedLessThan, at, end);
    b.ins().brif(active, body, &[], done, &[]);
    b.switch_to_block(body);
    let position = b.ins().iadd(begin, at);
    let fits = b
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, position, begin);
    b.ins().brif(fits, input_check, &[], errors[0], &[]);
    b.switch_to_block(input_check);
    let position = if mode == 2 {
        b.ins().iadd_imm(position, 1)
    } else {
        position
    };
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, position, input_len);
    b.ins().brif(valid, input_yes, &[], input_no, &[]);
    b.switch_to_block(input_yes);
    let address = b.ins().iadd(input, position);
    let word = b.ins().uload8(types::I64, MemFlags::new(), address, 0);
    b.ins().jump(input_join, &[word.into()]);
    b.switch_to_block(input_no);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(input_join, &[zero.into()]);
    b.switch_to_block(input_join);
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, byte, bin_len);
    b.ins().brif(valid, count_yes, &[], count_no, &[]);
    b.switch_to_block(count_yes);
    let offset = b.ins().imul_imm(byte, 4);
    let address = b.ins().iadd(bins, offset);
    let value = b.ins().uload32(MemFlags::new(), address, 0);
    b.ins().jump(count_join, &[value.into()]);
    b.switch_to_block(count_no);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(count_join, &[zero.into()]);
    b.switch_to_block(count_join);
    let updated = b.ins().iadd_imm(count, 1);
    let fits = b
        .ins()
        .icmp_imm(IntCC::UnsignedLessThanOrEqual, updated, u32::MAX as i64);
    b.ins().brif(fits, write_check, &[], errors[1], &[]);
    b.switch_to_block(write_check);
    let index = if mode == 3 {
        b.ins().iadd_imm(byte, 1)
    } else {
        byte
    };
    let valid = b.ins().icmp(IntCC::UnsignedLessThan, index, bin_len);
    b.ins().brif(valid, write, &[], errors[2], &[]);
    b.switch_to_block(write);
    let stored_index = if mode == 8 {
        b.ins().iadd_imm(index, 1)
    } else {
        index
    };
    let offset = b.ins().imul_imm(stored_index, 4);
    let address = b.ins().iadd(bins, offset);
    let stored_count = if mode == 7 {
        b.ins().iadd_imm(updated, 1)
    } else {
        updated
    };
    let value = b.ins().ireduce(types::I32, stored_count);
    if mode != 9 {
        b.ins().store(MemFlags::new(), value, address, 0);
    }
    if mode == 6 {
        b.ins().store(MemFlags::new(), value, address, 4);
    }
    let next = b.ins().iadd_imm(at, if mode == 1 { 2 } else { 1 });
    b.ins().jump(header, &[next.into()]);
    b.switch_to_block(done);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().return_(&[zero]);
    for (i, block) in errors.into_iter().enumerate() {
        b.switch_to_block(block);
        let origin = b.ins().iconst(types::I64, 100 + i as i64);
        b.ins().store(MemFlags::new(), origin, error, 0);
        let status = b.ins().iconst(types::I64, 1 + i as i64);
        b.ins().return_(&[status]);
    }
    b.seal_all_blocks();
    b.finalize();
    let view = |array, pointer_word, width| View {
        array,
        pointer_word,
        length_word: pointer_word + 1,
        width,
        literal: None,
    };
    (ctx, header, loads, vec![view(0, 3, 1)], vec![view(1, 5, 4)])
}
fn compile(mode: u8) -> (JITModule, Run) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let (mut ctx, header, loads, input, bins) = fixture(&module, 0);
    if mode != 0 {
        let mut fast = module.make_context();
        fast.func = ctx.func.clone();
        let before = fast
            .func
            .layout
            .blocks()
            .flat_map(|b| fast.func.layout.block_insts(b))
            .filter(|&inst| matches!(fast.func.dfg.insts[inst], Data::Brif { .. }))
            .count();
        let grouped = mode == 3 || mode == 5;
        let vector = mode == 4 || mode == 5;
        let admitted = if grouped {
            optimize_grouped_counter_capacity(&mut fast, header, &loads, &input, &bins)
        } else if mode >= 2 {
            optimize_counter_capacity(&mut fast, header, &loads, &input, &bins)
        } else {
            optimize(&mut fast, header, &loads, &input, &bins)
        }
        .expect("bounded histogram");
        let after = fast
            .func
            .layout
            .blocks()
            .flat_map(|b| fast.func.layout.block_insts(b))
            .filter(|&inst| matches!(fast.func.dfg.insts[inst], Data::Brif { .. }))
            .count();
        assert_eq!(before - after, if mode >= 2 && !grouped { 5 } else { 4 });
        assert_eq!(
            admitted,
            Admission {
                input_word: 3,
                bin_word: 5,
                begin_word: 0,
                end_word: 1,
                cursor_word: 2
            }
        );
        fast.verify(module.isa()).unwrap();
        let id = module
            .declare_function("bounded", Linkage::Local, &fast.func.signature)
            .unwrap();
        module.define_function(id, &mut fast).unwrap();
        let call = module.declare_func_in_func(id, &mut ctx.func);
        if vector {
            guard_vector_counter_capacity(&mut ctx, admitted, call);
        } else if mode >= 2 {
            guard_counter_capacity(&mut ctx, admitted, call);
        } else {
            guard_entry(&mut ctx, admitted, call);
        }
    }
    ctx.verify(module.isa()).unwrap();
    let id = module
        .declare_function("histogram", Linkage::Local, &ctx.func.signature)
        .unwrap();
    module.define_function(id, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();
    // The declared three-word ABI returns one status word; the module remains alive.
    let run = unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(id)) };
    (module, run)
}
fn oracle(input: &[u8], bins: &mut [u32], begin: u64, end: u64, mut at: u64) -> (u64, u64) {
    while at < end {
        let Some(position) = begin.checked_add(at) else {
            return (1, 100);
        };
        let byte = input.get(position as usize).copied().unwrap_or(0) as usize;
        let count = bins.get(byte).copied().unwrap_or(0);
        let Some(updated) = count.checked_add(1) else {
            return (2, 101);
        };
        let Some(slot) = bins.get_mut(byte) else {
            return (3, 102);
        };
        *slot = updated;
        at += 1;
    }
    (0, 0)
}
#[test]
fn bounded_byte_histogram_keeps_defaulting_reads_partial_writes_and_errors() {
    let entries: Vec<_> = (0..=5).map(compile).collect();
    let mut comparisons = 0;
    let bytes: Vec<u8> = (0..256).map(|i| (i as u8).wrapping_mul(197)).collect();
    for begin in [0, 1, 63, 255, 256, 257, u64::MAX] {
        for end in [0, 1, 7, 8, 63, 255, 256, 257] {
            for at in [0, 1, 7, 255, 256, 257, u64::MAX] {
                for length in [0, 1, 255, 256, 288] {
                    for seed in [0, u32::MAX - 1, u32::MAX] {
                        let mut expected = vec![seed; length];
                        let (status, origin) = oracle(&bytes, &mut expected, begin, end, at);
                        for run in entries.iter().map(|(_, run)| *run) {
                            let mut actual = vec![seed; length];
                            let packet = [
                                begin,
                                end,
                                at,
                                bytes.as_ptr() as u64,
                                bytes.len() as u64,
                                actual.as_mut_ptr() as u64,
                                length as u64,
                            ];
                            let mut observed_origin = 0;
                            // All buffers remain live and the packet describes exactly their extents.
                            let observed = unsafe { run(0, packet.as_ptr(), &mut observed_origin) };
                            assert_eq!(
                                (observed, observed_origin),
                                (status, origin),
                                "{begin}/{end}/{at}/{length}/{seed}"
                            );
                            assert_eq!(actual, expected);
                            comparisons += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(comparisons, 35280);
}

#[test]
fn histogram_counter_capacity_preserves_seeded_failure_prefixes_and_aliases() {
    let entries: Vec<_> = [0, 2, 3, 4, 5].into_iter().map(compile).collect();
    let bytes: Vec<u8> = (0..4096).map(|i| (i as u8).wrapping_mul(197)).collect();
    for seed_index in [0, 17, 255] {
        for room in [0, 1, 7, 16, 4096] {
            for (begin, end, at) in [(0, 4096, 0), (17, 4000, 9), (31, 2000, 1999)] {
                let mut initial = vec![3; 256];
                initial[seed_index] = u32::MAX - room;
                let mut expected = initial.clone();
                let status = oracle(&bytes, &mut expected, begin, end, at);
                for run in entries.iter().map(|(_, run)| *run) {
                    let mut actual = initial.clone();
                    let packet = [
                        begin,
                        end,
                        at,
                        bytes.as_ptr() as u64,
                        bytes.len() as u64,
                        actual.as_mut_ptr() as u64,
                        actual.len() as u64,
                    ];
                    let mut origin = 0;
                    // The immutable input and writable bins describe their live allocations.
                    let result = unsafe { run(0, packet.as_ptr(), &mut origin) };
                    assert_eq!((result, origin), status);
                    assert_eq!(actual, expected);
                }
            }
        }
    }
    // Extra admission reads do not mutate aliased source bytes. Each original
    // input read still observes all preceding source-order counter writes.
    for head in [0, 255, 65535, 0x00ff_ffff] {
        for seed in [0, u32::MAX - 1, u32::MAX] {
            for end in [0, 1, 7, 255, 256, 1024] {
                let mut initial: Vec<u32> = (0..256).map(|i| i * 257).collect();
                initial[0] = head;
                initial[255] = seed;
                let mut expected = initial.clone();
                let mut status = (0, 0);
                for at in 0..end as usize {
                    let byte = expected[at / 4].to_le_bytes()[at % 4] as usize;
                    let Some(updated) = expected[byte].checked_add(1) else {
                        status = (2, 101);
                        break;
                    };
                    expected[byte] = updated;
                }
                for run in entries.iter().map(|(_, run)| *run) {
                    let mut actual = initial.clone();
                    let packet = [
                        0,
                        end,
                        0,
                        actual.as_ptr() as u64,
                        1024,
                        actual.as_mut_ptr() as u64,
                        256,
                    ];
                    let mut origin = 0;
                    // Both views share the same live allocation; the ABI uses raw views.
                    let result = unsafe { run(0, packet.as_ptr(), &mut origin) };
                    assert_eq!((result, origin), status);
                    assert_eq!(actual, expected);
                }
            }
        }
    }
}

#[test]
fn histogram_counter_capacity_declines_mutations_extra_stores_and_calls() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for mode in 1..=9 {
        let (mut ctx, header, loads, input, bins) = fixture(&module, mode);
        let before = ctx.func.display().to_string();
        assert!(
            optimize_counter_capacity(&mut ctx, header, &loads, &input, &bins).is_none(),
            "mode {mode}"
        );
        assert_eq!(ctx.func.display().to_string(), before);
    }
    let (mut ctx, header, loads, input, bins) = fixture(&module, 0);
    let signature = module.make_signature();
    let id = module
        .declare_function("opaque", Linkage::Import, &signature)
        .unwrap();
    let call = module.declare_func_in_func(id, &mut ctx.func);
    let last = ctx
        .func
        .layout
        .last_inst(ctx.func.layout.entry_block().unwrap())
        .unwrap();
    let mut cursor = FuncCursor::new(&mut ctx.func);
    cursor.goto_inst(last);
    cursor.ins().call(call, &[]);
    let before = ctx.func.display().to_string();
    assert!(optimize_counter_capacity(&mut ctx, header, &loads, &input, &bins).is_none());
    assert_eq!(ctx.func.display().to_string(), before);
}

fn check_counter_capacity_entry_guard(vector: bool) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let constant = |module: &JITModule, value| {
        let mut ctx = module.make_context();
        ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
        ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
        let entry = block(&mut ctx.func);
        for _ in 0..3 {
            ctx.func.dfg.append_block_param(entry, types::I64);
        }
        let mut c = FuncCursor::new(&mut ctx.func);
        c.goto_bottom(entry);
        let result = c.ins().iconst(types::I64, value);
        c.ins().return_(&[result]);
        ctx
    };
    let mut fast = constant(&module, 2);
    let id = module
        .declare_function("admitted", Linkage::Local, &fast.func.signature)
        .unwrap();
    module.define_function(id, &mut fast).unwrap();
    let mut ctx = constant(&module, 1);
    let call = module.declare_func_in_func(id, &mut ctx.func);
    let admission = Admission {
        input_word: 3,
        bin_word: 5,
        begin_word: 0,
        end_word: 1,
        cursor_word: 2,
    };
    if vector {
        guard_vector_counter_capacity(&mut ctx, admission, call);
    } else {
        guard_counter_capacity(&mut ctx, admission, call);
    }
    ctx.verify(module.isa()).unwrap();
    let id = module
        .declare_function("guard", Linkage::Local, &ctx.func.signature)
        .unwrap();
    module.define_function(id, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();
    // Both targets return a tag and never read the source view or write bins.
    let run = unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(id)) };
    for begin in [0, 17, u64::MAX] {
        for end in [0, 1, 100, u32::MAX as u64, u32::MAX as u64 + 1, u64::MAX] {
            for at in [0, 1, 99, 100, u64::MAX] {
                for input_len in [0, 4096, u64::MAX] {
                    for length in [0, 255, 256] {
                        for seed in [0, u32::MAX - 1, u32::MAX] {
                            let mut bins = vec![0u32; 256];
                            bins[255] = seed;
                            let admitted = begin <= input_len
                                && end <= input_len - begin
                                && at <= end
                                && length >= 256
                                && end - at <= u32::MAX as u64
                                && (end == at || seed as u64 <= u32::MAX as u64 - (end - at));
                            let packet = [
                                begin,
                                end,
                                at,
                                0,
                                input_len,
                                bins.as_mut_ptr() as u64,
                                length,
                            ];
                            let mut origin = 0;
                            // Only the real 256-bin allocation is read by the guard.
                            let result = unsafe { run(0, packet.as_ptr(), &mut origin) };
                            assert_eq!(
                                result,
                                if admitted { 2 } else { 1 },
                                "{begin}/{end}/{at}/{input_len}/{length}/{seed}"
                            );
                            assert_eq!(origin, 0);
                            assert_eq!(bins[255], seed);
                            assert!(bins[..255].iter().all(|&n| n == 0));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn histogram_counter_capacity_entry_guard_checks_last_bin_and_remaining_span() {
    check_counter_capacity_entry_guard(false);
    check_counter_capacity_entry_guard(true);
}
#[test]
fn bounded_byte_histogram_declines_other_strides_shifted_indices_and_changed_limits() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for mode in 1..=4 {
        let (mut ctx, header, loads, input, bins) = fixture(&module, mode);
        let before = ctx.func.display().to_string();
        assert!(
            optimize(&mut ctx, header, &loads, &input, &bins).is_none(),
            "mode {mode}"
        );
        assert_eq!(ctx.func.display().to_string(), before);
    }
}
