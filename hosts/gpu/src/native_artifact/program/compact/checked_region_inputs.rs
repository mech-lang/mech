//! Align uses of captured values before comparing sibling region bodies.
use super::{MAX_PACKET_WORDS, Traversal, inputs, region_instructions};
use cranelift_codegen::ir::{Function, Inst, Opcode, Type, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

type Shape = Vec<(Opcode, Vec<Option<Type>>)>;
type Uses = Vec<(Inst, usize, Value)>;

fn capture_uses(function: &Function, traversal: &Traversal) -> (Shape, Uses) {
    let entry_parameters = function
        .dfg
        .block_params(traversal.candidate.start)
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let captures = traversal
        .inputs
        .iter()
        .copied()
        .filter(|value| !entry_parameters.contains(value))
        .collect::<BTreeSet<_>>();
    let mut uses = Vec::new();
    let mut shape = Vec::new();
    for inst in region_instructions(function, traversal.candidate.start, &traversal.blocks) {
        let roles = inputs(function, inst)
            .enumerate()
            .map(|(operand, value)| {
                let value = function.dfg.resolve_aliases(value);
                captures.contains(&value).then(|| {
                    uses.push((inst, operand, value));
                    function.dfg.value_type(value)
                })
            })
            .collect();
        shape.push((function.dfg.insts[inst].opcode(), roles));
    }
    (shape, uses)
}

pub(super) fn bind(function: &Function, family: &mut [Traversal]) {
    if family.len() < 2 {
        return;
    }
    let mut shapes = HashMap::<Shape, Vec<_>>::new();
    for (index, traversal) in family.iter().enumerate() {
        if traversal.packed_inputs {
            let (shape, uses) = capture_uses(function, traversal);
            shapes.entry(shape).or_default().push((index, uses));
        }
    }
    for members in shapes.into_values().filter(|members| members.len() > 1) {
        let mut roles = HashMap::new();
        let slots = (0..members[0].1.len())
            .map(|operand| {
                let column = members
                    .iter()
                    .map(|(_, uses)| uses[operand].2)
                    .collect::<Vec<_>>();
                let next = roles.len();
                *roles.entry(column).or_insert(next)
            })
            .collect::<Vec<_>>();
        let count = roles.len();
        // Split a capture only when at least one caller uses the same
        // value for roles that another caller keeps distinct.
        if members.iter().any(|(index, _)| {
            let traversal = &family[*index];
            let parameters = function.dfg.block_params(traversal.candidate.start);
            parameters.len() + count + traversal.slots.len() + traversal.literals.len()
                > MAX_PACKET_WORDS
        }) || members.iter().all(|(index, _)| {
            let traversal = &family[*index];
            function.dfg.block_params(traversal.candidate.start).len() + count
                == traversal.inputs.len()
        }) {
            continue;
        }
        for (index, uses) in members {
            let traversal = &mut family[index];
            let params = function.dfg.block_params(traversal.candidate.start);
            let mut arguments = params.to_vec();
            let mut bindings = BTreeMap::<Inst, Vec<Option<usize>>>::new();
            for ((inst, operand, value), slot) in uses.into_iter().zip(slots.iter().copied()) {
                let index = params.len() + slot;
                if index == arguments.len() {
                    arguments.push(value);
                }
                let at = bindings.entry(inst).or_default();
                if at.len() <= operand {
                    at.resize(operand + 1, None);
                }
                at[operand] = Some(index);
            }
            traversal.inputs = arguments;
            traversal.input_roles = bindings;
        }
    }
}
