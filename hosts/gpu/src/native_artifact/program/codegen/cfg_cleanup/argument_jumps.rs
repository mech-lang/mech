//! Bypass jump-only argument transport without evaluating or cloning code.
use cranelift_codegen::ir::{
    Block, BlockArg, BlockCall, Function, InstructionData, Value, ValueDef,
};
use std::collections::{BTreeMap, BTreeSet};

struct Forward {
    target: Block,
    args: Vec<Value>,
}

fn arguments(function: &Function, call: BlockCall) -> Option<Vec<Value>> {
    call.args(&function.dfg.value_lists)
        .map(|arg| {
            let value = function.dfg.resolve_aliases(arg.as_value()?);
            (!matches!(function.dfg.value_def(value), ValueDef::Union(..))).then_some(value)
        })
        .collect()
}

fn typed(function: &Function, block: Block, args: &[Value]) -> bool {
    let params = function.dfg.block_params(block);
    params.len() == args.len()
        && params
            .iter()
            .zip(args)
            .all(|(&param, &arg)| function.dfg.value_type(param) == function.dfg.value_type(arg))
}

fn compose(
    function: &Function,
    via: Block,
    incoming: &[Value],
    forward: &Forward,
) -> Option<Vec<Value>> {
    if !typed(function, via, incoming) {
        return None;
    }
    let args = forward
        .args
        .iter()
        .map(|&value| match function.dfg.value_def(value) {
            ValueDef::Param(block, index) if block == via => incoming[index],
            _ => value,
        })
        .collect::<Vec<_>>();
    typed(function, forward.target, &args).then_some(args)
}

pub(in crate::native_artifact) fn forward(function: &mut Function) -> usize {
    if function.preferred_fallthrough.is_some()
        || function.dfg.values_labels.is_some()
        || !function.debug_tags.is_empty()
    {
        return 0;
    }
    let blocks = function.layout.blocks().collect::<Vec<_>>();
    let entry = function.layout.entry_block();
    let jumps = blocks
        .iter()
        .filter_map(|&block| {
            let inst = function.layout.first_inst(block)?;
            (Some(block) != entry
                && function.layout.last_inst(block) == Some(inst)
                && matches!(function.dfg.insts[inst], InstructionData::Jump { .. }))
            .then_some((block, inst))
        })
        .collect::<BTreeMap<_, _>>();
    // A parameter can be used directly in a later dominated block. Removing
    // its defining block would invalidate such uses even after edge arguments
    // were composed. Include branch arguments, aliases and exception contexts.
    let mut escaping = BTreeSet::new();
    for &block in &blocks {
        for inst in function.layout.block_insts(block) {
            for value in function.dfg.inst_values(inst) {
                let value = function.dfg.resolve_aliases(value);
                // Egraph unions can hide uses of transport parameters until
                // extraction. This pass only handles ordinary attached SSA.
                if matches!(function.dfg.value_def(value), ValueDef::Union(..)) {
                    return 0;
                }
                if let ValueDef::Param(owner, _) = function.dfg.value_def(value)
                    && jumps.get(&owner).is_some_and(|&jump| jump != inst)
                {
                    escaping.insert(owner);
                }
            }
        }
    }
    let mut later = BTreeSet::new();
    let mut forwards = BTreeMap::<Block, Forward>::new();
    for &block in blocks.iter().rev() {
        if !escaping.contains(&block)
            && let Some(&jump) = jumps.get(&block)
            && let InstructionData::Jump { destination, .. } = function.dfg.insts[jump]
        {
            let target = destination.block(&function.dfg.value_lists);
            if later.contains(&target)
                && function.layout.is_cold(block) == function.layout.is_cold(target)
                && let Some(args) = arguments(function, destination)
                && typed(function, target, &args)
            {
                let next = forwards.get(&target).and_then(|next| {
                    compose(function, target, &args, next).map(|args| Forward {
                        target: next.target,
                        args,
                    })
                });
                forwards.insert(block, next.unwrap_or(Forward { target, args }));
            }
        }
        later.insert(block);
    }
    let mut redirected = 0;
    for block in blocks {
        let Some(inst) = function.layout.last_inst(block) else {
            continue;
        };
        let mut data = function.dfg.insts[inst];
        let calls = match &mut data {
            InstructionData::Jump { destination, .. } => std::slice::from_mut(destination),
            InstructionData::Brif { blocks, .. } => blocks.as_mut_slice(),
            _ => continue,
        };
        for call in calls {
            let old = call.block(&function.dfg.value_lists);
            if let Some(next) = forwards.get(&old)
                && let Some(incoming) = arguments(function, *call)
                && let Some(args) = compose(function, old, &incoming, next)
            {
                *call = BlockCall::new(
                    next.target,
                    args.into_iter().map(BlockArg::Value),
                    &mut function.dfg.value_lists,
                );
                redirected += 1;
            }
        }
        function.dfg.insts[inst] = data;
    }
    redirected
}

#[cfg(test)]
#[path = "argument_jumps/tests.rs"]
mod tests;
