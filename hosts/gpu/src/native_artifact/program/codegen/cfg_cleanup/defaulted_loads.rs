//! Select a safe address for a defaulted read before issuing its only load.
use super::{
    composed_reads::{View, arm, defaulted_read, read_comparison},
    word_ranges::literal,
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, Opcode as Op, Value,
    condcodes::IntCC, types,
};
use cranelift_module::{DataDescription, Module};
use std::collections::BTreeMap;

#[path = "defaulted_loads/immutable.rs"]
mod immutable;

struct Read {
    branch: Inst,
    condition: Value,
    load: Inst,
    address: Value,
    compute_address: Vec<Inst>,
    join: Block,
}

fn find(f: &Function, cfg: &ControlFlowGraph, branch: Inst) -> Option<Read> {
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[branch] else {
        return None;
    };
    let cmp = read_comparison(f, arg)?;
    if !matches!(
        f.dfg.insts[cmp],
        Data::IntCompare {
            cond: IntCC::UnsignedLessThan,
            ..
        } | Data::IntCompareImm {
            cond: IntCC::UnsignedLessThan,
            ..
        }
    ) || blocks
        .iter()
        .any(|call| call.args(&f.dfg.value_lists).next().is_some())
    {
        return None;
    }
    let (join, value, selected, loads) = arm(f, cfg, blocks[0].block(&f.dfg.value_lists))?;
    let (other_join, default, _, default_loads) = arm(f, cfg, blocks[1].block(&f.dfg.value_lists))?;
    let &[load] = loads.as_slice() else {
        return None;
    };
    let Data::Load {
        opcode: Op::Uload8 | Op::Uload32,
        arg: address,
        offset,
        ..
    } = f.dfg.insts[load]
    else {
        return None;
    };
    if join != other_join
        || cfg.pred_iter(join).count() != 2
        || f.dfg.block_params(join).len() != 1
        || !default_loads.is_empty()
        || literal(f, default) != Some(0)
        || f.dfg.resolve_aliases(value) != f.dfg.first_result(load)
        || i32::from(offset) != 0
        || f.dfg.value_type(address) != types::I64
    {
        return None;
    }

    // Every arm block has one predecessor and ends with an unconditional
    // jump. Its address arithmetic is total and dominates the single load.
    let mut compute_address = Vec::new();
    let mut loaded = false;
    for block in selected {
        for inst in f.layout.block_insts(block) {
            if inst == load {
                loaded = true;
            } else if f.dfg.insts[inst].opcode() != Op::Jump {
                if loaded
                    || !matches!(
                        f.dfg.insts[inst].opcode(),
                        Op::Iconst | Op::Iadd | Op::IaddImm | Op::Isub | Op::ImulImm | Op::IshlImm
                    )
                {
                    return None;
                }
                compute_address.push(inst);
            }
        }
    }

    Some(Read {
        branch,
        condition: f.dfg.first_result(cmp),
        load,
        address,
        compute_address,
        join,
    })
}

pub(in crate::native_artifact) fn fold<M: Module>(
    module: &mut M,
    context: &mut Context,
) -> Result<usize, NativeCompileError> {
    fold_with_views(module, context, None, false).map(|(folded, _)| folded)
}

/// Only the private caller's fixed, immutable view loan may supply these
/// packet words. A matching source bound and address make the selected load
/// safe independently of control flow; the zero arm uses immutable data.
pub(in crate::native_artifact) fn fold_immutable<M: Module>(
    module: &mut M,
    context: &mut Context,
    packet_loads: &BTreeMap<Value, (Inst, usize)>,
    views: &[View],
    rematerialize: bool,
) -> Result<(usize, usize), NativeCompileError> {
    fold_with_views(module, context, Some((packet_loads, views)), rematerialize)
}

type ImmutableViews<'a> = (&'a BTreeMap<Value, (Inst, usize)>, &'a [View]);

