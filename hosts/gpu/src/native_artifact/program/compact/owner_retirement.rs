//! Share complete owner retirements without moving their checked exits.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::{
    flowgraph::ControlFlowGraph,
    ir::{
        Block, BlockCall, ExternalName, Function, Inst, InstructionData as Data, MemFlags, Opcode,
        types,
    },
};
use cranelift_module::{FuncId, FuncOrDataId, Linkage, Module};

struct Retirement {
    removed: [Inst; 3],
    branch: Inst,
    release: BlockCall,
    call: Inst,
}

fn final_edge(f: &Function, mut edge: BlockCall) -> BlockCall {
    let dfg = &f.dfg;
    // Follow only empty forwarding blocks. Arguments of a real join can
    // observe whether the owner was retired, so compare them at that join.
    for _ in 0..4 {
        let next = edge.block(&dfg.value_lists);
        if !dfg.block_params(next).is_empty() {
            break;
        }
        let Some(only) = f.layout.first_inst(next) else {
            break;
        };
        if f.layout.next_inst(only).is_some() {
            break;
        }
        if let Data::Jump { destination, .. } = dfg.insts[only] {
            edge = destination;
        } else {
            break;
        }
    }
    edge
}

fn retirement(
    f: &Function,
    cfg: &ControlFlowGraph,
    uses: &[u32],
    block: Block,
    free: FuncId,
) -> Option<Retirement> {
    let dfg = &f.dfg;
    let branch = f.layout.last_inst(block)?;
    let Data::Brif {
        arg,
        blocks: [live, release],
        ..
    } = dfg.insts[branch]
    else {
        return None;
    };
    let store = f.layout.prev_inst(branch)?;
    let decrement = f.layout.prev_inst(store)?;
    let load = f.layout.prev_inst(decrement)?;
    if f.layout.first_inst(block) != Some(load) {
        return None;
    }
    for (inst, opcode) in [(load, Opcode::Load), (store, Opcode::Store)] {
        let data = &dfg.insts[inst];
        if data.opcode() != opcode
            || data.load_store_offset() != Some(0)
            || data.memflags() != Some(MemFlags::new())
        {
            return None;
        }
    }
    let &[owner] = dfg.inst_args(load) else {
        return None;
    };
    let &[old] = dfg.inst_results(load) else {
        return None;
    };
    let Data::BinaryImm64 {
        opcode: Opcode::IaddImm,
        arg: prior,
        imm,
    } = dfg.insts[decrement]
    else {
        return None;
    };
    let &[remaining] = dfg.inst_results(decrement) else {
        return None;
    };
    let resolve = |value| dfg.resolve_aliases(value);
    if dfg.value_type(old) != types::I64
        || imm.bits() != -1
        || resolve(prior) != old
        || resolve(arg) != remaining
        || !dfg
            .inst_args(store)
            .iter()
            .copied()
            .map(resolve)
            .eq([remaining, resolve(owner)])
        || uses[old.as_u32() as usize] != 1
        || uses[remaining.as_u32() as usize] != 2
    {
        return None;
    }

    let retiring = release.block(&dfg.value_lists);
    if !dfg.block_params(retiring).is_empty()
        || !cfg.pred_iter(retiring).map(|pred| pred.inst).eq([branch])
    {
        return None;
    }
    let call = f.layout.first_inst(retiring)?;
    let Data::Call {
        opcode: Opcode::Call,
        func_ref,
        ..
    } = dfg.insts[call]
    else {
        return None;
    };
    let ExternalName::User(name) = dfg.ext_funcs[func_ref].name else {
        return None;
    };
    let name = &f.params.user_named_funcs()[name];
    let &[_, release_owner] = dfg.inst_args(call) else {
        return None;
    };
    let &[status] = dfg.inst_results(call) else {
        return None;
    };
    if name.namespace != 0
        || name.index != free.as_u32()
        || resolve(owner) != resolve(release_owner)
    {
        return None;
    }
    let next = f.layout.next_inst(call)?;
    let Data::Brif {
        arg: tested,
        blocks: [_, success],
        ..
    } = dfg.insts[next]
    else {
        return None;
    };
    if resolve(tested) != status {
        return None;
    }
    let held = final_edge(f, live);
    let freed = final_edge(f, success);
    if held.block(&dfg.value_lists) != freed.block(&dfg.value_lists)
        || !held.args(&dfg.value_lists).eq(freed.args(&dfg.value_lists))
    {
        return None;
    }
    Some(Retirement {
        removed: [load, decrement, store],
        branch,
        release,
        call,
    })
}

