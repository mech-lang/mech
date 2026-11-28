//! A source-closed pure point expression within the current numeric loan.
//! No owner or loop permission is manufactured here. A failed speculative
//! prefix enters its unchanged lexical source before any scalar export/store.
use crate::native_artifact::codegen::common::ssa_words::Words;
use crate::native_artifact::codegen::options::OptimizationOptions;
use crate::native_artifact::compiler::f64_arithmetic;
use crate::native_artifact::program::prepare::readonly_numeric::{Operation, Proof};
use cranelift_codegen::ir::{Block, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeMatchTerminator as Branch, NativeProgramFacts,
    NativeProgramReadRole as Read, NativeScalarKind as Kind, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
enum Step {
    Copy(u32),
    Primitive(Op, Vec<u32>),
    Read {
        array: usize,
        row: u32,
        column: u32,
    },
    LinearRead {
        array: usize,
        index: u32,
        schema: mech_core::SchemaBody,
    },
}
#[derive(Clone, Debug)]
pub(in crate::native_artifact) struct Prefix {
    pub first: usize,
    pub next: u32,
    pub sites: BTreeSet<usize>,
    pub read_sites: Vec<usize>,
    pub exports: Vec<u32>,
    inputs: BTreeMap<u32, Kind>,
    kinds: BTreeMap<u32, Kind>,
    steps: Vec<(u32, Step)>,
}
fn scalar(op: Op) -> bool {
    matches!(
        op,
        Op::U64Add
            | Op::U64Sub
            | Op::U64Mul
            | Op::F64Add
            | Op::F64Sub
            | Op::F64Mul
            | Op::F64Div
            | Op::F64Neg
            | Op::F64Sqrt
    )
}
fn selected_read(
    facts: &NativeProgramFacts<'_>,
    proof: &Proof,
    site: usize,
) -> Option<(u32, Step)> {
    use mech_engine::native_artifact::NativeCollectionPrimitive as Primitive;
    let (array, output, primitive, selectors, step) = match proof.action(site)? {
        Operation::CellRead {
            array,
            row,
            column,
            output,
        } => (
            *array,
            *output,
            Primitive::MatrixCellAccess,
            vec![*row, *column],
            Step::Read {
                array: *array,
                row: *row,
                column: *column,
            },
        ),
        Operation::Read {
            array,
            index,
            output,
        } => {
            let Action::LibraryCall {
                template: witness, ..
            } = facts.plan().blocks()[proof.arrays[*array].witness].action()
            else {
                return None;
            };
            (
                *array,
                *output,
                Primitive::MatrixScalarAccess,
                vec![*index],
                Step::LinearRead {
                    array: *array,
                    index: *index,
                    schema: witness.inputs().first()?.body().clone(),
                },
            )
        }
        _ => return None,
    };
    let Action::LibraryCall {
        template,
        operands,
        output: selected,
    } = facts.plan().blocks()[site].action()
    else {
        return None;
    };
    (proof.arrays[array].width == 8
        && facts.plan().values()[output as usize].scalar_kind() == Some(Kind::F64)
        && template.primitive() == primitive
        && selected.get() == output
        && operands.len() == selectors.len() + 1
        && operands[1..].iter().map(|v| v.get()).eq(selectors)
        && template.output().body()
            == &mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64))
        .then_some((output, step))
}
fn plain(facts: &NativeProgramFacts<'_>, proof: &Proof, site: usize) -> bool {
    if proof.action(site).is_some() {
        return selected_read(facts, proof, site).is_some();
    }
    match facts.plan().blocks()[site].action() {
        Action::Scalar(Scalar::None) => true,
        Action::Scalar(Scalar::Copy { output, .. }) => matches!(
            facts.plan().values()[output.get() as usize].scalar_kind(),
            Some(Kind::U64 | Kind::F64)
        ),
        Action::Scalar(Scalar::Primitive { op, .. }) => scalar(*op),
        _ => false,
    }
}
fn successor(facts: &NativeProgramFacts<'_>, at: usize) -> Option<u32> {
    match facts.plan().blocks().get(at)?.terminator() {
        Term::Scalar(Branch::Jump(next)) => Some(next),
        _ => None,
    }
}
fn find(
    facts: &NativeProgramFacts<'_>,
    proof: &Proof,
    read: usize,
    available: &BTreeSet<usize>,
    forbidden: &BTreeSet<usize>,
) -> Option<Prefix> {
    let plan = facts.plan();
    let iteration = plan.blocks()[read].iteration();
    let eligible = |at: usize| {
        available.contains(&at)
            && !forbidden.contains(&at)
            && plan.blocks()[at].iteration() == iteration
            && plain(facts, proof, at)
    };
    let mut first = read;
    for _ in 0..128 {
        let [previous] = facts.predecessors(first as u32).ok()? else {
            break;
        };
        let previous = *previous as usize;
        if !eligible(previous) || successor(facts, previous)? != first as u32 {
            break;
        }
        first = previous;
    }
    let mut sites = BTreeSet::new();
    let mut steps = Vec::new();
    let mut kinds = BTreeMap::new();
    let mut produced = BTreeMap::new();
    let mut reads = Vec::new();
    let mut next = first as u32;
    while eligible(next as usize) {
        let at = next as usize;
        if sites.len() == 128 || !sites.insert(at) {
            return None;
        }
        let selected = match proof.action(at) {
            Some(_) => {
                let read = selected_read(facts, proof, at)?;
                reads.push(at);
                Some(read)
            }
            None => match plan.blocks()[at].action() {
                Action::Scalar(Scalar::None) => None,
                Action::Scalar(Scalar::Copy { source, output }) => {
                    Some((output.get(), Step::Copy(source.get())))
                }
                Action::Scalar(Scalar::Primitive {
                    op,
                    operands,
                    output,
                }) => Some((
                    output.get(),
                    Step::Primitive(*op, operands.iter().map(|v| v.get()).collect()),
                )),
                _ => return None,
            },
        };
        if let Some((output, step)) = selected {
            let source_value = match plan.blocks()[at].action() {
                Action::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. })
                | Action::LibraryCall { output, .. } => *output,
                _ => return None,
            };
            if source_value.get() != output {
                return None;
            }
            produced.insert(output, source_value);

            let [definition] = facts.definition_sites(output).ok()? else {
                return None;
            };
            if definition.site() != Some(at as u32)
                || kinds
                    .insert(output, plan.values()[output as usize].scalar_kind()?)
                    .is_some()
            {
                return None;
            }
            steps.push((output, step));
        }
        next = successor(facts, at)?;
        if facts.predecessors(next).ok()? != [at as u32] {
            break;
        }
    }
    if reads.is_empty() || reads.len() > 32 || sites.contains(&(next as usize)) {
        return None;
    }
    if proof
        .exits
        .iter()
        .any(|e| sites.contains(&e.from) || (e.to != first && sites.contains(&e.to)))
    {
        return None;
    }
    let mut inputs = BTreeMap::new();
    let mut defined = BTreeSet::new();
    for (output, step) in &steps {
        let operands = match step {
            Step::Copy(value) => vec![*value],
            Step::Primitive(_, values) => values.clone(),
            Step::Read { row, column, .. } => vec![*row, *column],
            Step::LinearRead { index, .. } => vec![*index],
        };
        for input in operands {
            if !defined.contains(&input) {
                if kinds.contains_key(&input) {
                    return None;
                }
                let kind = plan.values().get(input as usize)?.scalar_kind()?;
                if !matches!(kind, Kind::U64 | Kind::F64) {
                    return None;
                }
                inputs.insert(input, kind);
            }
        }
        defined.insert(*output);
    }
    let mut exports = Vec::new();
    for (output, _) in &steps {
        if facts.read_roles(produced[output]).ok()?.iter().any(|role|
            !matches!(role, Read::Action(site) | Read::Terminator(site) if sites.contains(&(*site as usize)))) {
            exports.push(*output);
        }
    }
    if exports.is_empty() || exports.len() > 8 || !exports.iter().any(|v| kinds[v] == Kind::F64) {
        return None;
    }
    // A load-only group exporting every F64 value removes no checked float
    // classification. Its speculative admission/fallback adds work and code;
    // retain the original direct native reads/control for such source regions.
    let checked_floats = reads.len()
        + steps
            .iter()
            .filter(|(output, step)| {
                kinds[output] == Kind::F64 && matches!(step, Step::Primitive(_, _))
            })
            .count();
    let escaping_floats = exports
        .iter()
        .filter(|output| kinds[output] == Kind::F64)
        .count();
    // Require at least one eliminated classification per escaping word; weak
    // partial closure otherwise duplicates admission and publication work.
    if checked_floats < 2 * escaping_floats {
        return None;
    }
    Some(Prefix {
        first,
        next,
        sites,
        read_sites: reads,
        exports,
        inputs,
        kinds,
        steps,
    })
}
pub(in crate::native_artifact) fn derive(
    facts: &NativeProgramFacts<'_>,
    proof: &Proof,
) -> Vec<Prefix> {
    let forbidden = super::numeric_read_packs::source_boundaries(facts, proof);
    let mut available = proof.sites.iter().copied().collect::<BTreeSet<_>>();
    let mut prefixes = Vec::new();
    for &site in &proof.sites {
        if available.contains(&site)
            && selected_read(facts, proof, site).is_some()
            && let Some(prefix) = find(facts, proof, site, &available, &forbidden)
        {
            for at in &prefix.sites {
                available.remove(at);
            }
            prefixes.push(prefix);
        }
    }
    prefixes
}

