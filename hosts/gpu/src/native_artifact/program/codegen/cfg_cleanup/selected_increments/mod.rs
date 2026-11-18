//! Prove only impossible checked I64 +1 failures across bounded selected edges.
//! Each state names current SSA atoms. Phi arguments are substituted together
//! at their exact incoming edge; definitions are never re-evaluated using a
//! later loop visit's values. Backedges, cycles and exhausted proofs decline.
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as D, Opcode as O, Value, ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_BLOCKS: usize = 4096;
const MAX_DEPTH: usize = 64;
const MAX_STATES: usize = 256;
const MAX_CONSTRAINTS: usize = 32;
const MAX_STEPS: usize = 262_144;

struct Budget(usize);
impl Budget {
    fn spend(&mut self, n: usize) -> Option<()> {
        self.0 = self.0.checked_sub(n)?;
        Some(())
    }
}
fn definition(f: &Function, v: Value) -> Option<&D> {
    let ValueDef::Result(i, 0) = f.dfg.value_def(f.dfg.resolve_aliases(v)) else {
        return None;
    };
    Some(&f.dfg.insts[i])
}
fn mask(f: &Function, v: Value) -> Option<u64> {
    let t = f.dfg.value_type(v);
    (t.is_int() && !t.is_vector() && (1..=64).contains(&t.bits()))
        .then(|| u64::MAX >> (64 - t.bits()))
}
fn literal(f: &Function, v: Value) -> Option<u64> {
    match definition(f, v)? {
        D::UnaryImm {
            opcode: O::Iconst,
            imm,
            ..
        } => Some(imm.bits() as u64 & mask(f, v)?),
        _ => None,
    }
}

// A saved predicate remains an atom. Only total bit masks/zero extensions are
// represented structurally here: no comparison, load, call or arithmetic is
// replayed under later facts. A raw odd word retains its exact masked semantics.
#[derive(Clone, Copy)]
struct Constraint {
    atom: Value,
    mask: u64,
    nonzero: bool,
}
fn constraint(f: &Function, v: Value, truth: bool, budget: &mut Budget) -> Option<Constraint> {
    let mut c = Constraint {
        atom: f.dfg.resolve_aliases(v),
        mask: mask(f, v)?,
        nonzero: truth,
    };
    for _ in 0..16 {
        budget.spend(1)?;
        let step = match definition(f, c.atom) {
            Some(D::Unary {
                opcode: O::Uextend,
                arg,
            }) => Some((*arg, mask(f, *arg)?)),
            Some(D::BinaryImm64 {
                opcode: O::BandImm,
                arg,
                imm,
            }) => Some((*arg, imm.bits() as u64)),
            Some(D::Binary {
                opcode: O::Band,
                args,
            }) => literal(f, args[1])
                .map(|n| (args[0], n))
                .or_else(|| literal(f, args[0]).map(|n| (args[1], n))),
            _ => None,
        };
        let Some((arg, bits)) = step else {
            return Some(c);
        };
        c.atom = f.dfg.resolve_aliases(arg);
        c.mask &= bits;
    }
    None
}
// This never interprets a prior predicate's definition under a new phi value.
fn feasible(f: &Function, constraints: &mut Vec<Constraint>) -> bool {
    let mut seen = BTreeMap::new();
    constraints.retain(|c| literal(f, c.atom).is_none() && c.mask != 0);
    for c in constraints.iter() {
        if let Some(old) = seen.insert((c.atom, c.mask), c.nonzero)
            && old != c.nonzero
        {
            return false;
        }
    }
    true
}
fn check_literals(f: &Function, constraints: &[Constraint]) -> bool {
    constraints.iter().all(|c| {
        if c.mask == 0 {
            !c.nonzero
        } else {
            literal(f, c.atom).is_none_or(|n| ((n & c.mask) != 0) == c.nonzero)
        }
    })
}

