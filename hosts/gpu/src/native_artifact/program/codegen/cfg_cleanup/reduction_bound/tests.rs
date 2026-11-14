use super::*;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{AbiParam, MemFlags};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Module, default_libcall_names};

fn fixture(module: &JITModule, guarded: bool, seed: u64, unknown_reset: bool) -> Context {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut state);
    let entry = b.create_block();
    let header = b.create_block();
    let visit = b.create_block();
    let read = b.create_block();
    let finish = b.create_block();
    let checked = b.create_block();
    let join = b.create_block();
    let advance = b.create_block();
    for block in [header, join] {
        for _ in 0..3 {
            b.append_block_param(block, types::I64);
        }
    }
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let input = b.block_params(entry)[0];
    let length = b.block_params(entry)[1];
    let unknown = b.block_params(entry)[2];
    let zero = b.ins().iconst(types::I64, 0);
    let start = b.ins().iconst(types::I64, seed as i64);
    b.ins()
        .jump(header, &[zero.into(), start.into(), zero.into()]);

    b.switch_to_block(header);
    let position = b.block_params(header)[0];
    let prior = b.block_params(header)[1];
    let observed = b.block_params(header)[2];
    let remaining = b.ins().icmp(IntCC::UnsignedLessThan, position, length);
    b.ins().brif(remaining, visit, &[], finish, &[]);

    b.switch_to_block(finish);
    b.ins().return_(&[observed, prior]);

    b.switch_to_block(visit);
    let stride = b.ins().ishl_imm(position, 3);
    let address = b.ins().iadd(input, stride);
    let raw = b.ins().load(types::I64, MemFlags::new(), address, 0);
    if guarded {
        let adjusted = b.ins().iadd_imm(raw, -3);
        let valid = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, adjusted, 255);
        b.ins().brif(valid, checked, &[], finish, &[]);
    } else {
        b.ins().jump(checked, &[]);
    }

    b.switch_to_block(checked);
    let stop = b.ins().iadd_imm(raw, -2);
    let active = b.ins().icmp(IntCC::UnsignedLessThan, prior, stop);
    let span = b.ins().isub(stop, prior);
    b.ins().brif(active, read, &[], advance, &[]);

    b.switch_to_block(read);
    let ordinal = b.ins().iadd_imm(prior, 3);
    let sum = b.ins().iadd(observed, span);
    let sum = b.ins().iadd(sum, ordinal);
    let next = if unknown_reset { unknown } else { stop };
    b.ins()
        .jump(join, &[position.into(), next.into(), sum.into()]);

    b.switch_to_block(advance);
    b.ins()
        .jump(join, &[position.into(), prior.into(), observed.into()]);

    b.switch_to_block(join);
    let next = b.block_params(join)[0];
    let bound = b.block_params(join)[1];
    let observed = b.block_params(join)[2];
    let next = b.ins().iadd_imm(next, 1);
    b.ins()
        .jump(header, &[next.into(), bound.into(), observed.into()]);
    b.seal_all_blocks();
    b.finalize();
    context
}

#[test]
fn carries_the_current_bound_across_both_paths_and_decodes_the_exit_value() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut optimized = fixture(&module, true, 0, false);
    optimized.flowgraph();
    let f = &optimized.func;
    let entry = f.layout.entry_block().unwrap();
    let header = f.layout.next_block(entry).unwrap();
    let start = f.dfg.block_params(header)[1];
    let test = f
        .layout
        .blocks()
        .flat_map(|block| f.layout.block_insts(block))
        .find(|&inst| {
            matches!(f.dfg.insts[inst], Data::IntCompare { args: [left, _], .. } if left == start)
        })
        .unwrap();
    let old_stop = f.dfg.inst_args(test)[1];
    let (raw, _) = biased_base(f, old_stop).unwrap();
    assert!(carry(
        &mut optimized.func,
        &optimized.cfg,
        &optimized.domtree
    ));
    optimized.verify(module.isa()).unwrap();
    let f = &optimized.func;
    assert_eq!(f.dfg.inst_args(test), &[start, raw]);

    let argument = |inst, lane| {
        let Data::Jump { destination, .. } = f.dfg.insts[inst] else {
            panic!("expected a transfer");
        };
        destination
            .args(&f.dfg.value_lists)
            .nth(lane)
            .unwrap()
            .as_value()
            .unwrap()
    };
    let seed = argument(f.layout.last_inst(entry).unwrap(), 1);
    assert_eq!(literal(f, seed), Some(2));
    let transferred = f
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = f.layout.last_inst(block)?;
            match f.dfg.insts[inst] {
                Data::Jump { destination, .. } if destination.len(&f.dfg.value_lists) == 3 => {
                    Some(argument(inst, 1))
                }
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    assert!(transferred.contains(&raw));
    assert!(transferred.contains(&start));
    let returned = f
        .layout
        .blocks()
        .find_map(|block| {
            let inst = f.layout.last_inst(block)?;
            (f.dfg.insts[inst].opcode() == Op::Return).then(|| f.dfg.inst_args(inst)[1])
        })
        .unwrap();
    assert_eq!(constant_offset(f, returned), (start, u64::MAX - 1));
}

#[test]
fn declines_unproved_current_bounds_unknown_resets_and_overflowing_seeds() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for (guarded, seed, unknown) in [(false, 0, false), (true, 0, true), (true, u64::MAX, false)] {
        let mut context = fixture(&module, guarded, seed, unknown);
        context.flowgraph();
        context.verify(module.isa()).unwrap();
        let old = context.func.display().to_string();
        assert!(!carry(&mut context.func, &context.cfg, &context.domtree));
        assert_eq!(context.func.display().to_string(), old);
    }
}
