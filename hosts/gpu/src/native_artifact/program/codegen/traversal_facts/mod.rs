//! Bounded facts for one original source traversal, shared by native strategies.
//! Facts never cross a latch; overwritten identities and joins lose information.
use crate::native_artifact::codegen::common::aggregates::field;
use crate::native_artifact::codegen::control::plan::CodePlan;
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeIterationTerminator as Term, NativeMatchAction as Scalar,
    NativeMatchTerminator as Branch, NativeMatchValue, NativeScalarAggregateAction as Aggregate,
    NativeScalarAggregates, NativeScalarKind, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};
mod captured_windows;
mod checks;
mod completion;
mod entry;
mod expressions;
mod flow;
mod loop_windows;
mod operations;
mod ranges;
mod read_reuse;
mod scope;
mod updates;
mod windows;
mod write_changes;
pub(super) use captured_windows::CapturedWindow;
pub(super) use checks::{Check, Operand, Queries};
pub(super) use completion::{Candidate, Completed};
pub(super) use expressions::{Graph, Node};
pub(super) use flow::with_checks;
pub(super) use flow::with_reused_reads;
pub(super) use flow::{Step, derive};
pub(super) use flow::{with_captured_windows, with_completion, with_windows};
pub(super) use loop_windows::Window;
use operations::{aggregate, scalar};
pub(super) use read_reuse::Snapshot;
pub(super) use scope::Scope;
const MAX_SITES: usize = 256;
const MAX_WORDS: usize = 512;
const MAX_PARTITIONS: usize = 9;
const MAX_TERMS: usize = 4096;
const MAX_RELATIONS: usize = 128;
type Id = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Relation {
    Eq,
    Ne,
    Lt,
    Le,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Arithmetic {
    Add,
    Sub,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Expr {
    Constant(u64),
    ArrayLength(usize),
    Initial(usize),
    Definition(usize, usize),
    Bounded(usize, usize, u64),
    Boolean(usize, usize),
    Phi(usize, usize),
    Arithmetic(Arithmetic, Id, Id),
    Wrapping(Arithmetic, Id, Id),
    Compare(Relation, Id, Id),
    Not(Id),
    And(Id, Id),
    Or(Id, Id),
}
#[derive(Default)]
struct Terms {
    terms: Vec<Expr>,
    interned: BTreeMap<Expr, Id>,
    recording: Option<expressions::Recording>,
    changes: write_changes::Values,
}
impl Terms {
    fn intern(&mut self, expr: Expr) -> Option<Id> {
        if let Some(&id) = self.interned.get(&expr) {
            return Some(id);
        }
        if self.terms.len() == MAX_TERMS {
            return None;
        }
        let id = self.terms.len();
        self.terms.push(expr.clone());
        self.interned.insert(expr, id);
        Some(id)
    }
    fn constant(&mut self, value: u64) -> Option<Id> {
        self.intern(Expr::Constant(value))
    }
    fn value(&self, id: Id) -> Option<u64> {
        match self.terms[id] {
            Expr::Constant(value) => Some(value),
            _ => None,
        }
    }
    fn boolean(&self, mut id: Id) -> bool {
        for _ in 0..MAX_TERMS {
            match self.terms[id] {
                Expr::Compare(..)
                | Expr::Boolean(..)
                | Expr::And(..)
                | Expr::Or(..)
                | Expr::Constant(0 | 1) => return true,
                Expr::Not(value) => id = value,
                _ => return false,
            }
        }
        false
    }
}
#[derive(Clone, Default)]
struct State {
    words: BTreeMap<usize, Id>,
    relations: BTreeSet<(Relation, Id, Id)>,
    successful: BTreeSet<(Arithmetic, Id, Id)>,
    windows: Vec<Window>,
    captures: Vec<CapturedWindow>,
    checked_reads: BTreeMap<(usize, Id), updates::CheckedRead>,
    content_version: write_changes::Version,
    snapshots: Option<BTreeMap<Snapshot, Id>>,
}
impl State {
    fn assign(&mut self, moves: &[(usize, usize)], parallel: bool) -> Option<()> {
        if parallel {
            let values = moves
                .iter()
                .map(|&(to, from)| Some((to, self.read(from)?)))
                .collect::<Option<Vec<_>>>()?;
            for (to, value) in values {
                self.words.insert(to, value);
            }
        } else {
            for &(to, from) in moves {
                self.words.insert(to, self.read(from)?);
            }
        }
        Some(())
    }

    fn read(&self, slot: usize) -> Option<Id> {
        self.words.get(&slot).copied()
    }
    fn constant(&self, terms: &Terms, value: Id) -> Option<u64> {
        terms.value(value).or_else(|| {
            self.relations.iter().find_map(|&(relation, a, b)| {
                if relation != Relation::Eq {
                    return None;
                }
                if a == value {
                    terms.value(b)
                } else if b == value {
                    terms.value(a)
                } else {
                    None
                }
            })
        })
    }
    fn truth(&self, terms: &Terms, relation: Relation, a: Id, b: Id) -> Option<bool> {
        if a == b {
            return Some(matches!(relation, Relation::Eq | Relation::Le));
        }
        if let (Some(a), Some(b)) = (self.constant(terms, a), self.constant(terms, b)) {
            return Some(match relation {
                Relation::Eq => a == b,
                Relation::Ne => a != b,
                Relation::Lt => a < b,
                Relation::Le => a <= b,
            });
        }
        if self.relations.contains(&(relation, a, b)) {
            return Some(true);
        }
        if relation == Relation::Le
            && let Expr::Arithmetic(Arithmetic::Add, x, y) = terms.terms[b]
            && (a == x || a == y)
        {
            return Some(true);
        }
        if self.window_holds(terms, a, b) {
            return Some(relation != Relation::Eq);
        }
        if self.window_holds(terms, b, a) {
            return Some(relation == Relation::Ne);
        }
        if self.capture_relation(terms, relation, a, b) {
            return Some(true);
        }
        if match relation {
            Relation::Lt => self.capture_relation(terms, Relation::Le, b, a),
            Relation::Le => self.capture_relation(terms, Relation::Lt, b, a),
            Relation::Eq | Relation::Ne => false,
        } {
            return Some(false);
        }
        let known = |r, x, y| self.relations.contains(&(r, x, y));
        match relation {
            Relation::Eq => {
                if known(Relation::Eq, b, a) {
                    return Some(true);
                }
                if known(Relation::Ne, a, b)
                    || known(Relation::Ne, b, a)
                    || known(Relation::Lt, a, b)
                    || known(Relation::Lt, b, a)
                {
                    return Some(false);
                }
            }
            Relation::Ne => return self.truth(terms, Relation::Eq, a, b).map(|v| !v),
            Relation::Lt => {
                if known(Relation::Le, b, a)
                    || known(Relation::Lt, b, a)
                    || known(Relation::Eq, a, b)
                    || known(Relation::Eq, b, a)
                {
                    return Some(false);
                }
                if terms.value(b) == Some(0) {
                    return Some(false);
                }
            }
            Relation::Le => {
                if known(Relation::Lt, a, b)
                    || known(Relation::Eq, a, b)
                    || known(Relation::Eq, b, a)
                    || terms.value(a) == Some(0)
                    || terms.value(b) == Some(u64::MAX)
                {
                    return Some(true);
                }
                if known(Relation::Lt, b, a) {
                    return Some(false);
                }
            }
        }
        None
    }
    fn assume(&mut self, terms: &Terms, relation: Relation, a: Id, b: Id) -> Option<bool> {
        if self.truth(terms, relation, a, b) == Some(false) {
            return Some(false);
        }
        if self.relations.len() >= MAX_RELATIONS {
            return None;
        }
        self.relations.insert((relation, a, b));
        Some(true)
    }
    fn condition(&mut self, terms: &Terms, condition: Id, yes: bool) -> Option<bool> {
        // Eager Boolean operands have already executed. Only the selected
        // result's logically forced operands supply facts: true AND / false
        // OR. The other cases cannot choose either operand. Repeated DAG
        // nodes are visited once under the existing term allowance.
        let mut pending = vec![(condition, yes)];
        let mut visited = BTreeSet::new();
        while let Some((condition, yes)) = pending.pop() {
            if !visited.insert((condition, yes)) {
                continue;
            }
            if visited.len() > MAX_TERMS {
                return None;
            }
            match terms.terms[condition] {
                Expr::Constant(value) if (value != 0) != yes => return Some(false),
                Expr::Not(value) => pending.push((value, !yes)),
                Expr::And(a, b) if yes => pending.extend([(a, true), (b, true)]),
                Expr::Or(a, b) if !yes => pending.extend([(a, false), (b, false)]),
                Expr::Compare(mut relation, mut a, mut b) => {
                    if !yes {
                        relation = match relation {
                            Relation::Eq => Relation::Ne,
                            Relation::Ne => Relation::Eq,
                            Relation::Lt => {
                                core::mem::swap(&mut a, &mut b);
                                Relation::Le
                            }
                            Relation::Le => {
                                core::mem::swap(&mut a, &mut b);
                                Relation::Lt
                            }
                        };
                    }
                    if !self.assume(terms, relation, a, b)? {
                        return Some(false);
                    }
                }
                _ => {}
            }
        }
        Some(true)
    }
    fn merge(&mut self, other: &Self, site: usize, terms: &mut Terms) -> Option<()> {
        if let Some(values) = &mut self.snapshots {
            values.retain(|key, value| {
                other.snapshots.as_ref().and_then(|v| v.get(key)) == Some(value)
            });
        }
        let condition = expressions::condition(self, other, terms);
        if self.content_version != other.content_version {
            self.content_version = write_changes::Version::Join(site);
        }
        for (&slot, value) in &mut self.words {
            if other.words.get(&slot) != Some(&*value) {
                let old = *value;
                let phi = terms.intern(Expr::Phi(site, slot))?;
                if let Some((condition, yes)) = condition
                    && let Some(&from) = other.words.get(&slot)
                    && old != phi
                    && from != phi
                {
                    terms.changes.record(
                        phi,
                        write_changes::Value::Select {
                            condition,
                            yes: if yes { old } else { from },
                            no: if yes { from } else { old },
                        },
                    );
                } else {
                    terms.changes.invalidate(phi);
                }
                if let Some(recording) = &mut terms.recording {
                    if let Some((condition, yes)) = condition
                        && let Some(&from) = other.words.get(&slot)
                        && old != phi
                        && from != phi
                    {
                        recording.record(
                            phi,
                            Node::Select {
                                condition,
                                yes: if yes { old } else { from },
                                no: if yes { from } else { old },
                            },
                        );
                    } else {
                        recording.invalidate(phi);
                    }
                }
                *value = phi;
            }
        }
        self.relations.retain(|fact| other.relations.contains(fact));
        self.successful
            .retain(|fact| other.successful.contains(fact));
        self.checked_reads
            .retain(|key, read| other.checked_reads.get(key) == Some(read));
        Some(())
    }
}

#[derive(Default)]
pub(super) struct Proof {
    reused_reads: BTreeMap<usize, Snapshot>,
    ambiguous_reuses: BTreeSet<usize>,
    reused_operands: BTreeMap<(usize, usize), Snapshot>,
    ambiguous_operands: BTreeSet<(usize, usize)>,
    guards: BTreeSet<(usize, u32, usize, u32)>,
    arithmetic: BTreeMap<usize, (Op, Vec<NativeMatchValue>)>,
    reads: BTreeSet<(usize, usize, u32)>,
    increment_writes: BTreeMap<usize, updates::CheckedUpdate>,
    writes: BTreeSet<(usize, usize, u32)>,
    changed_writes: BTreeSet<(usize, usize, u32, u32, u8)>,
    pub(super) completed: Option<Completed>,
    pub(super) expressions: Option<Graph>,
    pub(super) windows: BTreeSet<Window>,
    pub(super) successors: BTreeMap<usize, u32>,
    pub(super) predicates: BTreeMap<usize, bool>,
    pub(super) flat_arrays: BTreeSet<usize>,
    checks: BTreeSet<(usize, Check)>,
    // This traversal's ordinary, post-copy backedge identities. It is not
    // combined by extend, which may aggregate facts from different scopes.
    pub(super) query_upper: BTreeMap<(usize, usize), u64>,
    unchanged_at_latch: BTreeSet<usize>,
}
impl Proof {
    pub(super) fn extend_increment_writes(&mut self, other: Self) {
        self.increment_writes.extend(other.increment_writes);
    }
    pub(super) fn extend(&mut self, mut other: Self) {
        self.extend_reuses(&mut other);
        self.writes.extend(other.writes);
        self.changed_writes.extend(other.changed_writes);
        self.guards.extend(other.guards);
        self.arithmetic.extend(other.arithmetic);
        self.reads.extend(other.reads);
        self.increment_writes.extend(other.increment_writes);
        self.windows.extend(other.windows);
        self.successors.extend(other.successors);
        self.predicates.extend(other.predicates);
        self.checks.extend(other.checks);
        for (key, upper) in other.query_upper {
            self.query_upper
                .entry(key)
                .and_modify(|old| *old = (*old).max(upper))
                .or_insert(upper);
        }
    }

    pub fn check(&self, site: usize, check: Check) -> bool {
        self.checks.contains(&(site, check))
    }

    pub fn guard(
        &self,
        site: usize,
        source: NativeMatchValue,
        offset: usize,
        ordinal: u32,
    ) -> bool {
        self.guards.contains(&(site, source.get(), offset, ordinal))
    }
    pub fn arithmetic(&self, site: usize, op: Op, operands: &[NativeMatchValue]) -> bool {
        self.arithmetic
            .get(&site)
            .is_some_and(|(proved, inputs)| *proved == op && inputs == operands)
    }
    pub fn changes(&self, site: usize, array: usize, index: u32, incoming: u32, width: u8) -> bool {
        self.changed_writes
            .contains(&(site, array, index, incoming, width))
    }
    pub fn write(&self, site: usize, array: usize, index: u32) -> bool {
        self.writes.contains(&(site, array, index))
    }
    pub fn read(&self, site: usize, array: usize, index: u32) -> bool {
        self.reads.contains(&(site, array, index))
    }
    pub fn increment_write(
        &self,
        site: usize,
        array: usize,
        index: u32,
        incoming: u32,
    ) -> Option<&updates::CheckedUpdate> {
        self.increment_writes.get(&site).filter(|update| {
            update.array == array && update.index == index && update.incoming == incoming
        })
    }
}

#[cfg(test)]
mod tests;