// The success edge must itself be unique. A dynamic unsigned upper bound is
// enough: x<y implies x<=MAX-1, regardless of y's actual value or provenance.
struct StrictBound {
    at: Block,
    value: Value,
}
fn strict_bounds(
    f: &Function,
    cfg: &ControlFlowGraph,
    budget: &mut Budget,
) -> Option<Vec<StrictBound>> {
    let mut facts = Vec::new();
    for block in f.layout.blocks() {
        budget.spend(1)?;
        let Some(inst) = f.layout.last_inst(block) else {
            continue;
        };
        let D::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
            continue;
        };
        // A comparison produces canonical 0/1, so total masks/zero-extends
        // preserve its truth exactly when bit zero survives. Its operands are
        // recorded as immutable atoms, never re-evaluated from later facts.
        let Some(selected) = constraint(f, arg, true, budget) else {
            continue;
        };
        if selected.mask & 1 == 0 {
            continue;
        }
        let Some(D::IntCompare { cond, args, .. }) = definition(f, selected.atom) else {
            continue;
        };
        if args.iter().any(|&v| f.dfg.value_type(v) != types::I64) {
            continue;
        }
        let (ordinal, value) = match *cond {
            IntCC::UnsignedLessThan => (0, args[0]),
            IntCC::UnsignedGreaterThan => (0, args[1]),
            IntCC::UnsignedGreaterThanOrEqual => (1, args[0]),
            IntCC::UnsignedLessThanOrEqual => (1, args[1]),
            _ => continue,
        };
        let at = blocks[ordinal].block(&f.dfg.value_lists);
        if Some(at) == f.layout.entry_block()
            || blocks[0].block(&f.dfg.value_lists) == blocks[1].block(&f.dfg.value_lists)
        {
            continue;
        }
        let mut preds = cfg.pred_iter(at);
        if preds.next().is_none() || preds.next().is_some() {
            continue;
        }
        facts.push(StrictBound {
            at,
            value: f.dfg.resolve_aliases(value),
        });
    }
    Some(facts)
}

fn increment(f: &Function, condition: Value, budget: &mut Budget) -> Option<(Value, usize)> {
    let selected = constraint(f, condition, true, budget)?;
    if selected.mask & 1 == 0 {
        return None;
    }
    let D::IntCompare { cond, args, .. } = *definition(f, selected.atom)? else {
        return None;
    };
    let (sum, base, success) = match cond {
        IntCC::UnsignedGreaterThanOrEqual => (args[0], args[1], 0),
        IntCC::UnsignedLessThanOrEqual => (args[1], args[0], 0),
        IntCC::UnsignedLessThan => (args[0], args[1], 1),
        IntCC::UnsignedGreaterThan => (args[1], args[0], 1),
        _ => return None,
    };
    let base = f.dfg.resolve_aliases(base);
    if f.dfg.value_type(base) != types::I64 || f.dfg.value_type(sum) != types::I64 {
        return None;
    }
    let same = match *definition(f, sum)? {
        D::BinaryImm64 {
            opcode: O::IaddImm,
            arg,
            imm,
        } => imm.bits() == 1 && f.dfg.resolve_aliases(arg) == base,
        D::Binary {
            opcode: O::Iadd,
            args,
        } => {
            (f.dfg.resolve_aliases(args[0]) == base && literal(f, args[1]) == Some(1))
                || (f.dfg.resolve_aliases(args[1]) == base && literal(f, args[0]) == Some(1))
        }
        _ => false,
    };
    same.then_some((base, success))
}

