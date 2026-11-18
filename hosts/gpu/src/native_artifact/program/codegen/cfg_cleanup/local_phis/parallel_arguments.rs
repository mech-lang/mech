//! Prove equality of same-block arguments, including jointly carried loop
//! values. The function entry and opaque control arguments remain distinct.

use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{Block, Function, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(super) fn equivalents(f: &Function, cfg: &ControlFlowGraph) -> BTreeMap<Value, Value> {
    if f.dfg.values_labels.is_some() || !f.debug_tags.is_empty() {
        return BTreeMap::new();
    }
    let entry = f.layout.entry_block();
    let mut columns = BTreeMap::new();
    let mut representatives = BTreeMap::new();
    for block in f.layout.blocks() {
        let formals = f.dfg.block_params(block);
        if Some(block) == entry || formals.len() < 2 {
            continue;
        }
        let mut first_of_type = HashMap::new();
        let mut arguments = Vec::new();
        for (slot, &formal) in formals.iter().enumerate() {
            let incoming = super::incoming(f, cfg, block, slot)
                .map(|(_, value)| value)
                .collect::<Option<Vec<_>>>();
            let Some(incoming) = incoming.filter(|values| !values.is_empty()) else {
                continue;
            };
            let representative = *first_of_type
                .entry(f.dfg.value_type(formal))
                .or_insert(formal);
            representatives.insert(formal, representative);
            arguments.push((formal, incoming));
        }
        if first_of_type.len() < arguments.len() {
            columns.insert(block, arguments);
        }
    }

    let mut users = BTreeMap::<Value, BTreeSet<Block>>::new();
    for (&block, arguments) in &columns {
        for (_, values) in arguments {
            for &source in values {
                if representatives
                    .get(&source)
                    .is_some_and(|other| *other != source)
                {
                    users.entry(source).or_default().insert(block);
                }
            }
        }
    }

    // Refine each class by the correlated pair of inputs on every predecessor
    // edge. On an executed edge the inputs were defined earlier, so induction
    // also covers a class whose edges refer back to other members of the class.
    let mut pending = columns.keys().copied().collect::<BTreeSet<_>>();
    while let Some(block) = pending.pop_first() {
        let mut partitions = BTreeMap::new();
        let mut refined = Vec::new();
        for (formal, incoming) in &columns[&block] {
            let old = representatives[formal];
            let signature = incoming
                .iter()
                .map(|value| representatives.get(value).copied().unwrap_or(*value))
                .collect::<Vec<_>>();
            let new = *partitions.entry((old, signature)).or_insert(*formal);
            if old != new {
                refined.push((*formal, new));
            }
        }
        for (formal, representative) in refined {
            representatives.insert(formal, representative);
            if let Some(blocks) = users.get(&formal) {
                pending.extend(blocks.iter().copied());
            }
        }
    }
    representatives.retain(|formal, representative| formal != representative);
    representatives
}