pub(super) fn share<M: Module>(
    module: &mut M,
    function: &mut Function,
) -> Result<usize, NativeCompileError> {
    let Some(FuncOrDataId::Func(free)) = module.get_name("mech_compact_free") else {
        return Ok(0);
    };
    let cfg = ControlFlowGraph::with_function(function);
    let mut uses = vec![0u32; function.dfg.num_values()];
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            for value in function.dfg.inst_values(inst) {
                uses[function.dfg.resolve_aliases(value).as_u32() as usize] += 1;
            }
        }
    }
    let retirements = function
        .layout
        .blocks()
        .filter_map(|block| retirement(function, &cfg, &uses, block, free))
        .collect::<Vec<_>>();
    if !retirements.is_empty() {
        let signature = module
            .declarations()
            .get_function_decl(free)
            .signature
            .clone();
        let provider = module
            .declare_function("mech_compact_release_owner", Linkage::Import, &signature)
            .map_err(backend)?;
        let provider = module.declare_func_in_func(provider, function);
        for item in &retirements {
            let Data::Call { func_ref, .. } = &mut function.dfg.insts[item.call] else {
                unreachable!();
            };
            *func_ref = provider;
            function.dfg.insts[item.branch] = Data::Jump {
                opcode: Opcode::Jump,
                destination: item.release,
            };
            for inst in item.removed {
                function.layout.remove_inst(inst);
            }
        }
    }
    Ok(retirements.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, InstBuilder};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_jit::{JITBuilder, JITModule};
    use cranelift_module::default_libcall_names;

    unsafe extern "C" fn free(heap: *mut [u64; 2], count: *mut u64) -> u64 {
        assert_eq!(unsafe { *count }, 0);
        let heap = unsafe { &mut *heap };
        heap[0] += 1;
        heap[1]
    }

    unsafe extern "C" fn release(heap: *mut [u64; 2], count: *mut u64) -> u64 {
        unsafe { *count = (*count).wrapping_sub(1) };
        if unsafe { *count } == 0 {
            unsafe { free(heap, count) }
        } else {
            0
        }
    }

    #[test]
    fn shares_only_when_both_successes_keep_the_same_continuation_words() {
        let mut jit = JITBuilder::new(default_libcall_names()).unwrap();
        jit.symbol("mech_compact_free", free as *const u8);
        jit.symbol("mech_compact_release_owner", release as *const u8);
        let mut module = JITModule::new(jit);
        let mut helper = module.make_signature();
        helper.params = vec![AbiParam::new(types::I64); 2];
        helper.returns = vec![AbiParam::new(types::I64)];
        let free = module
            .declare_function("mech_compact_free", Linkage::Import, &helper)
            .unwrap();
        let mut signature = helper.clone();
        signature.returns.push(AbiParam::new(types::I64));
        let mut entries = vec![];
        for (name, same, apply) in [
            ("original", true, false),
            ("shared", true, true),
            ("distinct", false, true),
        ] {
            let mut context = module.make_context();
            context.func.signature = signature.clone();
            let mut builder = FunctionBuilderContext::new();
            let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
            let entry = b.create_block();
            let dec = b.create_block();
            let dealloc = b.create_block();
            let next = b.create_block();
            let done = b.create_block();
            let error = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.append_block_param(done, types::I64);
            b.append_block_param(error, types::I64);
            b.switch_to_block(entry);
            let [heap, owner] = *b.block_params(entry) else {
                unreachable!()
            };
            let zero = b.ins().iconst(types::I64, 0);
            let read = if same {
                zero
            } else {
                b.ins().iconst(types::I64, 1)
            };
            b.ins().brif(owner, dec, &[], done, &[zero.into()]);
            b.switch_to_block(dec);
            let old = b.ins().load(types::I64, MemFlags::new(), owner, 0);
            let remaining = b.ins().iadd_imm(old, -1);
            b.ins().store(MemFlags::new(), remaining, owner, 0);
            b.ins().brif(remaining, done, &[read.into()], dealloc, &[]);
            b.switch_to_block(dealloc);
            let provider = module.declare_func_in_func(free, b.func);
            let call = b.ins().call(provider, &[heap, owner]);
            let status = b.inst_results(call)[0];
            b.ins().brif(status, error, &[status.into()], next, &[]);
            b.switch_to_block(next);
            b.ins().jump(done, &[zero.into()]);
            b.switch_to_block(done);
            let output = b.block_params(done)[0];
            b.ins().return_(&[zero, output]);
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            let origin = b.ins().iconst(types::I64, 79);
            b.ins().return_(&[status, origin]);
            b.seal_all_blocks();
            b.finalize();
            if apply {
                assert_eq!(
                    share(&mut module, &mut context.func).unwrap(),
                    usize::from(same)
                );
            }
            let id = module
                .declare_function(name, Linkage::Local, &signature)
                .unwrap();
            module.define_function(id, &mut context).unwrap();
            entries.push((id, same));
        }
        module.finalize_definitions().unwrap();
        #[derive(Debug, Eq, PartialEq)]
        #[repr(C)]
        struct ResultPacket(u64, u64);
        for (id, same) in entries {
            let entry: unsafe extern "C" fn(*mut [u64; 2], *mut u64) -> ResultPacket =
                unsafe { std::mem::transmute(module.get_finalized_function(id)) };
            let mut heap = [0, 17];
            assert_eq!(
                unsafe { entry(&mut heap, std::ptr::null_mut()) },
                ResultPacket(0, 0)
            );
            let mut count = 2;
            assert_eq!(
                unsafe { entry(&mut heap, &mut count) },
                ResultPacket(0, u64::from(!same))
            );
            assert_eq!((count, heap[0]), (1, 0));
            assert_eq!(
                unsafe { entry(&mut heap, &mut count) },
                ResultPacket(17, 79)
            );
            assert_eq!((count, heap[0]), (0, 1));
            count = 1;
            heap[1] = 0;
            assert_eq!(unsafe { entry(&mut heap, &mut count) }, ResultPacket(0, 0));
            assert_eq!((count, heap[0]), (0, 2));
        }
    }
}
