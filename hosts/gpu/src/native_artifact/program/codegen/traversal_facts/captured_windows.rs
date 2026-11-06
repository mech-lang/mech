//! An activation-current base+induction window, admitted before this version.
use super::*;

#[derive(Clone, Copy, Debug, serde::Serialize)]
pub(in crate::native_artifact::program::codegen) struct CapturedWindow {
    pub array: usize,
    pub base: usize,
    pub induction: usize,
    pub limit: usize,
    pub width: u64,
}

impl CapturedWindow {
    fn initial(self, terms: &Terms, slot: usize) -> Option<Id> {
        terms.interned.get(&Expr::Initial(slot)).copied()
    }

    fn active(self, state: &State, terms: &Terms) -> bool {
        let Some(k) = self.initial(terms, self.induction) else {
            return false;
        };
        let Some(limit) = self.initial(terms, self.limit) else {
            return false;
        };
        let Some(&width) = terms.interned.get(&Expr::Constant(self.width)) else {
            return false;
        };
        let Some(&end) = terms
            .interned
            .get(&Expr::Arithmetic(Arithmetic::Add, k, width))
        else {
            return false;
        };
        state.relations.contains(&(Relation::Le, end, limit))
            || state.relations.contains(&(Relation::Lt, end, limit))
            || state.relations.contains(&(Relation::Eq, end, limit))
    }

    /// Only mathematical additions survive normalization here. A wrapped
    /// expression cannot grant a readable window merely by matching modulo64.
    fn index(self, terms: &Terms, value: Id, fuel: usize) -> Option<u64> {
        if fuel == 0 {
            return None;
        }
        let Expr::Arithmetic(Arithmetic::Add, a, b) = terms.terms[value] else {
            return None;
        };
        self.sum(terms, a, b, fuel - 1)
    }

    fn sum(self, terms: &Terms, a: Id, b: Id, fuel: usize) -> Option<u64> {
        let (base, k) = (
            self.initial(terms, self.base)?,
            self.initial(terms, self.induction)?,
        );
        if (a == base && b == k) || (a == k && b == base) {
            return Some(0);
        }
        if let Some(n) = terms.value(b) {
            return self.index(terms, a, fuel)?.checked_add(n);
        }
        self.index(terms, b, fuel)?.checked_add(terms.value(a)?)
    }
}

impl State {
    pub(super) fn capture_upper(&self, terms: &Terms, value: Id) -> Option<u64> {
        self.captures
            .iter()
            .filter_map(|window| {
                (window.initial(terms, window.induction) == Some(value)
                    || window.initial(terms, window.limit) == Some(value))
                .then_some(u64::MAX - window.width)
            })
            .min()
    }

    pub(super) fn capture_add(&self, terms: &Terms, a: Id, b: Id) -> bool {
        self.captures.iter().any(|&window| {
            window.sum(terms, a, b, 8).is_some_and(|delta| {
                delta <= window.width && (delta == 0 || window.active(self, terms))
            })
        })
    }

    pub(super) fn capture_relation(&self, terms: &Terms, relation: Relation, a: Id, b: Id) -> bool {
        if !matches!(relation, Relation::Le | Relation::Lt) {
            return false;
        }
        self.captures.iter().any(|&window| {
            let (array, tail) = match terms.terms[b] {
                Expr::ArrayLength(array) => (array, 0),
                Expr::Arithmetic(Arithmetic::Sub, length, amount) => {
                    let Expr::ArrayLength(array) = terms.terms[length] else {
                        return false;
                    };
                    let Some(amount) = terms.value(amount) else {
                        return false;
                    };
                    (array, amount)
                }
                _ => return false,
            };
            if array != window.array {
                return false;
            }
            let fits = |n| {
                if relation == Relation::Le {
                    n <= window.width
                } else {
                    n < window.width
                }
            };
            if let Some(amount) = terms.value(a) {
                return amount.checked_add(tail).is_some_and(fits);
            }
            window.active(self, terms)
                && window
                    .index(terms, a, 8)
                    .and_then(|delta| delta.checked_add(tail))
                    .is_some_and(fits)
        })
    }
}

#[cfg(test)]
#[test]
fn captured_window_facts_require_current_selected_mathematical_coordinates() {
    let mut terms = Terms::default();
    let (base, k, limit) = (
        terms.intern(Expr::Initial(10)).unwrap(),
        terms.intern(Expr::Initial(20)).unwrap(),
        terms.intern(Expr::Initial(30)).unwrap(),
    );
    let width = terms.constant(8).unwrap();
    let one = terms.constant(1).unwrap();
    let nine = terms.constant(9).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let other_length = terms.intern(Expr::ArrayLength(1)).unwrap();
    let available = terms
        .intern(Expr::Arithmetic(Arithmetic::Sub, length, width))
        .unwrap();
    let at = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, base, k))
        .unwrap();
    let wrapped = terms
        .intern(Expr::Wrapping(Arithmetic::Add, base, k))
        .unwrap();
    let end = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, k, width))
        .unwrap();
    let mut state = State::default();
    state.captures.push(CapturedWindow {
        array: 0,
        base: 10,
        induction: 20,
        limit: 30,
        width: 8,
    });
    state.assume(&terms, Relation::Le, k, limit).unwrap();
    assert!(state.safe(&terms, Arithmetic::Add, base, k));
    assert!(state.safe(&terms, Arithmetic::Add, k, width));
    assert_eq!(state.truth(&terms, Relation::Le, width, length), Some(true));
    assert_eq!(
        state.truth(&terms, Relation::Lt, length, width),
        Some(false)
    );
    assert!(!state.safe(&terms, Arithmetic::Add, at, one));
    assert_ne!(state.truth(&terms, Relation::Le, at, available), Some(true));
    state.assume(&terms, Relation::Le, end, limit).unwrap();
    assert!(state.safe(&terms, Arithmetic::Add, at, one));
    assert!(!state.safe(&terms, Arithmetic::Add, at, nine));
    assert_eq!(state.truth(&terms, Relation::Le, at, available), Some(true));
    assert_eq!(
        state.truth(&terms, Relation::Lt, available, at),
        Some(false)
    );
    assert_eq!(state.truth(&terms, Relation::Lt, at, length), Some(true));
    assert_ne!(
        state.truth(&terms, Relation::Lt, at, other_length),
        Some(true)
    );
    assert_ne!(
        state.truth(&terms, Relation::Lt, wrapped, length),
        Some(true)
    );
}
