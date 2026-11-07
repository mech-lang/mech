//! Whole-child totality and an exact unit-step, two-state completion proof.
//! This is source analysis. Machine check elimination supplies no authority.
use super::*;

#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact::program::codegen) struct Candidate {
    pub tag: usize,
    pub active: u64,
    pub terminal: u64,
    pub induction: usize,
    pub count: usize,
    pub next_tag: usize,
    pub next_induction: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub(in crate::native_artifact::program::codegen) struct Completed(Candidate);
impl Completed {
    pub fn candidate(&self) -> &Candidate {
        &self.0
    }
}

pub(super) struct Collector {
    candidate: Candidate,
    valid: bool,
    advance: bool,
    terminal: bool,
    exit: bool,
}
impl Collector {
    pub fn new(candidate: Candidate) -> Self {
        Self {
            candidate,
            valid: true,
            advance: false,
            terminal: false,
            exit: false,
        }
    }

    pub fn action(&mut self, terms: &Terms, states: &[Option<State>], step: &Step<'_>, term: Term) {
        self.valid &= !matches!(term, Term::Scalar(Branch::Reject));
        for state in states.iter().flatten() {
            self.valid &= match step {
                Step::Aggregate(layout, action) => {
                    if let Aggregate::Project { source, guards, .. }
                    | Aggregate::Observe { source, guards, .. } = action
                    {
                        guards.iter().all(|&(offset, ordinal)| {
                            field(layout, *source, offset)
                                .and_then(|at| state.read(at))
                                .zip(
                                    terms
                                        .interned
                                        .get(&Expr::Constant(u64::from(ordinal)))
                                        .copied(),
                                )
                                .is_some_and(|(tag, expected)| {
                                    state.truth(terms, Relation::Eq, tag, expected) == Some(true)
                                })
                        })
                    } else {
                        true
                    }
                }
                Step::Scalar(Scalar::Primitive { op, operands, .. }) => {
                    let args = operands
                        .iter()
                        .map(|v| state.read(v.get() as usize))
                        .collect::<Option<Vec<_>>>();
                    args.is_some_and(|args| total(state, terms, *op, &args))
                }
                Step::Read { array, index, .. } => state
                    .read(*index as usize)
                    .zip(terms.interned.get(&Expr::ArrayLength(*array)).copied())
                    .is_some_and(|(index, length)| state.readable(terms, index, length)),
                Step::Define(_) | Step::Replace => false,
                Step::Scalar(_) | Step::Assign { .. } | Step::Length { .. } | Step::Keep => true,
            };
        }
    }

    fn initial(&self, terms: &Terms, slot: usize) -> Option<Id> {
        terms.interned.get(&Expr::Initial(slot)).copied()
    }
    fn entered(&self, state: &State, terms: &Terms, ordinal: u64) -> bool {
        self.initial(terms, self.candidate.tag)
            .zip(terms.interned.get(&Expr::Constant(ordinal)).copied())
            .is_some_and(|(tag, expected)| {
                state.truth(terms, Relation::Eq, tag, expected) == Some(true)
            })
    }

    pub fn latch(&mut self, terms: &Terms, states: &[Option<State>]) {
        for state in states.iter().flatten() {
            let proved = (|| {
                if !self.entered(state, terms, self.candidate.active) {
                    return None;
                }
                let k = self.initial(terms, self.candidate.induction)?;
                let count = self.initial(terms, self.candidate.count)?;
                let tag = state.constant(terms, state.read(self.candidate.next_tag)?)?;
                let next = state.read(self.candidate.next_induction)?;
                if tag == self.candidate.active {
                    let (Expr::Arithmetic(Arithmetic::Add, a, b)
                    | Expr::Wrapping(Arithmetic::Add, a, b)) = terms.terms[next]
                    else {
                        return None;
                    };
                    if a != k
                        || terms.value(b) != Some(1)
                        || state.truth(terms, Relation::Lt, k, count) != Some(true)
                        || !state.safe(terms, Arithmetic::Add, a, b)
                    {
                        return None;
                    }
                    self.advance = true;
                } else if tag == self.candidate.terminal
                    && next == k
                    && state.truth(terms, Relation::Lt, k, count) == Some(false)
                {
                    self.terminal = true;
                } else {
                    return None;
                }
                Some(())
            })();
            self.valid &= proved.is_some();
        }
    }
    pub fn exit(&mut self, terms: &Terms, states: &[Option<State>]) {
        for state in states.iter().flatten() {
            self.valid &= self.entered(state, terms, self.candidate.terminal);
            self.exit = true;
        }
    }
    pub fn finish(self) -> Option<Completed> {
        (self.valid && self.advance && self.terminal && self.exit)
            .then_some(Completed(self.candidate))
    }
}

