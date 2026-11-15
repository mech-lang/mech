//! Preserve differing literals at the caller of an otherwise repeated region.
use super::{MAX_PACKET_WORDS, Traversal, inputs, region_instructions};
use cranelift_codegen::ir::{Function, InstructionData, Opcode, Type, Value, ValueDef};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, Copy)]
pub(super) struct Capture {
    pub value: Value,
    pub ty: Type,
    pub bits: i64,
}

type OperandShape = Vec<(Opcode, Vec<Option<(usize, Type)>>)>;

fn literal_values(function: &Function, traversal: &Traversal) -> (OperandShape, Vec<Capture>) {
    let instructions = region_instructions(function, traversal.candidate.start, &traversal.blocks);
    // Return operands already have a checked status/origin contract. Leave
    // them and any definition with direct uses outside the region in place.
    let excluded = traversal
        .outputs
        .iter()
        .copied()
        .chain(
            instructions
                .iter()
                .copied()
                .filter(|&inst| function.dfg.insts[inst].opcode() == Opcode::Return)
                .flat_map(|inst| inputs(function, inst)),
        )
        .map(|value| function.dfg.resolve_aliases(value))
        .collect::<BTreeSet<_>>();
    let mut columns = BTreeMap::new();
    let mut candidates = Vec::new();
    let mut shape = Vec::new();
    for inst in instructions {
        let operands = inputs(function, inst)
            .map(|value| {
                let value = function.dfg.resolve_aliases(value);
                let ValueDef::Result(definition, _) = function.dfg.value_def(value) else {
                    return None;
                };
                let InstructionData::UnaryImm {
                    opcode: Opcode::Iconst,
                    imm,
                } = function.dfg.insts[definition]
                else {
                    return None;
                };
                let ty = function.dfg.value_type(value);
                if ty.bytes() > 8 || excluded.contains(&value) {
                    return None;
                }
                let index = *columns.entry(value).or_insert_with(|| {
                    let index = candidates.len();
                    candidates.push(Capture {
                        value,
                        ty,
                        bits: i64::from(imm),
                    });
                    index
                });
                Some((index, ty))
            })
            .collect();
        shape.push((function.dfg.insts[inst].opcode(), operands));
    }
    (shape, candidates)
}

pub(super) fn bind(function: &Function, family: &mut [Traversal]) {
    if family.len() < 2 {
        return;
    }
    let mut shapes = HashMap::<_, Vec<_>>::new();
    for (index, traversal) in family.iter().enumerate() {
        if traversal.packed_inputs {
            let (shape, candidates) = literal_values(function, traversal);
            if !candidates.is_empty() {
                shapes.entry(shape).or_default().push((index, candidates));
            }
        }
    }
    for choices in shapes.into_values().filter(|choices| choices.len() > 1) {
        let first = &choices[0].1;
        let differing = (0..first.len())
            .filter(|&at| {
                choices[1..]
                    .iter()
                    .any(|(_, captures)| first[at].bits != captures[at].bits)
            })
            .collect::<Vec<_>>();
        if choices.iter().any(|(index, _)| {
            let traversal = &family[*index];
            traversal.inputs.len() + traversal.slots.len() + differing.len() > MAX_PACKET_WORDS
        }) {
            continue;
        }
        // This is only a candidate ABI. Matching final native code and
        // relocations is still required before any caller is replaced.
        for (index, captures) in choices {
            family[index]
                .literals
                .extend(differing.iter().map(|&at| captures[at]));
        }
    }
}