#[derive(Clone, Copy)]
struct View {
    pointer: Value,
    rows: Value,
    columns: Value,
    // Loaded only for actual source linear indexing. Cell-only regions retain
    // their existing current geometry and emit no extra shadow-word read.
    length: Option<Value>,
}
/// Evaluate only a proved prefix. Inputs are current raw source words; views
/// contain the already admitted current numeric bank and source geometry.
/// Do not export any SSA word until both admission and result guards succeed.
fn evaluate(
    b: &mut FunctionBuilder<'_>,
    options: OptimizationOptions,
    prefix: &Prefix,
    inputs: &BTreeMap<u32, Value>,
    views: &BTreeMap<usize, View>,
    slow: Block,
) -> Vec<(u32, Value)> {
    let mut integers = BTreeMap::new();
    let mut floats = BTreeMap::new();
    for (&id, &kind) in &prefix.inputs {
        match kind {
            Kind::U64 => {
                integers.insert(id, inputs[&id]);
            }
            Kind::F64 => {
                let value = b.ins().bitcast(types::F64, MemFlags::new(), inputs[&id]);
                floats.insert(id, value);
            }
            _ => unreachable!("proved scalar prefix input"),
        }
    }
    let mut valid = b.ins().iconst(types::I8, 1);
    let mut arithmetic = Vec::<(Op, Vec<Value>, Value)>::new();
    for (id, step) in &prefix.steps {
        if prefix.kinds[id] != Kind::U64 {
            continue;
        }
        let value = match step {
            Step::Copy(source) => integers[source],
            Step::Primitive(op, args) => {
                let args = args.iter().map(|v| integers[v]).collect::<Vec<_>>();
                if let Some((_, _, value)) = arithmetic
                    .iter()
                    .find(|(old, values, _)| old == op && *values == args)
                {
                    *value
                } else {
                    let [a, c] = args[..] else {
                        unreachable!("proved binary U64")
                    };
                    let (value, good) = match op {
                        Op::U64Add => {
                            let value = b.ins().iadd(a, c);
                            let good = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, value, a);
                            (value, good)
                        }
                        Op::U64Sub => {
                            let good = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, a, c);
                            let value = b.ins().isub(a, c);
                            (value, good)
                        }
                        Op::U64Mul => {
                            let high = b.ins().umulhi(a, c);
                            let good = b.ins().icmp_imm(IntCC::Equal, high, 0);
                            let value = b.ins().imul(a, c);
                            (value, good)
                        }
                        _ => unreachable!("proved U64 arithmetic"),
                    };
                    valid = b.ins().band(valid, good);
                    arithmetic.push((*op, args, value));
                    value
                }
            }
            _ => unreachable!("proved unsigned prefix value"),
        };
        integers.insert(*id, value);
    }
    let mut checked = BTreeSet::new();
    for (_, step) in &prefix.steps {
        if let Step::Read { array, row, column } = step {
            let View { rows, columns, .. } = views[array];
            for (axis, id, bound) in [(0, row, rows), (1, column, columns)] {
                let index = integers[id];
                if checked.insert((*array, axis, index)) {
                    let index = b.ins().iadd_imm(index, -1);
                    let good = b.ins().icmp(IntCC::UnsignedLessThan, index, bound);
                    valid = b.ins().band(valid, good);
                }
            }
        }
        if let Step::LinearRead { array, index, .. } = step {
            let index = integers[index];
            if checked.insert((*array, 2, index)) {
                let ordinal = b.ins().iadd_imm(index, -1);
                let good = b.ins().icmp(
                    IntCC::UnsignedLessThan,
                    ordinal,
                    views[array]
                        .length
                        .expect("source linear read retains current bank length"),
                );
                valid = b.ins().band(valid, good);
            }
        }
    }
    let admitted = b.create_block();
    b.ins().brif(valid, admitted, &[], slow, &[]);
    b.switch_to_block(admitted);
    for (id, step) in &prefix.steps {
        if prefix.kinds[id] != Kind::F64 {
            continue;
        }
        let value = match step {
            Step::Copy(source) => floats[source],
            Step::Read { array, row, column } => {
                let View {
                    pointer, columns, ..
                } = views[array];
                let row = b.ins().iadd_imm(integers[row], -1);
                let column = b.ins().iadd_imm(integers[column], -1);
                let base = b.ins().imul(row, columns);
                let physical = b.ins().iadd(base, column);
                let bytes = b.ins().ishl_imm(physical, 3);
                let address = b.ins().iadd(pointer, bytes);
                b.ins().load(
                    types::F64,
                    super::numeric_words::guarded_load_flags(options),
                    address,
                    0,
                )
            }
            Step::LinearRead {
                array,
                index,
                schema,
            } => {
                let ordinal = b.ins().iadd_imm(integers[index], -1);
                let physical = super::numeric_words::linear_ordinal(
                    b,
                    ordinal,
                    views[array].rows,
                    views[array].columns,
                    schema,
                );
                let bytes = b.ins().ishl_imm(physical, 3);
                let address = b.ins().iadd(views[array].pointer, bytes);
                b.ins().load(
                    types::F64,
                    super::numeric_words::guarded_load_flags(options),
                    address,
                    0,
                )
            }
            Step::Primitive(op, args) => {
                let args = args.iter().map(|v| floats[v]).collect::<Vec<_>>();
                match (op, args.as_slice()) {
                    (Op::F64Neg, [v]) => b.ins().fneg(*v),
                    (Op::F64Sqrt, [v]) => b.ins().sqrt(*v),
                    (op, [a, c]) => f64_arithmetic(b, *op, *a, *c),
                    _ => unreachable!("proved scalar F64 prefix"),
                }
            }
        };
        floats.insert(*id, value);
    }
    let mut bad = b.ins().iconst(types::I8, 0);
    for id in &prefix.exports {
        if prefix.kinds[id] == Kind::F64 {
            let mask = super::numeric_product_epilogues::nonfinite_mask(b, floats[id], false);
            bad = b.ins().bor(bad, mask);
        }
    }
    let complete = b.create_block();
    b.ins().brif(bad, slow, &[], complete, &[]);
    b.switch_to_block(complete);
    prefix
        .exports
        .iter()
        .map(|id| {
            let value = if prefix.kinds[id] == Kind::F64 {
                b.ins().bitcast(types::I64, MemFlags::new(), floats[id])
            } else {
                integers[id]
            };
            (*id, value)
        })
        .collect()
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    options: OptimizationOptions,
    words: &Words,
    shadow: usize,
    proof: &Proof,
    prefix: &Prefix,
    slow: Block,
    next: Block,
) {
    let inputs = prefix
        .inputs
        .keys()
        .map(|id| (*id, words.read(b, *id as usize)))
        .collect();
    let arrays = prefix
        .steps
        .iter()
        .filter_map(|(_, step)| match step {
            Step::Read { array, .. } | Step::LinearRead { array, .. } => Some(*array),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let views = arrays
        .into_iter()
        .map(|array| {
            let at = shadow + proof.view(array);
            (
                array,
                View {
                    pointer: words.read(b, at),
                    rows: words.read(b, at + 2),
                    columns: words.read(b, at + 3),
                    length: prefix.steps.iter().any(|(_, step)|
                        matches!(step, Step::LinearRead { array: selected, .. } if *selected == array))
                        .then(|| words.read(b, at + 1)),
                },
            )
        })
        .collect();
    let results = evaluate(b, options, prefix, &inputs, &views, slow);
    for (id, value) in results {
        words.write(b, id as usize, value);
    }
    b.ins().jump(next, &[]);
}
#[cfg(all(test, feature = "canonical-native-program-runtime"))]
#[path = "numeric_cell_prefix_tests.rs"]
mod tests;
