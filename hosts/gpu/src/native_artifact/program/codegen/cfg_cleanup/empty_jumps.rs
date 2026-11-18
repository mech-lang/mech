//! Bypass forward argument-free jumps before native register allocation.
use cranelift_codegen::ir::{Function, InstructionData};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::native_artifact) fn forward(function: &mut Function) -> usize {
    if function.preferred_fallthrough.is_some()
        || function.dfg.values_labels.is_some()
        || !function.debug_tags.is_empty()
    {
        return 0;
    }
    let blocks = function.layout.blocks().collect::<Vec<_>>();
    let entry = function.layout.entry_block();
    let mut later = BTreeSet::new();
    let mut forward = BTreeMap::new();
    for &block in blocks.iter().rev() {
        if Some(block) != entry
            && function.dfg.block_params(block).is_empty()
            && let Some(jump) = function.layout.first_inst(block)
            && function.layout.last_inst(block) == Some(jump)
            && let InstructionData::Jump { destination, .. } = function.dfg.insts[jump]
        {
            let target = destination.block(&function.dfg.value_lists);
            if later.contains(&target)
                && function.dfg.block_params(target).is_empty()
                && function.layout.is_cold(block) == function.layout.is_cold(target)
            {
                forward.insert(block, forward.get(&target).copied().unwrap_or(target));
            }
        }
        later.insert(block);
    }
    let mut redirected = 0;
    for block in blocks {
        let Some(inst) = function.layout.last_inst(block) else {
            continue;
        };
        let targets = match function.dfg.insts[inst] {
            InstructionData::Jump { destination, .. } => [Some(destination), None],
            InstructionData::Brif { blocks, .. } => blocks.map(Some),
            _ => continue,
        };
        for target in targets.into_iter().flatten() {
            let old = target.block(&function.dfg.value_lists);
            if let Some(&new) = forward.get(&old) {
                function.rewrite_branch_destination(inst, old, new);
                redirected += 1;
            }
        }
    }
    redirected
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::{
        ir::{AbiParam, InstBuilder, types},
        settings, verify_function,
    };
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    #[test]
    fn forwards_transitively_and_retains_the_cold_boundary() {
        let mut function = Function::new();
        function.signature.params.push(AbiParam::new(types::I8));
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut function, &mut state);
        let entry = b.create_block();
        let first = b.create_block();
        let second = b.create_block();
        let cold = b.create_block();
        let done = b.create_block();
        b.set_cold_block(cold);
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let input = b.block_params(entry)[0];
        b.ins().brif(input, first, &[], cold, &[]);
        for (block, next) in [(first, second), (second, done), (cold, done)] {
            b.switch_to_block(block);
            b.ins().jump(next, &[]);
        }
        b.switch_to_block(done);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();

        assert_eq!(forward(&mut function), 2);
        verify_function(&function, &settings::Flags::new(settings::builder())).unwrap();
        assert_eq!(
            function.block_successors(entry).collect::<Vec<_>>(),
            [done, cold]
        );
        assert_eq!(function.block_successors(cold).collect::<Vec<_>>(), [done]);
    }
}
