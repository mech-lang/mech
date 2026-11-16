//! Preserve differing literals at the caller of an otherwise repeated region.
use super::{MAX_DIRECT_INPUT_WORDS, MAX_PACKET_WORDS, Traversal, inputs, region_instructions};
use cranelift_codegen::ir::{
    Function, InstructionData, Opcode, Type, Value, ValueDef, condcodes::IntCC, types,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Clone, Copy)]
struct Literal {
    value: Value,
    ty: Type,
    bits: i64,
}

#[derive(Clone)]
pub(super) struct Capture {
    pub values: Vec<Value>,
    pub ty: Type,
    pub bits: i64,
}

pub(super) fn bind_repeated_bound(function: &Function, traversal: &mut Traversal) -> bool {
    let (_, literals) = literal_values(function, traversal);
    let constants = literals
        .iter()
        .filter(|literal| literal.ty == types::I64 && literal.bits > 1)
        .map(|literal| (literal.value, literal.bits))
        .collect::<BTreeMap<_, _>>();
    let mut bounds = BTreeMap::<i64, usize>::new();
    for inst in region_instructions(function, traversal.candidate.start, &traversal.blocks) {
        if let InstructionData::IntCompare {
            cond: IntCC::UnsignedLessThan,
            args: [_, limit],
            ..
        } = function.dfg.insts[inst]
            && let Some(&bits) = constants.get(&function.dfg.resolve_aliases(limit))
        {
            *bounds.entry(bits).or_default() += 1;
        }
    }
    let Some((&bits, &uses)) = bounds.iter().max_by_key(|(_, uses)| *uses) else {
        return false;
    };
    if uses < 2
        || bounds.values().filter(|&&count| count == uses).count() != 1
        || traversal.inputs.len() + traversal.slots.len() + traversal.literals.len()
            >= MAX_PACKET_WORDS
    {
        return false;
    }
    traversal.literals.push(Capture {
        values: constants
            .into_iter()
            .filter_map(|(value, constant)| (constant == bits).then_some(value))
            .collect(),
        ty: types::I64,
        bits,
    });
    true
}

type OperandShape = Vec<(Opcode, Vec<Option<(usize, Type)>>)>;

fn literal_values(function: &Function, traversal: &Traversal) -> (OperandShape, Vec<Literal>) {
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
                    candidates.push(Literal {
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

pub(super) fn bind(function: &Function, family: &mut [Traversal], merge_equal_columns: bool) {
    if family.len() < 2 {
        return;
    }
    let mut shapes = HashMap::<_, Vec<_>>::new();
    for (index, traversal) in family.iter().enumerate() {
        if traversal.packed_inputs || merge_equal_columns {
            let (shape, candidates) = literal_values(function, traversal);
            if !candidates.is_empty() {
                shapes
                    .entry((traversal.packed_inputs, shape))
                    .or_default()
                    .push((index, candidates));
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
        let mut columns = Vec::<Vec<usize>>::new();
        for index in differing {
            let matching = if merge_equal_columns {
                columns.iter_mut().find(|column| {
                    first[index].ty == first[column[0]].ty
                        && choices
                            .iter()
                            .all(|(_, literals)| literals[index].bits == literals[column[0]].bits)
                })
            } else {
                None
            };
            if let Some(column) = matching {
                column.push(index);
            } else {
                columns.push(vec![index]);
            }
        }
        if choices.iter().any(|(index, _)| {
            let traversal = &family[*index];
            let input_limit = if traversal.packed_inputs {
                MAX_PACKET_WORDS
            } else {
                MAX_DIRECT_INPUT_WORDS
            };
            traversal.inputs.len() + traversal.slots.len() + columns.len() > input_limit
        }) {
            continue;
        }
        // This is only a candidate ABI. Matching final native code and
        // relocations is still required before any caller is replaced.
        for (index, literals) in choices {
            family[index].literals.extend(columns.iter().map(|column| {
                let first = literals[column[0]];
                Capture {
                    values: column.iter().map(|&at| literals[at].value).collect(),
                    ty: first.ty,
                    bits: first.bits,
                }
            }));
        }
    }
}
