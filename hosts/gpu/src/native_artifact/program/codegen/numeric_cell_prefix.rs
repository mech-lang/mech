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
    Read { array: usize, row: u32, column: u32 },
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
fn supported(facts: &NativeProgramFacts<'_>, proof: &Proof, site: usize) -> bool {
    match proof.action(site) {
        Some(Operation::CellRead {
            array,
            row,
            column,
            output,
        }) => {
            proof.arrays[*array].width == 8
                && facts.plan().values()[*output as usize].scalar_kind() == Some(Kind::F64)
                && matches!(facts.plan().blocks()[site].action(), Action::LibraryCall { template, operands, output: selected }
                    if template.primitive() == mech_engine::native_artifact::NativeCollectionPrimitive::MatrixCellAccess
                    && selected.get() == *output && operands.len() == 3
                    && operands[1].get() == *row && operands[2].get() == *column
                    && template.output().body() == &mech_core::SchemaBody::FloatingPoint(mech_core::FloatWidth::W64))
        }
        _ => false,
    }
}
fn plain(facts: &NativeProgramFacts<'_>, proof: &Proof, site: usize) -> bool {
    if proof.action(site).is_some() {
        return supported(facts, proof, site);
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
            Some(Operation::CellRead {
                array,
                row,
                column,
                output,
            }) => {
                reads.push(at);
                Some((
                    *output,
                    Step::Read {
                        array: *array,
                        row: *row,
                        column: *column,
                    },
                ))
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
            _ => return None,
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
    if reads.len() < 2 || reads.len() > 32 || sites.contains(&(next as usize)) {
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
    if exports.len() != 1 || kinds[&exports[0]] != Kind::F64 {
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
            && matches!(proof.action(site), Some(Operation::CellRead { .. }))
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

fn admitted_indices(
    b: &mut FunctionBuilder<'_>,
    prefix: &Prefix,
    inputs: &BTreeMap<u32, Value>,
    views: &BTreeMap<usize, [Value; 3]>,
    slow: Block,
) -> BTreeMap<u32, Value> {
    let mut integers = prefix
        .inputs
        .iter()
        .filter(|(_, kind)| **kind == Kind::U64)
        .map(|(&id, _)| (id, inputs[&id]))
        .collect::<BTreeMap<_, _>>();
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
            let [_, rows, columns] = views[array];
            for (axis, id, bound) in [(0, row, rows), (1, column, columns)] {
                let index = integers[id];
                if checked.insert((*array, axis, index)) {
                    let index = b.ins().iadd_imm(index, -1);
                    let good = b.ins().icmp(IntCC::UnsignedLessThan, index, bound);
                    valid = b.ins().band(valid, good);
                }
            }
        }
    }
    let admitted = b.create_block();
    b.ins().brif(valid, admitted, &[], slow, &[]);
    b.switch_to_block(admitted);
    integers
}
fn point_load(
    b: &mut FunctionBuilder<'_>,
    options: OptimizationOptions,
    view: [Value; 3],
    row: Value,
    column: Value,
) -> Value {
    let [pointer, _, columns] = view;
    let row = b.ins().iadd_imm(row, -1);
    let column = b.ins().iadd_imm(column, -1);
    let base = b.ins().imul(row, columns);
    let index = b.ins().iadd(base, column);
    let bytes = b.ins().ishl_imm(index, 3);
    let address = b.ins().iadd(pointer, bytes);
    b.ins().load(
        types::F64,
        super::numeric_words::guarded_load_flags(options),
        address,
        0,
    )
}
struct Evaluated {
    words: Vec<(u32, Value)>,
    reused: Option<Value>,
}
/// A fresh point proves every source checked subexpression and source axis.
/// The rolling edge inherits that complete certificate only under its exact
/// source-carried unit-column continuation and remaining upper guards.
fn evaluate(
    b: &mut FunctionBuilder<'_>,
    options: OptimizationOptions,
    prefix: &Prefix,
    inputs: &BTreeMap<u32, Value>,
    views: &BTreeMap<usize, [Value; 3]>,
    slow: Block,
    cache: Option<&rolling::Cache>,
) -> Evaluated {
    let source_reads = prefix
        .steps
        .iter()
        .filter_map(|(id, step)| matches!(step, Step::Read { .. }).then_some(*id))
        .collect::<Vec<_>>();
    let (reads, reused) = if let Some(cache) = cache {
        let fresh = b.create_block();
        let cached = b.create_block();
        let join = b.create_block();
        b.set_cold_block(fresh);
        for _ in &source_reads {
            b.append_block_param(join, types::F64);
        }
        b.append_block_param(join, types::I8);
        let yes = cache.reusable(b, views[&cache.recipe.array][2]);
        b.ins().brif(yes, cached, &[], fresh, &[]);
        b.switch_to_block(cached);
        let mut values = Vec::new();
        for id in &source_reads {
            let key = cache.recipe.offsets[id];
            let value = if let Some(value) = cache.old_read(b, key) {
                value
            } else {
                let row = b.ins().iadd_imm(inputs[&cache.recipe.row], key.0);
                let column = b.ins().iadd_imm(inputs[&cache.recipe.column], key.1);
                point_load(b, options, views[&cache.recipe.array], row, column)
            };
            values.push(value.into());
        }
        let one = b.ins().iconst(types::I8, 1);
        values.push(one.into());
        b.ins().jump(join, &values);
        b.switch_to_block(fresh);
        let integers = admitted_indices(b, prefix, inputs, views, slow);
        let mut values = Vec::new();
        for (id, step) in &prefix.steps {
            if let Step::Read { array, row, column } = step {
                let value = point_load(b, options, views[array], integers[row], integers[column]);
                values.push(value.into());
                let _ = id;
            }
        }
        let zero = b.ins().iconst(types::I8, 0);
        values.push(zero.into());
        b.ins().jump(join, &values);
        b.switch_to_block(join);
        let parameters = b.block_params(join).to_vec();
        (
            source_reads
                .iter()
                .enumerate()
                .map(|(i, &id)| (id, parameters[i]))
                .collect::<BTreeMap<_, _>>(),
            Some(parameters[source_reads.len()]),
        )
    } else {
        let integers = admitted_indices(b, prefix, inputs, views, slow);
        (
            prefix
                .steps
                .iter()
                .filter_map(|(id, step)| {
                    if let Step::Read { array, row, column } = step {
                        Some((
                            *id,
                            point_load(b, options, views[array], integers[row], integers[column]),
                        ))
                    } else {
                        None
                    }
                })
                .collect(),
            None,
        )
    };
    let mut floats = prefix
        .inputs
        .iter()
        .filter_map(|(&id, &kind)| {
            (kind == Kind::F64).then(|| {
                (
                    id,
                    b.ins().bitcast(types::F64, MemFlags::new(), inputs[&id]),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    for (id, step) in &prefix.steps {
        if prefix.kinds[id] != Kind::F64 {
            continue;
        }
        let value = match step {
            Step::Copy(source) => floats[source],
            Step::Read { .. } => reads[id],
            Step::Primitive(op, args) => {
                let args = args.iter().map(|id| floats[id]).collect::<Vec<_>>();
                match (op, args.as_slice()) {
                    (Op::F64Neg, [v]) => b.ins().fneg(*v),
                    (Op::F64Sqrt, [v]) => b.ins().sqrt(*v),
                    (op, [a, c]) => f64_arithmetic(b, *op, *a, *c),
                    _ => unreachable!("proved F64 point"),
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
    let mut words = prefix
        .exports
        .iter()
        .map(|&id| {
            (
                id,
                b.ins().bitcast(types::I64, MemFlags::new(), floats[&id]),
            )
        })
        .collect::<Vec<_>>();
    if let Some(cache) = cache {
        if let Some(id) = cache.recipe.old {
            if !prefix.exports.contains(&id) {
                words.push((
                    id,
                    b.ins().bitcast(types::I64, MemFlags::new(), floats[&id]),
                ));
            }
        }
        cache.stage(
            b,
            inputs[&cache.recipe.column],
            &reads,
            floats[&cache.recipe.result],
        );
    }
    Evaluated { words, reused }
}
pub(super) fn emit(
    b: &mut FunctionBuilder<'_>,
    options: OptimizationOptions,
    words: &Words,
    shadow: usize,
    proof: &Proof,
    code: crate::native_artifact::codegen::control::plan::CodePlan<'_>,
    prefix: &Prefix,
    cache: Option<&rolling::Cache>,
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
        .filter_map(|(_, step)| {
            if let Step::Read { array, .. } = step {
                Some(*array)
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    let views = arrays
        .into_iter()
        .map(|array| {
            let at = shadow + proof.view(array);
            let axes = if options.enabled(
                crate::native_artifact::codegen::options::OptimizationPass::StaticNumericExtents,
            ) {
                super::numeric_words::fixed_axes(code, proof, array)
            } else {
                [None, None]
            };
            let pointer = words.read(b, at);
            let rows = match axes[0] {
                Some(n) => b.ins().iconst(types::I64, n as i64),
                None => words.read(b, at + 2),
            };
            let columns = match axes[1] {
                Some(n) => b.ins().iconst(types::I64, n as i64),
                None => words.read(b, at + 3),
            };
            (array, [pointer, rows, columns])
        })
        .collect();
    let result = evaluate(b, options, prefix, &inputs, &views, slow, cache);
    let _ = result.reused;
    for (id, value) in result.words {
        words.write(b, id as usize, value);
    }
    b.ins().jump(next, &[]);
}
#[cfg(all(test, feature = "canonical-native-program-runtime"))]
#[path = "numeric_cell_prefix_tests.rs"]
mod tests;

#[path = "numeric_cell_rolling.rs"]
pub(in crate::native_artifact) mod rolling;
