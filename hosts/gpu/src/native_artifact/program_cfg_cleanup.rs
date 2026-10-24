//! Fold literal native branches after data optimization, then retire unreachable
//! blocks and their phi inputs before register allocation. No runtime predicate,
//! source operation, failure or work debit is assumed away.
use super::super::{NativeCompileError, compiler::backend, iteration_compiler::CfgCleanup};
use cranelift_codegen::{Context, ir::{Function, InstBuilder, InstructionData, Opcode, Value, ValueDef}, isa::TargetIsa};

const MAX_ROUNDS: usize = 8;

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EMISSIONS: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_cfg_cleanup<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore { fn drop(&mut self) { DISABLED.with(|v| v.set(self.0)); } }
    let _restore = Restore(DISABLED.with(|v| v.replace(true))); f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_cfg_cleanup_emissions() -> (usize, usize) {
    EMISSIONS.with(|v| v.replace((0, 0)))
}

fn counts(function: &Function) -> (usize, usize) {
    (function.layout.blocks().count(), function.layout.blocks().map(|block| function.layout.block_insts(block).count()).sum())
}
fn constant_condition(function: &Function, value: Value) -> Option<bool> {
    let value = function.dfg.resolve_aliases(value);
    let ty = function.dfg.value_type(value);
    if !ty.is_int() || ty.bits() > 64 { return None; }
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else { return None; };
    let InstructionData::UnaryImm { opcode: Opcode::Iconst, imm, .. } = function.dfg.insts[inst] else { return None; };
    // Iconst's immediate is sign extended; branch truth uses only the actual
    // integer type's low bits, including noncanonical nonzero boolean words.
    let mask = u64::MAX >> (64 - ty.bits());
    Some(imm.bits() as u64 & mask != 0)
}
fn fold(function: &mut Function) -> usize {
    let changes = function.layout.blocks().filter_map(|block| {
        let inst = function.layout.last_inst(block)?;
        let InstructionData::Brif { arg, blocks, .. } = &function.dfg.insts[inst] else { return None; };
        let selected = &blocks[usize::from(!constant_condition(function, *arg)?)];
        Some((inst, selected.block(&function.dfg.value_lists), selected.args(&function.dfg.value_lists).collect::<Vec<_>>()))
    }).collect::<Vec<_>>();
    for (inst, target, args) in &changes { function.dfg.replace(*inst).jump(*target, args); }
    changes.len()
}

pub(in crate::native_artifact) fn optimize(context: &mut Context, isa: &dyn TargetIsa) -> Result<CfgCleanup, NativeCompileError> {
    let (before_blocks, before_instructions) = counts(&context.func);
    #[cfg(not(test))]
    let enabled = true;
    #[cfg(test)]
    let enabled = !DISABLED.with(std::cell::Cell::get);
    let mut report = CfgCleanup { enabled, rounds: 0, constant_branches_removed: 0,
        before_blocks, before_instructions, after_blocks: before_blocks,
        after_instructions: before_instructions, fixed_point: !enabled };
    if !enabled { return Ok(report); }
    // Cranelift's ordinary speed pipeline runs data optimization after its
    // initial unreachable-block cleanup. Repeat only this bounded constant-CFG
    // cleanup; the ordinary final backend pipeline remains enabled as well.
    for _ in 0..MAX_ROUNDS {
        context.optimize(isa, &mut Default::default()).map_err(backend)?;
        report.rounds += 1;
        let removed = fold(&mut context.func);
        report.constant_branches_removed += removed;
        if removed == 0 { report.fixed_point = true; break; }
        // Recompute after edge replacement before using the backend's own
        // unreachable-block and constant-phi cleanup on the next round.
        context.compute_cfg(); context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
    }
    let (blocks, instructions) = counts(&context.func);
    report.after_blocks = blocks; report.after_instructions = instructions;
    #[cfg(test)]
    EMISSIONS.with(|v| { let old = v.get(); v.set((old.0 + 1, old.1 + report.constant_branches_removed)); });
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, MemFlags, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
    use cranelift_module::{Linkage, Module, default_libcall_names};
    #[test]
    fn cfg_cleanup_executes_only_selected_effects_and_keeps_dynamic_branches_and_block_arguments() {
        for (enabled, literal) in [false, true].into_iter().flat_map(|enabled| [0i64, -128].map(|literal| (enabled, literal))) {
            let mut module = cranelift_jit::JITModule::new(cranelift_jit::JITBuilder::with_flags(
                &[("opt_level", "speed")], default_libcall_names()).unwrap());
            let mut context = module.make_context();
            context.func.signature.params = vec![AbiParam::new(types::I64); 2];
            context.func.signature.returns = vec![AbiParam::new(types::I64)];
            let id = module.declare_function("constant_branch_effects", Linkage::Local, &context.func.signature).unwrap();
            let mut fc = FunctionBuilderContext::new();
            {
                let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
                let entry = b.create_block(); let live = b.create_block(); let dead = b.create_block();
                let yes = b.create_block(); let no = b.create_block(); let join = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.append_block_param(join, types::I64);
                b.switch_to_block(entry);
                let pointer = b.block_params(entry)[0]; let input = b.block_params(entry)[1];
                let constant = b.ins().iconst(types::I8, literal);
                b.ins().brif(constant, live, &[], dead, &[]);
                b.switch_to_block(dead);
                let bad = b.ins().iconst(types::I64, 999);
                b.ins().store(MemFlags::new(), bad, pointer, 0);
                b.ins().jump(join, &[bad.into()]);
                b.switch_to_block(live);
                b.ins().brif(input, yes, &[], no, &[]);
                b.switch_to_block(yes);
                let a = b.ins().iconst(types::I64, 11);
                b.ins().store(MemFlags::new(), a, pointer, 0);
                b.ins().jump(join, &[a.into()]);
                b.switch_to_block(no);
                let c = b.ins().iconst(types::I64, 22);
                b.ins().store(MemFlags::new(), c, pointer, 0);
                b.ins().jump(join, &[c.into()]);
                b.switch_to_block(join);
                let output = b.block_params(join)[0]; b.ins().return_(&[output]);
                b.seal_all_blocks(); b.finalize();
            }
            let report = if enabled { optimize(&mut context, module.isa()).unwrap() }
                else { without_cfg_cleanup(|| optimize(&mut context, module.isa())).unwrap() };
            assert_eq!(report.enabled, enabled);
            if enabled {
                assert!(report.fixed_point && report.constant_branches_removed > 0);
                assert!(report.after_blocks < report.before_blocks);
            }
            module.define_function(id, &mut context).unwrap(); module.finalize_definitions().unwrap();
            // SAFETY: this matching owned JIT function accepts exactly these
            // two words; the mutable output lives through each synchronous call.
            let entry: unsafe extern "C" fn(*mut u64, u64) -> u64 = unsafe { core::mem::transmute(module.get_finalized_function(id)) };
            for (condition, expected) in [(0,22), (1,11), (2,11), (u64::MAX,11)] {
                let expected = if literal == 0 { 999 } else { expected };
                let mut output = 0;
                assert_eq!(unsafe { entry(&mut output, condition) }, expected);
                assert_eq!(output, expected);
            }
        }
    }
}
