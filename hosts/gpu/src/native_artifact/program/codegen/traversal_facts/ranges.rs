//! Source-word bounds used by successful traversal and completion proofs.
use super::*;

impl State {
    pub(super) fn lower(&self, terms: &Terms, value: Id) -> u64 {
        self.bounds(terms, value).0
    }

    pub(super) fn upper(&self, terms: &Terms, value: Id) -> u64 {
        self.bounds(terms, value).1
    }

    fn bounds(&self, terms: &Terms, value: Id) -> (u64, u64) {
        // Only arithmetic operands are followed. Their IDs precede their
        // definition, so an ascending pass visits each needed DAG node once.
        // Shared expressions never expand exponentially and no recursion or
        // additional depth allowance limits useful range propagation.
        let mut needed = BTreeSet::new();
        let mut pending = vec![value];
        while let Some(at) = pending.pop() {
            if !needed.insert(at) || self.constant(terms, at).is_some() {
                continue;
            }
            if let Expr::Arithmetic(_, a, b) = terms.terms[at] {
                pending.extend([a, b]);
            }
        }
        let mut ranges = BTreeMap::<Id, (u64, u64)>::new();
        for &value in &needed {
            if let Some(constant) = self.constant(terms, value) {
                ranges.insert(value, (constant, constant));
                continue;
            }
            let mut range = match terms.terms[value] {
                Expr::Bounded(_, _, upper) => (0, upper),
                Expr::Compare(..) | Expr::Boolean(..) | Expr::And(..) | Expr::Or(..) => (0, 1),
                Expr::Arithmetic(op, a, b) => {
                    let (al, ah) = ranges[&a];
                    let (bl, bh) = ranges[&b];
                    match op {
                        Arithmetic::Add => (al.saturating_add(bl), ah.saturating_add(bh)),
                        Arithmetic::Sub => (al.saturating_sub(bh), ah.saturating_sub(bl)),
                    }
                }
                // Wrapping expressions retain identity, not mathematical ranges.
                _ => (0, u64::MAX),
            };
            range.1 = range
                .1
                .min(self.capture_upper(terms, value).unwrap_or(u64::MAX));
            // Only direct constant edges refine the interval. Do not chase a
            // relation cycle or import an unselected predecessor's facts.
            for &(relation, a, b) in &self.relations {
                if a == value {
                    if relation == Relation::Lt {
                        range.1 = range
                            .1
                            .min(terms.value(b).unwrap_or(u64::MAX).saturating_sub(1));
                    }
                    if let Some(bound) = terms.value(b) {
                        match relation {
                            Relation::Eq => {
                                range.0 = range.0.max(bound);
                                range.1 = range.1.min(bound);
                            }
                            Relation::Le => range.1 = range.1.min(bound),
                            Relation::Lt => range.1 = range.1.min(bound.saturating_sub(1)),
                            Relation::Ne => {}
                        }
                    }
                } else if b == value {
                    if let Some(bound) = terms.value(a) {
                        match relation {
                            Relation::Eq => {
                                range.0 = range.0.max(bound);
                                range.1 = range.1.min(bound);
                            }
                            Relation::Le => range.0 = range.0.max(bound),
                            Relation::Lt => range.0 = range.0.max(bound.saturating_add(1)),
                            Relation::Ne => {}
                        }
                    }
                }
            }
            ranges.insert(value, range);
        }
        ranges[&value]
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
