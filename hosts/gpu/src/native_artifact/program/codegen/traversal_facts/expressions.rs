//! Value expressions recorded by the same source traversal as the safety proof.
//! These describe values, never permission. Only a completed traversal exports
//! them, and an ambiguous definition or join remains unknown.
use super::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(in crate::native_artifact::program::codegen) enum Node {
    Unknown,
    Constant(u64),
    Initial(usize),
    Length(usize),
    Scalar(Op, Vec<Id>),
    Read { array: usize, index: Id },
    Select { condition: Id, yes: Id, no: Id },
}

#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact::program::codegen) struct Graph {
    pub nodes: Vec<Node>,
    pub advances: Vec<BTreeMap<usize, Id>>,
    /// Current payload values on each active-to-terminal latch edge.
    pub exhausted: Vec<BTreeMap<usize, Id>>,
    pub written: BTreeSet<usize>,
    pub terminal_inputs: Option<BTreeSet<Id>>,
}

#[derive(Default)]
pub(super) struct Recording {
    definitions: BTreeMap<Id, Node>,
    ambiguous: BTreeSet<Id>,
    advances: Vec<BTreeMap<usize, Id>>,
    exhausted: Vec<BTreeMap<usize, Id>>,
    overflowed: bool,
    candidate: Option<Candidate>,
    terminal_inputs: Option<BTreeSet<Id>>,
}

impl Recording {
    pub fn new(candidate: Candidate) -> Self {
        Self {
            candidate: Some(candidate),
            terminal_inputs: Some(BTreeSet::new()),
            ..Self::default()
        }
    }

    pub fn terminal(&mut self, terms: &Terms, states: &[Option<State>], step: &Step<'_>) {
        let Some(candidate) = self.candidate.as_ref() else {
            return;
        };
        let tag = terms.interned.get(&Expr::Initial(candidate.tag));
        let terminal = terms.interned.get(&Expr::Constant(candidate.terminal));
        let (Some(&tag), Some(&terminal)) = (tag, terminal) else {
            self.terminal_inputs = None;
            return;
        };
        for state in states
            .iter()
            .flatten()
            .filter(|s| s.truth(terms, Relation::Eq, tag, terminal) == Some(true))
        {
            let values = (|| {
                let slots = match step {
                    Step::Scalar(Scalar::Copy { source, .. }) => vec![source.get() as usize],
                    Step::Scalar(Scalar::Primitive { operands, .. }) => {
                        operands.iter().map(|v| v.get() as usize).collect()
                    }
                    Step::Aggregate(layout, action) => {
                        let mut slots = BTreeSet::new();
                        match action {
                            Aggregate::Pack { words, .. } => {
                                for word in words {
                                    match word {
                                        Word::Scalar(v) => {
                                            slots.insert(v.get() as usize);
                                        }
                                        Word::Field(v, i) => {
                                            slots.insert(field(layout, *v, *i)?);
                                        }
                                        Word::Constant(_) => {}
                                    }
                                }
                            }
                            Aggregate::Copy { source, output } => {
                                for i in 0..layout.slot(*output)?.1 {
                                    slots.insert(field(layout, *source, i)?);
                                }
                            }
                            Aggregate::Project {
                                source,
                                offset,
                                guards,
                                output,
                            } => {
                                for i in 0..layout.slot(*output)?.1 {
                                    slots.insert(field(layout, *source, offset.checked_add(i)?)?);
                                }
                                for (i, _) in guards {
                                    slots.insert(field(layout, *source, *i)?);
                                }
                            }
                            Aggregate::Observe {
                                source,
                                offset,
                                guards,
                                ..
                            } => {
                                slots.insert(field(layout, *source, *offset)?);
                                for (i, _) in guards {
                                    slots.insert(field(layout, *source, *i)?);
                                }
                            }
                        }
                        slots.into_iter().collect()
                    }
                    Step::Assign { moves, .. } => moves.iter().map(|(_, from)| *from).collect(),
                    Step::Read { index, .. } => vec![*index as usize],
                    Step::Define(_) | Step::Replace => return None,
                    Step::Scalar(Scalar::None) | Step::Length { .. } | Step::Keep => vec![],
                };
                slots
                    .into_iter()
                    .map(|slot| state.read(slot))
                    .collect::<Option<Vec<_>>>()
            })();
            match (&mut self.terminal_inputs, values) {
                (Some(inputs), Some(values)) => inputs.extend(values),
                _ => self.terminal_inputs = None,
            }
        }
    }

