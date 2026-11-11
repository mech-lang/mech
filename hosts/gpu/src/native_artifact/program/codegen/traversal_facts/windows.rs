//! Selected Boolean cases and one-based reads of the same immutable view.
use super::*;

pub(super) fn read_base(terms: &Terms, index: Id) -> Option<Id> {
    let Expr::Arithmetic(Arithmetic::Add, a, b) = terms.terms[index] else {
        return None;
    };
    if terms.value(a) == Some(1) {
        Some(b)
    } else if terms.value(b) == Some(1) {
        Some(a)
    } else {
        None
    }
}

impl State {
    pub(super) fn literal(&mut self, terms: &Terms, a: Id, b: Id, equal: bool) -> Option<bool> {
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
            let selected = selected == equal;
            if let Some(&literal) = terms.interned.get(&Expr::Constant(u64::from(selected))) {
                if !self.assume(terms, Relation::Eq, value, literal)? {
                    return Some(false);
                }
            }
            self.condition(terms, value, selected)
        } else {
            Some(true)
        }
    }

    pub(super) fn readable(&self, terms: &Terms, index: Id, length: Id) -> bool {
        let Some(base) = read_base(terms, index) else {
            return false;
        };
        // The expression is mathematical only after its checked operation
        // succeeded or this visit proved its wrapping addition cannot wrap.
        self.truth(terms, Relation::Lt, base, length) == Some(true)
            || self.upper(terms, base) < self.lower(terms, length)
    }
}
