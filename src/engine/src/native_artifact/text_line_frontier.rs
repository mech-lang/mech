//! Prove where an unfinished text/lines count can remain symbolic.
//!
//! For `index > final_count`, an available prefix proves false, never true.
//! Otherwise execution must await more input or EOF. ALL other numeric reads
//! of the count must be unreachable until an exhausted edge has been taken.
//! This is only an EOF-dependence proof: it does not authorize acquisition,
//! discard line storage, construct a partial canonical Value, or resume code.
use super::{
    NativeControlLoop, NativeIterationAction as Action, NativeIterationTerminator as Terminator,
    NativeIterationValueScope as Scope, NativeMatchAction as Scalar, NativeMatchTerminator as Term,
    NativeMatchValue as Value, NativeProgramArtifactPlan as Plan, NativeScalarKind,
    NativeScalarOp as Op, NativeTextLineMatrixRegion,
    fusion_semantics::{Read, SemanticActions, reads, semantic_actions},
    iteration_validation::ControlPlan,
};
use mech_core::ValueData;

#[path = "text_line_availability.rs"]
mod availability;
pub use availability::{NativeTextLineReadFrontier, NativeTextLineReadGuard};

#[path = "text_line_retention.rs"]
mod retention;
pub use retention::{NativeTextLineRetentionFrontier, NativeTextLineRetentionState};

/// A predicate at its original scalar comparison and branch sites. The count
/// may not be replaced by a prefix length in any canonical/native value slot.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeTextLineCountGuard {
    comparison: u32,
    branch: u32,
    index: Value,
    predicate: Value,
    available: u32,
    exhausted: u32,
}
impl NativeTextLineCountGuard {
    pub const fn comparison(&self) -> u32 {
        self.comparison
    }
    pub const fn branch(&self) -> u32 {
        self.branch
    }
    pub const fn index(&self) -> Value {
        self.index
    }
    pub const fn predicate(&self) -> Value {
        self.predicate
    }
    /// Successor when the original `index > count` predicate is false.
    pub const fn available(&self) -> u32 {
        self.available
    }
    /// Successor when it is true; taking it requires the genuine final count.
    pub const fn exhausted(&self) -> u32 {
        self.exhausted
    }
}

/// Constructor-private, plan-local metadata. Before using this in a streaming
/// backend, separately prove line availability/retention, bind the exact host
/// capture/session lifetime, and preserve full acquisition errors and effects.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeTextLineCountFrontier {
    lines: NativeTextLineMatrixRegion,
    counts: Box<[Value]>,
    aliases: Box<[u32]>,
    guards: Box<[NativeTextLineCountGuard]>,
    exact: Box<[u32]>,
}
impl NativeTextLineCountFrontier {
    pub fn lines(&self) -> &NativeTextLineMatrixRegion {
        &self.lines
    }
    /// Length results and their proved identical, immutable U64 aliases.
    pub fn counts(&self) -> &[Value] {
        &self.counts
    }
    /// Original Copy sites that can propagate the symbolic count identity.
    pub fn aliases(&self) -> &[u32] {
        &self.aliases
    }
    pub fn guards(&self) -> &[NativeTextLineCountGuard] {
        &self.guards
    }
    /// Other original readers, all proved unreachable before an EOF edge.
    pub fn exact(&self) -> &[u32] {
        &self.exact
    }
}

/// Every definition of a possible alias is inventoried. In particular, one
/// count-valued phi arm must not hide another arm, a loop result, or an input.
fn definitions(plan: &Plan, actions: &SemanticActions<'_>) -> Vec<Vec<Option<Value>>> {
    let mut all = vec![Vec::new(); plan.values().len()];
    for input in plan.inputs() {
        all[input.value().get() as usize].push(None);
    }
    for observation in plan.observations() {
        all[observation.value().get() as usize].push(None);
    }
    for constant in plan.constants() {
        all[constant.value().get() as usize].push(None);
    }
    for action in actions.iter() {
        let (output, source) = match action {
            Action::Scalar(Scalar::Copy { source, output }) => (*output, Some(*source)),
            Action::Scalar(Scalar::Primitive { output, .. })
            | Action::Structural { output, .. }
            | Action::LibraryCall { output, .. } => (*output, None),
            Action::CollectionElement { iteration } => {
                let NativeControlLoop::Collection(generator) =
                    &plan.iterations()[*iteration as usize]
                else {
                    continue;
                };
                let Some(output) = generator.element() else {
                    continue;
                };
                (output, None)
            }
            Action::CollectionFinish { collection } => {
                (plan.collections()[*collection as usize].output(), None)
            }
            _ => continue,
        };
        all[output.get() as usize].push(source);
    }
    for iteration in plan.iterations() {
        let pair = match iteration {
            NativeControlLoop::Iteration(loop_) => Some((loop_.carrier(), loop_.result())),
            NativeControlLoop::Tail(loop_) => Some((loop_.carrier(), loop_.result())),
            NativeControlLoop::Collection(_) => None,
        };
        if let Some((carrier, result)) = pair {
            all[carrier.get() as usize].push(None);
            all[result.get() as usize].push(None);
        }
    }
    all
}

