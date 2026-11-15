//! Give equivalent single-continuation bodies the same packet positions.
use super::{MAX_PACKET_WORDS, Traversal, region_instructions};
use cranelift_codegen::{
    dominator_tree::DominatorTree,
    flowgraph::ControlFlowGraph,
    ir::{Function, Opcode, Type, Value},
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(PartialEq, Eq, Hash)]
enum Step {
    Parameters(Vec<Type>),
    Instruction(Opcode, Vec<Type>),
}

struct Outputs {
    available: Vec<Option<Value>>,
    required: Vec<usize>,
}

fn outputs(
    function: &Function,
    dom: &DominatorTree,
    traversal: &Traversal,
) -> Option<(Vec<Step>, Outputs)> {
    let dfg = &function.dfg;
    let types = |values: &[Value]| values.iter().map(|&v| dfg.value_type(v)).collect();
    let mut shape = vec![Step::Parameters(types(&traversal.finish))];
    let mut available = traversal
        .finish
        .iter()
        .copied()
        .map(Some)
        .collect::<Vec<_>>();
    let mut previous = None;
    let mut dominates = false;
    for inst in region_instructions(function, traversal.candidate.start, &traversal.blocks) {
        let block = function.layout.inst_block(inst)?;
        if previous != Some(block) {
            previous = Some(block);
            dominates = dom.dominates(block, traversal.candidate.exit, &function.layout);
            let params = dfg.block_params(block);
            shape.push(Step::Parameters(types(params)));
            available.extend(params.iter().map(|&value| dominates.then_some(value)));
        }
        let values = dfg.inst_results(inst);
        shape.push(Step::Instruction(dfg.insts[inst].opcode(), types(values)));
        available.extend(values.iter().map(|&value| dominates.then_some(value)));
    }
    let positions = available
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.map(|value| (value, index)))
        .collect::<BTreeMap<_, _>>();
    let required = traversal
        .finish
        .iter()
        .chain(&traversal.outputs)
        .filter(|value| !traversal.passthrough.contains_key(value))
        .map(|value| positions.get(value).copied())
        .collect::<Option<_>>()?;
    Some((
        shape,
        Outputs {
            available,
            required,
        },
    ))
}

pub(super) fn bind(
    function: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    family: &mut [Traversal],
) {
    if family.len() < 2 {
        return;
    }
    let mut shapes = HashMap::<_, Vec<_>>::new();
    for (index, traversal) in family.iter().enumerate() {
        // Additional stores must use values defined on every normal edge in
        // the extracted region. A separate resume packet keeps its own ABI.
        if traversal.packed_inputs
            && traversal.candidate.resume.is_none()
            && cfg
                .pred_iter(traversal.candidate.exit)
                .all(|pred| traversal.blocks.contains(&pred.block))
            && let Some((shape, outputs)) = outputs(function, dom, traversal)
        {
            shapes.entry(shape).or_default().push((index, outputs));
        }
    }
    for group in shapes.into_values().filter(|group| group.len() > 1) {
        let required = &group[0].1.required;
        if group[1..]
            .iter()
            .all(|(_, outputs)| outputs.required == *required)
        {
            continue;
        }
        let union = group
            .iter()
            .flat_map(|(_, outputs)| outputs.required.iter().copied())
            .collect::<BTreeSet<_>>();
        if union.len() > MAX_PACKET_WORDS
            || group.iter().any(|(_, outputs)| {
                union
                    .iter()
                    .any(|&index| outputs.available[index].is_none())
            })
        {
            continue;
        }
        // Callers restore only their original values. The whole native body
        // and its relocations must still match before any caller changes.
        for (index, outputs) in group {
            family[index].aligned_outputs = union
                .iter()
                .map(|&position| outputs.available[position].unwrap())
                .collect();
        }
    }
}