#[derive(Clone)]
struct State {
    block: Block,
    base: Value,
    constraints: Vec<Constraint>,
}
struct Proof<'a> {
    f: &'a Function,
    cfg: &'a ControlFlowGraph,
    dom: &'a DominatorTree,
    facts: &'a [StrictBound],
    budget: &'a mut Budget,
    remaining_states: usize,
    visiting: BTreeSet<Block>,
}
impl Proof<'_> {
    fn prove(&mut self, mut state: State, depth: usize) -> Option<bool> {
        self.budget.spend(state.constraints.len() + 1)?;
        self.remaining_states = self.remaining_states.checked_sub(1)?;
        if depth == 0 || state.constraints.len() > MAX_CONSTRAINTS {
            return None;
        }
        // Constants are checked before dropping them; contradictory states are
        // genuinely infeasible, never a substitute for an exhausted proof.
        if !check_literals(self.f, &state.constraints) || !feasible(self.f, &mut state.constraints)
        {
            return Some(true);
        }
        self.budget.spend(self.facts.len())?;
        if self
            .facts
            .iter()
            .any(|fact| fact.value == state.base && self.dom.block_dominates(fact.at, state.block))
        {
            return Some(true);
        }
        if !self.visiting.insert(state.block) {
            return None;
        }
        let block = state.block;
        let result = self.incoming_inner(&state, depth);
        self.visiting.remove(&block);
        result
    }
    fn incoming_inner(&mut self, state: &State, depth: usize) -> Option<bool> {
        if Some(state.block) == self.f.layout.entry_block() {
            return Some(false);
        }
        let mut preds = Vec::new();
        for pred in self.cfg.pred_iter(state.block) {
            self.budget.spend(1)?;
            preds.push(pred);
        }
        if preds.is_empty() {
            return Some(false);
        }
        for pred in preds {
            // Current loop parameters are snapshots. Do not cross a backedge,
            // even if a later visit would happen to reuse the same Value IDs.
            if self.dom.block_dominates(state.block, pred.block) {
                return None;
            }
            let data = &self.f.dfg.insts[pred.inst];
            if !matches!(data, D::Jump { .. } | D::Brif { .. }) {
                return None;
            }
            let destinations =
                data.branch_destination(&self.f.dfg.jump_tables, &self.f.dfg.exception_tables);
            let mut matching = destinations
                .iter()
                .enumerate()
                .filter(|(_, c)| c.block(&self.f.dfg.value_lists) == state.block);
            let (ordinal, call) = matching.next()?;
            if matching.next().is_some() {
                return None;
            }
            let mut args = Vec::new();
            for arg in call.args(&self.f.dfg.value_lists) {
                self.budget.spend(1)?;
                args.push(arg.as_value()?);
            }
            let params = self.f.dfg.block_params(state.block);
            self.budget
                .spend(args.len() + state.constraints.len() + 1)?;
            if args.len() != params.len() {
                return None;
            }
            let mut substitutions = BTreeMap::new();
            for (&p, a) in params.iter().zip(args) {
                // A removed frontend phi may alias an external atom. Such an
                // atom is not this edge's newly bound current parameter.
                if self.f.dfg.resolve_aliases(p) != p
                    || substitutions
                        .insert(p, self.f.dfg.resolve_aliases(a))
                        .is_some()
                {
                    return None;
                }
            }
            if substitutions
                .iter()
                .any(|(&p, &a)| self.f.dfg.value_type(p) != self.f.dfg.value_type(a))
            {
                return None;
            }
            let replace = |v: Value| substitutions.get(&v).copied().unwrap_or(v);
            let base = replace(state.base);
            let mut constraints = state
                .constraints
                .iter()
                .map(|c| Constraint {
                    atom: replace(c.atom),
                    ..*c
                })
                .collect::<Vec<_>>();
            if let D::Brif { arg, .. } = data {
                constraints.push(constraint(self.f, *arg, ordinal == 0, self.budget)?);
            }
            if !self.prove(
                State {
                    block: pred.block,
                    base,
                    constraints,
                },
                depth - 1,
            )? {
                return Some(false);
            }
        }
        Some(true)
    }
}

pub(super) fn optimize(f: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> usize {
    optimize_bounded(f, cfg, dom, MAX_STEPS)
}
fn optimize_bounded(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    steps: usize,
) -> usize {
    let mut budget = Budget(steps);
    for (n, block) in f.layout.blocks().enumerate() {
        if n >= MAX_BLOCKS || budget.spend(1).is_none() {
            return 0;
        }
        for inst in f.layout.block_insts(block) {
            if budget.spend(1).is_none() || f.dfg.insts[inst].exception_table().is_some() {
                return 0;
            }
        }
    }
    let Some(facts) = strict_bounds(f, cfg, &mut budget) else {
        return 0;
    };
    let mut changes = Vec::<(Inst, Block, Vec<_>)>::new();
    for block in f.layout.blocks() {
        if !dom.is_reachable(block) {
            continue;
        }
        let Some(inst) = f.layout.last_inst(block) else {
            continue;
        };
        let D::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
            continue;
        };
        let Some((base, success)) = increment(f, arg, &mut budget) else {
            continue;
        };
        let mut proof = Proof {
            f,
            cfg,
            dom,
            facts: &facts,
            budget: &mut budget,
            remaining_states: MAX_STATES,
            visiting: BTreeSet::new(),
        };
        if proof.prove(
            State {
                block,
                base,
                constraints: Vec::new(),
            },
            MAX_DEPTH,
        ) == Some(true)
        {
            let call = blocks[success];
            let count = call.len(&f.dfg.value_lists);
            if count > MAX_STEPS || budget.spend(count + 1).is_none() {
                break;
            }
            let mut args = Vec::with_capacity(count);
            for arg in call.args(&f.dfg.value_lists) {
                args.push(arg);
            }
            changes.push((inst, call.block(&f.dfg.value_lists), args));
        }
        if budget.0 == 0 {
            break;
        }
    }
    for (inst, target, args) in &changes {
        f.dfg.replace(*inst).jump(*target, args);
    }
    changes.len()
}

#[cfg(test)]
mod tests;