fn successors(terminator: Terminator) -> [Option<u32>; 2] {
    match terminator {
        Terminator::Scalar(Term::Jump(next)) => [Some(next), None],
        Terminator::Scalar(Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. }) => {
            [Some(yes), Some(no)]
        }
        Terminator::Scalar(Term::Complete | Term::Reject) => [None, None],
        Terminator::Header { body, exit, .. } => [Some(body), Some(exit)],
        Terminator::TailHeader { body, .. } => [Some(body), None],
    }
}

fn bool_constant(plan: &Plan, value: Value) -> Option<bool> {
    plan.constants().iter().find_map(|constant| {
        if constant.value() == value
            && let ValueData::Bool(value) = constant.constant().data()
        {
            Some(*value)
        } else {
            None
        }
    })
}

fn guards_for_comparison(
    plan: &Plan,
    site: u32,
    counts: &[bool],
    uses: &[Vec<Read>],
    definitions: &[Vec<Option<Value>>],
) -> Option<Vec<NativeTextLineCountGuard>> {
    let block = &plan.blocks()[site as usize];
    let Action::Scalar(Scalar::Primitive {
        op,
        operands,
        output,
        ..
    }) = block.action()
    else {
        return None;
    };
    let [left, right] = operands.as_ref() else {
        return None;
    };
    if definitions[output.get() as usize].as_slice() != [None] {
        return None;
    }
    let index = match op {
        Op::U64Gt if !counts[left.get() as usize] && counts[right.get() as usize] => *left,
        Op::U64Lt if counts[left.get() as usize] && !counts[right.get() as usize] => *right,
        _ => return None,
    };
    Some(
        predicate_branches(plan, site, *output, uses, definitions)?
            .into_iter()
            .map(|(branch, available, exhausted)| NativeTextLineCountGuard {
                comparison: site,
                branch,
                index,
                predicate: *output,
                available,
                exhausted,
            })
            .collect(),
    )
}

/// Complete uses of one Bool predicate and identity-only copies. Tuples are
/// (branch site, false successor, true successor), independent of arm order.
fn predicate_branches(
    plan: &Plan,
    site: u32,
    predicate: Value,
    uses: &[Vec<Read>],
    definitions: &[Vec<Option<Value>>],
) -> Option<Vec<(u32, u32, u32)>> {
    let block = &plan.blocks()[site as usize];
    if definitions[predicate.get() as usize].as_slice() != [None] {
        return None;
    }
    let mut aliases = vec![predicate];
    let mut branches = Vec::new();
    let mut next = 0;
    while next < aliases.len() {
        let value = aliases[next];
        next += 1;
        for usage in &uses[value.get() as usize] {
            match *usage {
                Read::Action(reader) => {
                    let Action::Scalar(Scalar::Copy { source, output }) =
                        plan.blocks()[reader as usize].action()
                    else {
                        return None;
                    };
                    if *source != value
                        || definitions[output.get() as usize].as_slice() != [Some(value)]
                        || aliases.contains(output)
                        || plan.lifecycle_barrier(reader as usize)
                    {
                        return None;
                    }
                    aliases.push(*output);
                }
                Read::Terminator(reader) => {
                    if plan.blocks()[reader as usize].iteration() != block.iteration()
                        || plan.lifecycle_barrier(reader as usize)
                    {
                        return None;
                    }
                    let (yes, no, truth) = match plan.blocks()[reader as usize].terminator() {
                        Terminator::Scalar(Term::Branch { condition, yes, no })
                            if condition == value =>
                        {
                            (yes, no, true)
                        }
                        Terminator::Scalar(Term::LiteralBranch {
                            left,
                            right,
                            yes,
                            no,
                        }) => {
                            let literal = if left == value {
                                bool_constant(plan, right)?
                            } else if right == value {
                                bool_constant(plan, left)?
                            } else {
                                return None;
                            };
                            (yes, no, literal)
                        }
                        _ => return None,
                    };
                    // Removing the EOF edge must NOT also remove an available
                    // edge to the same successor.
                    if yes == no {
                        return None;
                    }
                    branches.push((
                        reader,
                        if truth { no } else { yes },
                        if truth { yes } else { no },
                    ));
                }
                Read::Other => return None,
            }
        }
    }
    if branches.is_empty() {
        return None;
    }
    Some(branches)
}

