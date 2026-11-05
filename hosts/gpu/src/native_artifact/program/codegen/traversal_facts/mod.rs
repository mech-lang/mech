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
mod flow;
mod operations;
mod scope;
pub(super) use flow::{Step, derive};
use operations::{aggregate, scalar};
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
    Phi(usize, usize),
    Arithmetic(Arithmetic, Id, Id),
    Compare(Relation, Id, Id),
    Not(Id),
}
#[derive(Default)]
struct Terms {
    terms: Vec<Expr>,
    interned: BTreeMap<Expr, Id>,
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
                Expr::Compare(..) | Expr::Constant(0 | 1) => return true,
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
}
impl State {
    fn literal(&mut self, terms: &Terms, a: Id, b: Id, equal: bool) -> Option<bool> {
        if !self.assume(terms, if equal { Relation::Eq } else { Relation::Ne }, a, b)? {
            return Some(false);
        }
        // Only a proven normalized predicate has an exhaustive 0/1 domain.
        // An arbitrary raw Bool/tag may still be 2 or another invalid word.
        let predicate = [(a, b), (b, a)].into_iter().find_map(|(value, literal)| {
            terms
                .boolean(value)
                .then(|| terms.value(literal))
                .flatten()
                .filter(|v| *v <= 1)
                .map(|literal| (value, literal != 0))
        });
        if let Some((value, selected)) = predicate {
            self.condition(terms, value, selected == equal)
        } else {
            Some(true)
        }
    }

    fn readable(&self, terms: &Terms, index: Id, length: Id) -> bool {
        let Expr::Arithmetic(Arithmetic::Add, a, b) = terms.terms[index] else {
            return false;
        };
        let base = if terms.value(a) == Some(1) {
            b
        } else if terms.value(b) == Some(1) {
            a
        } else {
            return false;
        };
        // The expression is mathematical only after its checked operation
        // succeeded or this visit proved its wrapping addition cannot wrap.
        self.truth(terms, Relation::Lt, base, length) == Some(true)
    }
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
    fn condition(&mut self, terms: &Terms, mut condition: Id, mut yes: bool) -> Option<bool> {
        // Not chains are acyclic interned expressions; bound the walk anyway.
        for _ in 0..MAX_TERMS {
            let Expr::Not(value) = terms.terms[condition] else {
                break;
            };
            condition = value;
            yes = !yes;
        }
        match terms.terms[condition] {
            Expr::Constant(value) => Some((value != 0) == yes),
            Expr::Not(_) => None,
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
                self.assume(terms, relation, a, b)
            }
            _ => Some(true),
        }
    }
    fn upper(&self, terms: &Terms, value: Id) -> u64 {
        if let Some(value) = self.constant(terms, value) {
            return value;
        }
        // A single established strict upper edge suffices for x+1. Do not
        // recursively chase relations or infer a bound from one phi arm.
        let mut upper = u64::MAX;
        for &(relation, a, b) in &self.relations {
            if a == value {
                match relation {
                    Relation::Eq | Relation::Le => {
                        upper = upper.min(terms.value(b).unwrap_or(u64::MAX));
                    }
                    Relation::Lt => {
                        upper = upper.min(terms.value(b).unwrap_or(u64::MAX).saturating_sub(1));
                    }
                    Relation::Ne => {}
                }
            } else if b == value && relation == Relation::Eq {
                upper = upper.min(terms.value(a).unwrap_or(u64::MAX));
            }
        }
        upper
    }
    fn safe(&self, terms: &Terms, op: Arithmetic, a: Id, b: Id) -> bool {
        if self.successful.contains(&(op, a, b))
            || (op == Arithmetic::Add && self.successful.contains(&(op, b, a)))
        {
            return true;
        }
        match op {
            Arithmetic::Sub => self.truth(terms, Relation::Le, b, a) == Some(true),
            Arithmetic::Add => {
                if u128::from(self.upper(terms, a)) + u128::from(self.upper(terms, b))
                    <= u128::from(u64::MAX)
                {
                    return true;
                }
                // After a successful checked difference d=n-i, w<=d proves
                // i+w<=n. The exact i identity must match the subtraction.
                self.relations.iter().any(|&(relation, width, difference)| {
                    if !matches!(relation, Relation::Le | Relation::Lt | Relation::Eq) {
                        return false;
                    }
                    let Expr::Arithmetic(Arithmetic::Sub, n, index) = terms.terms[difference]
                    else {
                        return false;
                    };
                    self.successful.contains(&(Arithmetic::Sub, n, index))
                        && ((width == a && index == b) || (width == b && index == a))
                })
            }
        }
    }
    fn merge(&mut self, other: &Self, site: usize, terms: &mut Terms) -> Option<()> {
        for (&slot, value) in &mut self.words {
            if other.words.get(&slot) != Some(&*value) {
                *value = terms.intern(Expr::Phi(site, slot))?;
            }
        }
        self.relations.retain(|fact| other.relations.contains(fact));
        self.successful
            .retain(|fact| other.successful.contains(fact));
        Some(())
    }
}

#[derive(Default)]
pub(super) struct Proof {
    guards: BTreeSet<(usize, u32, usize, u32)>,
    arithmetic: BTreeMap<usize, (Op, Vec<NativeMatchValue>)>,
    reads: BTreeSet<(usize, usize, u32)>,
}
impl Proof {
    pub(super) fn extend(&mut self, other: Self) {
        self.guards.extend(other.guards);
        self.arithmetic.extend(other.arithmetic);
        self.reads.extend(other.reads);
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
    pub fn read(&self, site: usize, array: usize, index: u32) -> bool {
        self.reads.contains(&(site, array, index))
    }
}

#[cfg(test)]
mod tests;
