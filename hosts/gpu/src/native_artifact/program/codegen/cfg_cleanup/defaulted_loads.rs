//! Select a safe address for a defaulted read before issuing its only load.
use super::{
    composed_reads::{arm, read_comparison},
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
    context.flowgraph();
    let f = &mut context.func;
    let reads = f
        .layout
        .blocks()
        .filter_map(|block| find(f, &context.cfg, f.layout.last_inst(block)?))
        .collect::<Vec<_>>();
    if reads.is_empty() {
        return Ok(0);
    }

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

    let count = reads.len();
    for Read {
        branch,
        condition,
        load,
        address,
        compute_address,
        join,
    } in reads
    {
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
    Ok(count)
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
}