    pub fn terminal_condition(&mut self, terms: &Terms, states: &[Option<State>], term: Term) {
        let Some(candidate) = self.candidate.as_ref() else {
            return;
        };
        let (Some(&tag), Some(&ordinal)) = (
            terms.interned.get(&Expr::Initial(candidate.tag)),
            terms.interned.get(&Expr::Constant(candidate.terminal)),
        ) else {
            self.terminal_inputs = None;
            return;
        };
        let operands = match term {
            Term::Scalar(Branch::Branch { condition, .. }) => vec![condition],
            Term::Scalar(Branch::LiteralBranch { left, right, .. }) => vec![left, right],
            _ => vec![],
        };
        for state in states
            .iter()
            .flatten()
            .filter(|s| s.truth(terms, Relation::Eq, tag, ordinal) == Some(true))
        {
            let values = operands
                .iter()
                .map(|v| state.read(v.get() as usize))
                .collect::<Option<Vec<_>>>();
            match (&mut self.terminal_inputs, values) {
                (Some(inputs), Some(values)) => inputs.extend(values),
                _ => self.terminal_inputs = None,
            }
        }
    }
    pub fn record(&mut self, id: Id, node: Node) {
        if self.ambiguous.contains(&id) {
            return;
        }
        if self.definitions.get(&id).is_some_and(|old| *old != node) {
            self.invalidate(id);
        } else {
            self.definitions.insert(id, node);
        }
    }

    pub fn invalidate(&mut self, id: Id) {
        self.definitions.remove(&id);
        self.ambiguous.insert(id);
    }

    pub fn latch(&mut self, candidate: &Candidate, state: &State, terms: &Terms) {
        if self.overflowed {
            return;
        }
        let tag = state
            .read(candidate.next_tag)
            .and_then(|v| state.constant(terms, v));
        let active = tag == Some(candidate.active);
        if !active && tag != Some(candidate.terminal) {
            return;
        }
        let count = if active {
            self.advances.len()
        } else {
            self.exhausted.len()
        };
        if count == MAX_PARTITIONS {
            self.overflowed = true;
            self.advances.clear();
            self.exhausted.clear();
        } else if active {
            self.advances.push(state.words.clone());
        } else {
            self.exhausted.push(state.words.clone());
        }
    }

    pub fn finish(self, terms: &Terms, written: BTreeSet<usize>) -> Graph {
        let nodes = terms
            .terms
            .iter()
            .enumerate()
            .map(|(id, term)| match *term {
                Expr::Constant(value) => Node::Constant(value),
                Expr::Initial(word) => Node::Initial(word),
                Expr::ArrayLength(array) => Node::Length(array),
                // Completion proves these arithmetic operations total on
                // every reached edge. Their word value has ring semantics.
                Expr::Arithmetic(op, a, b) | Expr::Wrapping(op, a, b) => Node::Scalar(
                    match op {
                        Arithmetic::Add => Op::U64WrappingAdd,
                        Arithmetic::Sub => Op::U64WrappingSub,
                    },
                    vec![a, b],
                ),
                Expr::Compare(relation, a, b) => Node::Scalar(
                    match relation {
                        Relation::Eq => Op::U64Eq,
                        Relation::Ne => Op::U64Ne,
                        Relation::Lt => Op::U64Lt,
                        Relation::Le => Op::U64Le,
                    },
                    vec![a, b],
                ),
                Expr::Not(value) => Node::Scalar(Op::BoolNot, vec![value]),
                Expr::And(a, b) => Node::Scalar(Op::BoolAnd, vec![a, b]),
                Expr::Or(a, b) => Node::Scalar(Op::BoolOr, vec![a, b]),
                _ => self.definitions.get(&id).cloned().unwrap_or(Node::Unknown),
            })
            .collect();
        Graph {
            nodes,
            advances: self.advances,
            exhausted: self.exhausted,
            written,
            terminal_inputs: self.terminal_inputs,
        }
    }
}

/// A normalized predicate that is true throughout one incoming state and false
/// throughout the other justifies a value select. No speculation or safety fact
/// follows from this description. More complex joins simply lose the recipe.
pub(super) fn condition(a: &State, b: &State, terms: &Terms) -> Option<(Id, bool)> {
    a.relations.iter().find_map(|&(relation, left, right)| {
        if relation != Relation::Eq || !terms.boolean(left) {
            return None;
        }
        let expected = terms.value(right)?;
        if expected > 1 {
            return None;
        }
        let other = b.constant(terms, left)?;
        (other == 1 - expected).then_some((left, expected == 1))
    })
}