fn fold_with_views<M: Module>(
    module: &mut M,
    context: &mut Context,
    immutable: Option<ImmutableViews<'_>>,
    rematerialize: bool,
) -> Result<(usize, usize), NativeCompileError> {
    context.flowgraph();
    let endianness = module.isa().endianness();
    let f = &mut context.func;
    let reads = f
        .layout
        .blocks()
        .filter_map(|block| {
            let branch = f.layout.last_inst(block)?;
            let read = find(f, &context.cfg, branch)?;
            let pure = immutable.and_then(|(loads, views)| {
                let matched = defaulted_read(f, &context.cfg, loads, views, branch, endianness)?;
                let Data::Load { flags, .. } = f.dfg.insts[read.load] else {
                    return None;
                };
                Some(immutable::Spec {
                    index: matched.index,
                    pointer_word: matched.view.pointer_word,
                    length_word: matched.view.length_word,
                    width: matched.view.width,
                    flags,
                })
            });
            // This private-loan pass changes only certified immutable reads.
            // Keep every other checked branch in place; the independent
            // ordinary safe-address fold can still be enabled separately.
            if immutable.is_some() && pure.is_none() {
                return None;
            }
            Some((read, pure))
        })
        .collect::<Vec<_>>();
    if reads.is_empty() {
        return Ok((0, 0));
    }
    let (groups, owners) = immutable.map_or_else(
        || (Vec::new(), vec![None; reads.len()]),
        |views| immutable::plan(f, &context.cfg, &context.domtree, &reads, views),
    );

    let object = module
        .declare_anonymous_data(false, false)
        .map_err(backend)?;
    let mut data = DataDescription::new();
    data.define(vec![0_u8; 8].into_boxed_slice());
    data.set_align(8);
    module.define_data(object, &data).map_err(backend)?;
    let global = module.declare_data_in_func(object, f);
    let entry = f
        .layout
        .entry_block()
        .expect("a checked read requires an entry");
    let mut cursor = FuncCursor::new(f);
    cursor.goto_first_inst(entry);
    let zero = cursor.ins().global_value(types::I64, global);
    let values = groups
        .iter()
        .map(|group| group.emit(&mut cursor, zero, rematerialize))
        .collect::<Vec<_>>();

    let count = reads.len();
    let immutable_count = owners.iter().flatten().count();
    for (
        (
            Read {
                branch,
                condition,
                load,
                address,
                compute_address,
                join,
            },
            _,
        ),
        owner,
    ) in reads.into_iter().zip(owners)
    {
        if let Some(owner) = owner {
            let loaded = values[owner];
            cursor.func.dfg.replace(branch).jump(join, &[loaded.into()]);
            continue;
        }
        for inst in compute_address {
            cursor.func.layout.remove_inst(inst);
            cursor.func.layout.insert_inst(inst, branch);
        }
        cursor.goto_inst(branch);
        cursor.use_srcloc(load);
        // Integer address calculation can wrap on the default path. The
        // wrapping address is never used there. The load keeps the original
        // flags and the admitted address whenever the check succeeds.
        let selected = cursor.ins().select(condition, address, zero);
        let Data::Load { arg, .. } = &mut cursor.func.dfg.insts[load] else {
            unreachable!("validated load shape");
        };
        *arg = selected;
        cursor.func.layout.remove_inst(load);
        cursor.func.layout.insert_inst(load, branch);
        let loaded = cursor.func.dfg.first_result(load);
        cursor.func.dfg.replace(branch).jump(join, &[loaded.into()]);
    }
    let isa = module.isa();
    context.flowgraph();
    context.eliminate_unreachable_code(isa).map_err(backend)?;
    context.verify(isa).map_err(backend)?;
    Ok((count, immutable_count))
}

#[cfg(all(test, feature = "jit", target_pointer_width = "64"))]
mod tests {
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
                        let byte =
                            |index: u64| bytes.get(index as usize).copied().unwrap_or(0) as u64;
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
}
