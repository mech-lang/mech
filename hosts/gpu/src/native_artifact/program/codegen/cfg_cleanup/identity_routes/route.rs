//! An edge may bypass only total scalar control. No load, store, call, checked
//! arithmetic, source debit or exceptional edge is crossed.
use super::super::route_uses::Uses;
use super::super::word_ranges::{Facts, Range};
use super::values::{self, Word};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::ir::{Block, Function, Inst, InstructionData as Data, Opcode, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Edge {
    pub inst: Inst,
    pub ordinal: usize,
    pub target: Block,
    pub args: Vec<Value>,
}

pub(super) struct Plan {
    pub edge: Edge,
    pub target: Block,
    pub args: Vec<Word>,
    pub skipped: BTreeSet<Block>,
    pub repairs: BTreeMap<Value, Word>,
    pub conditions: usize,
}

pub(super) fn pure(f: &Function, block: Block, last: Inst) -> bool {
    f.layout
        .block_insts(block)
        .take_while(|&i| i != last)
        .all(|i| {
            matches!(
                f.dfg.insts[i].opcode(),
                Opcode::Iconst
                    | Opcode::Icmp
                    | Opcode::IcmpImm
                    | Opcode::Uextend
                    | Opcode::Ireduce
                    | Opcode::Band
                    | Opcode::BandImm
                    | Opcode::Bor
                    | Opcode::Bxor
            )
        })
}

// A pure route can pass the same SSA value into two different comparison
// operands. Their exact unsigned or signed relation does not need a range.
fn identical_comparison(f: &Function, inst: Inst, words: &BTreeMap<Value, Word>) -> Option<Range> {
    let Data::IntCompare { cond, args, .. } = f.dfg.insts[inst] else {
        return None;
    };
    let ty = f.dfg.value_type(args[0]);
    if !ty.is_int() || ty.is_vector() || ty.bits() > 64 {
        return None;
    }
    let source = |value| {
        let value = f.dfg.resolve_aliases(value);
        match words.get(&value) {
            Some(Word::Value(source, _)) => Some(f.dfg.resolve_aliases(*source)),
            Some(Word::Constant(_, _)) => None,
            None => Some(value),
        }
    };
    if source(args[0])? != source(args[1])? {
        return None;
    }
    let truth = u64::from(super::super::integer_comparison(cond, 0, 0, ty.bits()));
    Some(Range(truth, truth))
}

pub(super) fn find(
    f: &Function,
    dom: &DominatorTree,
    uses: &Uses,
    edge: Edge,
    mut known: BTreeMap<Value, Range>,
    equate_transports: bool,
) -> Option<Plan> {
    let source = f.layout.inst_block(edge.inst)?;
    if !dom.is_reachable(source) {
        return None;
    }
    if let Data::Brif { arg, .. } = f.dfg.insts[edge.inst] {
        if !super::super::word_ranges::condition::assume(f, arg, edge.ordinal == 0, &mut known) {
            return None;
        }
    } else if !edge.args.iter().any(|&v| {
        values::range(values::source(f, v, &known)).is_some_and(|r| r.0 == r.1 && r.1 <= 1)
    }) {
        return None;
    }
    let mut target = edge.target;
    let source_known = known;
    let mut known = BTreeMap::new();
    let mut args = edge
        .args
        .iter()
        .copied()
        .map(|v| values::source(f, v, &source_known))
        .collect::<Vec<_>>();
    let mut words = BTreeMap::new();
    let mut skipped = BTreeSet::new();
    let mut conditions = 0;
    loop {
        if skipped.contains(&target)
            || Some(target) == f.layout.entry_block()
            || dom.block_dominates(target, source)
        {
            return None;
        }
        let last = f.layout.last_inst(target)?;
        if !pure(f, target, last) {
            break;
        }
        let params = f.dfg.block_params(target);
        if params.len() != args.len() {
            return None;
        }
        let mut next_known = known.clone();
        for (&param, &word) in params.iter().zip(&args) {
            if let Some(r) = values::range(word) {
                next_known.insert(param, r);
            } else {
                next_known.remove(&param);
            }
        }
        for inst in f.layout.block_insts(target).take_while(|&i| i != last) {
            for &arg in f.dfg.inst_args(inst) {
                let arg = f.dfg.resolve_aliases(arg);
                if !next_known.contains_key(&arg) {
                    if let Some(r) = values::range(values::source(f, arg, &source_known)) {
                        next_known.insert(arg, r);
                    }
                }
            }
            for &result in f.dfg.inst_results(inst) {
                next_known.remove(&result);
                let range = equate_transports
                    .then(|| identical_comparison(f, inst, &words))
                    .flatten()
                    .or_else(|| Facts::bound(f, &next_known).value_range(result));
                if let Some(r) = range {
                    next_known.insert(result, r);
                }
            }
        }
        let call = match &f.dfg.insts[last] {
            Data::Jump { destination, .. } => *destination,
            Data::Brif { arg, blocks, .. } => {
                let arg = f.dfg.resolve_aliases(*arg);
                if !next_known.contains_key(&arg) {
                    if let Some(r) = values::range(values::source(f, arg, &source_known)) {
                        next_known.insert(arg, r);
                    }
                }
                let Some(truth) = Facts::bound(f, &next_known).condition(arg) else {
                    break;
                };
                conditions += 1;
                blocks[usize::from(!truth)]
            }
            _ => break,
        };
        for (&param, &word) in params.iter().zip(&args) {
            words.insert(param, word);
        }
        known = next_known;
        skipped.insert(target);
        args = call
            .args(&f.dfg.value_lists)
            .map(|arg| values::forward(f, arg.as_value()?, &words, &known, &skipped, &source_known))
            .collect::<Option<Vec<_>>>()?;
        target = call.block(&f.dfg.value_lists);
    }
    if conditions == 0 || skipped.is_empty() {
        return None;
    }
    // Any skipped SSA definition used after the new destination must be
    // transported through a destination phi. At a shared merge outside this
    // destination's dominance region we decline rather than guess an SSA fix.
    let definitions = skipped
        .iter()
        .flat_map(|&block| {
            f.dfg.block_params(block).iter().copied().chain(
                f.layout
                    .block_insts(block)
                    .flat_map(|i| f.dfg.inst_results(i).iter().copied()),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut repairs = BTreeMap::new();
    let mut live_outs = uses.live_outs(target);
    for value in definitions {
        let owner = values::owner(f, value)?;
        if live_outs.required(dom, value, owner)? {
            repairs.insert(
                value,
                values::forward(f, value, &words, &known, &skipped, &source_known)?,
            );
        }
    }
    Some(Plan {
        edge,
        target,
        args,
        skipped,
        repairs,
        conditions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cranelift_codegen::ir::{AbiParam, InstBuilder, condcodes::IntCC, types};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    #[test]
    fn transported_comparisons_require_the_same_source_word() {
        let mut f = Function::new();
        f.signature.params = vec![AbiParam::new(types::I64); 2];
        let mut state = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut f, &mut state);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        let [left, right] = b.block_params(entry).try_into().unwrap();
        b.switch_to_block(entry);
        let greater = b.ins().icmp(IntCC::UnsignedGreaterThan, left, right);
        let less_equal = b.ins().icmp(IntCC::SignedLessThanOrEqual, left, right);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();

        let greater = f.dfg.value_def(greater).inst().unwrap();
        let less_equal = f.dfg.value_def(less_equal).inst().unwrap();
        let mut words = BTreeMap::new();
        words.insert(left, Word::Value(left, Some(Range(0, 255))));
        words.insert(right, Word::Value(right, Some(Range(0, 255))));
        assert!(identical_comparison(&f, greater, &words).is_none());
        words.insert(right, Word::Value(left, None));
        assert_eq!(identical_comparison(&f, greater, &words), Some(Range(0, 0)));
        assert_eq!(
            identical_comparison(&f, less_equal, &words),
            Some(Range(1, 1))
        );
    }
}
