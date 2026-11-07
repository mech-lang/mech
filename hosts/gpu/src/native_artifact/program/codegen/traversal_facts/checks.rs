//! Queries about current words, evaluated after a source action's transfer.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
pub(in crate::native_artifact::program::codegen) enum Operand {
    Word(usize),
    Constant(u64),
    Length(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
pub(in crate::native_artifact::program::codegen) enum Check {
    Le(Operand, Operand),
    Addition {
        word: usize,
        extra: u64,
    },
    Fits {
        array: usize,
        base: Option<usize>,
        count: usize,
        extra: u64,
    },
}

pub(in crate::native_artifact::program::codegen) type Queries = BTreeMap<usize, BTreeSet<Check>>;

impl Operand {
    fn value(self, state: &State, terms: &mut Terms) -> Option<Id> {
        match self {
            Self::Word(word) => state.read(word),
            Self::Constant(value) => terms.constant(value),
            Self::Length(array) => terms.intern(Expr::ArrayLength(array)),
        }
    }
}

impl Check {
    pub(super) fn holds(self, state: &State, terms: &mut Terms) -> Option<bool> {
        match self {
            Self::Le(a, b) => {
                let (a, b) = (a.value(state, terms)?, b.value(state, terms)?);
                Some(
                    state.truth(terms, Relation::Le, a, b) == Some(true)
                        || terms
                            .value(b)
                            .is_some_and(|bound| state.upper(terms, a) <= bound)
                        || state
                            .constant(terms, a)
                            .is_some_and(|bound| bound <= state.lower(terms, b)),
                )
            }
            Self::Addition { word, extra } => {
                let (word, extra) = (state.read(word)?, terms.constant(extra)?);
                Some(state.safe(terms, Arithmetic::Add, word, extra))
            }
            Self::Fits {
                array,
                base,
                count,
                extra,
            } => {
                let length = terms.intern(Expr::ArrayLength(array))?;
                let mut end = state.read(count)?;
                if extra != 0 {
                    let addend = terms.constant(extra)?;
                    if !state.safe(terms, Arithmetic::Add, end, addend) {
                        return Some(false);
                    }
                    let forward = terms.intern(Expr::Arithmetic(Arithmetic::Add, end, addend))?;
                    let reverse = terms.intern(Expr::Arithmetic(Arithmetic::Add, addend, end))?;
                    if base.is_none() {
                        return Some(
                            state.truth(terms, Relation::Le, forward, length) == Some(true)
                                || state.truth(terms, Relation::Le, reverse, length) == Some(true),
                        );
                    }
                    end = forward;
                }
                if let Some(base) = base {
                    let base = state.read(base)?;
                    if !state.safe(terms, Arithmetic::Add, end, base) {
                        return Some(false);
                    }
                    let forward = terms.intern(Expr::Arithmetic(Arithmetic::Add, end, base))?;
                    let reverse = terms.intern(Expr::Arithmetic(Arithmetic::Add, base, end))?;
                    return Some(
                        state.truth(terms, Relation::Le, forward, length) == Some(true)
                            || state.truth(terms, Relation::Le, reverse, length) == Some(true),
                    );
                }
                Some(state.truth(terms, Relation::Le, end, length) == Some(true))
            }
        }
    }
}
