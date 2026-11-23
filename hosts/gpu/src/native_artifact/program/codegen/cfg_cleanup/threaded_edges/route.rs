//! An edge may bypass only total scalar control. No load, store, call, checked
//! arithmetic, source debit or exceptional edge is crossed.
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

fn pure(f: &Function, block: Block, last: Inst) -> bool {
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

// A route narrows at most one source-edge fact. Restore that entry when
// discovery finishes, including every early refusal, before the next edge.
struct SourceRanges<'a> {
    known: &'a mut BTreeMap<Value, Range>,
    prior: Option<(Value, Option<Range>)>,
}

impl Drop for SourceRanges<'_> {
    fn drop(&mut self) {
        if let Some((value, prior)) = self.prior {
            if let Some(range) = prior {
                self.known.insert(value, range);
            } else {
                self.known.remove(&value);
            }
        }
    }
}

pub(super) fn find(
    f: &Function,
    dom: &DominatorTree,
    edge: Edge,
    known: &mut BTreeMap<Value, Range>,
    visits: &mut super::visits::Visits,
) -> Option<Plan> {
    let source = f.layout.inst_block(edge.inst)?;
    if !dom.is_reachable(source) {
        return None;
    }
    let mut source_ranges = SourceRanges { known, prior: None };
    if let Data::Brif { arg, .. } = f.dfg.insts[edge.inst] {
        let ty = f.dfg.value_type(arg);
        if ty.is_int() && !ty.is_vector() && ty.bits() <= 64 {
            let bound = if edge.ordinal == 0 {
                Range(1, u64::MAX >> (64 - ty.bits()))
            } else {
                Range(0, 0)
            };
            let arg = f.dfg.resolve_aliases(arg);
            let bound = source_ranges
                .known
                .get(&arg)
                .map_or(bound, |old| Range(old.0.max(bound.0), old.1.min(bound.1)));
            if bound.0 > bound.1 {
                return None;
            }
            source_ranges.prior = Some((arg, source_ranges.known.insert(arg, bound)));
        }
    }
    let source_known = &*source_ranges.known;
    // A constant current tag is the witness; unrestricted payload words alone
    // never justify threading. This is not a source enum/type assumption.
    if !edge.args.iter().any(|&v| {
        values::range(values::source(f, v, source_known)).is_some_and(|r| r.0 == r.1 && r.1 <= 1)
    }) {
        return None;
    }
    let mut target = edge.target;
    let mut known = BTreeMap::new();
    let mut args = edge
        .args
        .iter()
        .copied()
        .map(|v| values::source(f, v, source_known))
        .collect::<Vec<_>>();
    let mut words = BTreeMap::new();
    let mut skipped = BTreeSet::new();
    let mut backedge = false;
    let mut conditions = 0;
    loop {
        if skipped.contains(&target) || Some(target) == f.layout.entry_block() {
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
                    if let Some(r) = values::range(values::source(f, arg, source_known)) {
                        next_known.insert(arg, r);
                    }
                }
            }
            for &result in f.dfg.inst_results(inst) {
                next_known.remove(&result);
                if let Some(r) = Facts::bound(f, &next_known).value_range(result) {
                    next_known.insert(result, r);
                }
            }
        }
        let call = match &f.dfg.insts[last] {
            Data::Jump { destination, .. } => *destination,
            Data::Brif { arg, blocks, .. } => {
                let arg = f.dfg.resolve_aliases(*arg);
                if !next_known.contains_key(&arg) {
                    if let Some(r) = values::range(values::source(f, arg, source_known)) {
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
        backedge |= dom.block_dominates(target, source);
        args = call
            .args(&f.dfg.value_lists)
            .map(|arg| values::forward(f, arg.as_value()?, &words, &known, &skipped, source_known))
            .collect::<Option<Vec<_>>>()?;
        target = call.block(&f.dfg.value_lists);
    }
    if !backedge || conditions == 0 || skipped.is_empty() {
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
    for value in definitions {
        let owner = values::owner(f, value)?;
        let mut pending = vec![target];
        visits.begin();
        let mut used = false;
        while let Some(block) = pending.pop() {
            // A fresh visit to its definition ends this value's old activation.
            if block == owner || !visits.insert(block) {
                continue;
            }
            for inst in f.layout.block_insts(block) {
                let mut uses = f.dfg.inst_args(inst).contains(&value);
                for call in f.dfg.insts[inst]
                    .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                {
                    uses |= call
                        .args(&f.dfg.value_lists)
                        .any(|a| a.as_value() == Some(value));
                    pending.push(call.block(&f.dfg.value_lists));
                }
                if uses {
                    if !dom.block_dominates(target, block) || !dom.block_dominates(owner, target) {
                        return None;
                    }
                    used = true;
                }
            }
        }
        if used {
            repairs.insert(
                value,
                values::forward(f, value, &words, &known, &skipped, source_known)?,
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