fn frontier(
    plan: &Plan,
    lines: NativeTextLineMatrixRegion,
    actions: &SemanticActions<'_>,
    uses: &[Vec<Read>],
    definitions: &[Vec<Option<Value>>],
) -> Option<NativeTextLineCountFrontier> {
    if lines.lengths().is_empty()
        || uses[lines.source().get() as usize].as_slice() != [Read::Action(lines.producer())]
    {
        return None;
    }
    let mut counts = vec![false; plan.values().len()];
    for &site in lines.lengths() {
        let Action::LibraryCall { output, .. } = actions.get(site as usize)? else {
            return None;
        };
        if definitions[output.get() as usize].as_slice() != [None]
            || plan.lifecycle_barrier(site as usize)
        {
            return None;
        }
        counts[output.get() as usize] = true;
    }
    // A least fixed point admits aliases only when ALL definitions are already
    // known to carry this same count. Mixed/recursive phi identities stay exact.
    loop {
        let mut changed = false;
        for (value, defs) in definitions.iter().enumerate() {
            if !counts[value]
                && !defs.is_empty()
                && plan.values()[value].scalar_kind() == Some(NativeScalarKind::U64)
                && matches!(plan.value_scopes()[value], Scope::Immutable { .. })
                && defs
                    .iter()
                    .all(|source| source.is_some_and(|source| counts[source.get() as usize]))
            {
                counts[value] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut aliases = Vec::new();
    let mut guards = Vec::new();
    let mut exact = Vec::new();
    for (value, selected) in counts.iter().enumerate() {
        if !selected {
            continue;
        }
        for usage in &uses[value] {
            let site = match *usage {
                Read::Other => return None,
                Read::Action(site) | Read::Terminator(site) => site,
            };
            if plan.lifecycle_barrier(site as usize) {
                return None;
            }
            if let Read::Action(_) = usage {
                if matches!(actions[site as usize], Action::Scalar(Scalar::Copy { source, output })
                    if source.get() as usize == value && counts[output.get() as usize])
                {
                    aliases.push(site);
                    continue;
                }
                if let Some(selected) =
                    guards_for_comparison(plan, site, &counts, uses, definitions)
                {
                    guards.extend(selected);
                    continue;
                }
            }
            exact.push(site);
        }
    }
    if guards.is_empty() {
        return None;
    }
    aliases.sort_unstable();
    aliases.dedup();
    guards.sort_by_key(|guard| (guard.comparison, guard.branch));
    guards.dedup_by_key(|guard| (guard.comparison, guard.branch));
    exact.sort_unstable();
    exact.dedup();

    // Full CFG reachability, including every cold branch and declared loop
    // backedge. A heuristic source-order check would miss bypasses and reentry.
    let mut before_eof = vec![false; plan.blocks().len()];
    let mut pending = vec![plan.entry()];
    while let Some(site) = pending.pop() {
        if before_eof[site as usize] {
            continue;
        }
        before_eof[site as usize] = true;
        for next in successors(plan.blocks()[site as usize].terminator())
            .into_iter()
            .flatten()
        {
            if !guards
                .iter()
                .any(|guard| guard.branch == site && guard.exhausted == next)
            {
                pending.push(next);
            }
        }
    }
    if exact.iter().any(|site| before_eof[*site as usize]) {
        return None;
    }
    Some(NativeTextLineCountFrontier {
        lines,
        counts: counts
            .into_iter()
            .enumerate()
            .filter_map(|(value, selected)| selected.then_some(Value(value as u32)))
            .collect(),
        aliases: aliases.into(),
        guards: guards.into(),
        exact: exact.into(),
    })
}

impl Plan {
    /// Prove the exact EOF dependency of every read of a private line count.
    /// This does not by itself prove a program streamable, monotonic, or safe
    /// to discard. No consumer currently substitutes an unfinished count.
    pub fn text_line_count_frontiers(&self) -> Vec<NativeTextLineCountFrontier> {
        let regions = self.text_line_matrix_regions();
        if regions.is_empty() {
            return Vec::new();
        }
        let Ok(actions) = semantic_actions(self) else {
            return Vec::new();
        };
        let Ok(uses) = reads(self, &actions) else {
            return Vec::new();
        };
        let definitions = definitions(self, &actions);
        regions
            .into_iter()
            .filter_map(|lines| frontier(self, lines, &actions, &uses, &definitions))
            .collect()
    }
}

#[cfg(all(test, feature = "source", feature = "semantic-compiler"))]
#[path = "text_line_frontier_tests.rs"]
mod tests;
