//! Source-word bounds used by successful traversal and completion proofs.
use super::*;

impl State {
    pub(super) fn upper(&self, terms: &Terms, value: Id) -> u64 {
        if let Some(value) = self.constant(terms, value) {
            return value;
        }
        // A single established strict upper edge suffices for x+1. Do not
        // recursively chase relations or infer a bound from one phi arm.
        let mut upper = match terms.terms[value] {
            Expr::Bounded(_, _, upper) => upper,
            Expr::Compare(..) | Expr::Boolean(..) => 1,
            _ => u64::MAX,
        };
        upper = upper.min(self.capture_upper(terms, value).unwrap_or(u64::MAX));
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
    pub(super) fn safe(&self, terms: &Terms, op: Arithmetic, a: Id, b: Id) -> bool {
        if self.successful.contains(&(op, a, b))
            || (op == Arithmetic::Add && self.successful.contains(&(op, b, a)))
        {
            return true;
        }
        match op {
            Arithmetic::Sub => self.truth(terms, Relation::Le, b, a) == Some(true),
            Arithmetic::Add => {
                if self.capture_add(terms, a, b) {
                    return true;
                }
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
}