fn total(state: &State, terms: &Terms, op: Op, args: &[Id]) -> bool {
    use Op::*;
    match op {
        U64Add | U64Sub => state.safe(
            terms,
            if op == U64Add {
                Arithmetic::Add
            } else {
                Arithmetic::Sub
            },
            args[0],
            args[1],
        ),
        U64Mul => state
            .upper(terms, args[0])
            .checked_mul(state.upper(terms, args[1]))
            .is_some(),
        U64Div | U64Rem => state.constant(terms, args[1]).is_some_and(|n| n != 0),
        U16ToU8 | U32ToU8 | U32ToU16 | U64ToU8 | U64ToU16 | U64ToU32 => {
            state.upper(terms, args[0]) <= op.signature().1.unsigned_maximum().unwrap()
        }
        U8ToU16 | U8ToU32 | U8ToU64 | U16ToU32 | U16ToU64 | U32ToU64 | U64BitAnd | U64BitOr
        | U64BitXor | U64BitNot | U64ShlWrap | U64ShrWrap | U64WrappingAdd | U64WrappingSub
        | U64WrappingMul | U64Clz | U64Ctz => true,
        _ => op.signature().1 == NativeScalarKind::Bool,
    }
}

#[cfg(test)]
#[test]
fn completion_does_not_get_totality_from_a_successful_replacement() {
    let mut collector = Collector::new(Candidate {
        tag: 0,
        active: 0,
        terminal: 1,
        induction: 1,
        count: 2,
        next_tag: 3,
        next_induction: 4,
    });
    collector.action(
        &Terms::default(),
        &[Some(State::default())],
        &Step::Replace,
        Term::Scalar(Branch::Jump(1)),
    );
    assert!(
        !collector.valid,
        "a retained local fact does not make a checked write total"
    );
}

#[cfg(test)]
#[test]
fn completion_keeps_unknown_and_overflowing_source_arithmetic_fallible() {
    let mut terms = Terms::default();
    let k = terms.intern(Expr::Initial(0)).unwrap();
    let count = terms.intern(Expr::Initial(1)).unwrap();
    let one = terms.constant(1).unwrap();
    let zero = terms.constant(0).unwrap();
    let masked = terms.intern(Expr::Bounded(2, 3, u32::MAX as u64)).unwrap();
    let mut state = State::default();
    assert!(!total(&state, &terms, Op::U64Add, &[k, one]));
    assert!(!total(&state, &terms, Op::U64Div, &[k, count]));
    assert!(!total(&state, &terms, Op::U64Div, &[k, zero]));
    assert!(total(&state, &terms, Op::U64Div, &[k, one]));
    assert!(!total(&state, &terms, Op::U64ToU32, &[k]));
    assert!(total(&state, &terms, Op::U64ToU32, &[masked]));
    state.assume(&terms, Relation::Lt, k, count).unwrap();
    assert!(total(&state, &terms, Op::U64Add, &[k, one]));
}

#[cfg(test)]
#[test]
fn completion_exhausts_normalized_predicates_but_preserves_raw_boolean_rejects() {
    let mut terms = Terms::default();
    let zero = terms.constant(0).unwrap();
    let one = terms.constant(1).unwrap();
    let predicate = terms.intern(Expr::Boolean(1, 2)).unwrap();
    let raw = terms.intern(Expr::Initial(3)).unwrap();
    let mut normalized = State::default();
    assert!(normalized.literal(&terms, predicate, one, false).unwrap());
    assert!(!normalized.literal(&terms, predicate, zero, false).unwrap());
    let mut unknown = State::default();
    assert!(unknown.literal(&terms, raw, one, false).unwrap());
    assert!(unknown.literal(&terms, raw, zero, false).unwrap());
}
