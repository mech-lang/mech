//! Conditional affine windows, admitted once for this child activation.
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
pub(in crate::native_artifact::program::codegen) struct Window {
    pub induction: usize,
    pub count: usize,
    pub array: usize,
    pub delta: u64,
    pub reverse: bool,
}

impl Window {
    pub fn adjustment(self) -> u64 {
        self.delta - u64::from(self.reverse)
    }
}

fn forward(terms: &Terms, id: Id) -> Option<(Id, u64)> {
    if matches!(terms.terms[id], Expr::Initial(_)) {
        return Some((id, 0));
    }
    let (Expr::Wrapping(Arithmetic::Add, a, b) | Expr::Arithmetic(Arithmetic::Add, a, b)) =
        terms.terms[id]
    else {
        return None;
    };
    let (base, delta) = if let Some(delta) = terms.value(a) {
        (b, delta)
    } else {
        (a, terms.value(b)?)
    };
    (delta > 0).then_some((base, delta))
}

fn access(terms: &Terms, index: Id, length: Id) -> Option<(Id, u64, bool)> {
    if let Some((base, delta)) = forward(terms, index) {
        return Some((base, delta, false));
    }
    let (Expr::Wrapping(Arithmetic::Sub, n, at) | Expr::Arithmetic(Arithmetic::Sub, n, at)) =
        terms.terms[index]
    else {
        return None;
    };
    if n != length {
        return None;
    }
    let (base, delta) = forward(terms, at)?;
    if delta == 0 {
        return None;
    }
    Some((base, delta, true))
}

impl State {
    pub(super) fn discover_window(&self, terms: &Terms, index: Id, length: Id) -> Option<Window> {
        let Expr::ArrayLength(array) = terms.terms[length] else {
            return None;
        };
        let (base, delta, reverse) = access(terms, index, length)?;
        let Expr::Initial(induction) = terms.terms[base] else {
            return None;
        };
        self.relations.iter().find_map(|&(relation, k, count)| {
            if relation != Relation::Lt || k != base {
                return None;
            }
            let Expr::Initial(count) = terms.terms[count] else {
                return None;
            };
            Some(Window {
                induction,
                count,
                array,
                delta,
                reverse,
            })
        })
    }

    pub(super) fn window_holds(&self, terms: &Terms, index: Id, length: Id) -> bool {
        if self.windows.is_empty() {
            return false;
        }
        let Expr::ArrayLength(array) = terms.terms[length] else {
            return false;
        };
        let Some((base, delta, reverse)) = access(terms, index, length) else {
            return false;
        };
        let Expr::Initial(induction) = terms.terms[base] else {
            return false;
        };
        self.windows.iter().any(|window| {
            window.induction == induction
                && window.array == array
                && window.delta == delta
                && window.reverse == reverse
                && terms
                    .interned
                    .get(&Expr::Initial(window.count))
                    .is_some_and(|&count| {
                        // The entry admits the complete interval, but no fact
                        // applies until this visit selects the original k<count.
                        self.truth(terms, Relation::Lt, base, count) == Some(true)
                    })
        })
    }
}

pub(super) fn writes(step: Step<'_>, written: &mut BTreeSet<usize>) -> Option<()> {
    match step {
        Step::Scalar(Scalar::Copy { output, .. } | Scalar::Primitive { output, .. }) => {
            written.insert(output.get() as usize);
        }
        Step::Aggregate(layout, action) => match action {
            Aggregate::Observe { output, .. } => {
                written.insert(output.get() as usize);
            }
            Aggregate::Pack { output, .. }
            | Aggregate::Copy { output, .. }
            | Aggregate::Project { output, .. } => {
                let (at, count) = layout.slot(*output)?;
                written.extend(at..at + count);
            }
        },
        Step::Assign { moves, .. } => written.extend(moves.into_iter().map(|(to, _)| to)),
        Step::Define(output) | Step::Length { output, .. } | Step::Read { output, .. } => {
            written.insert(output);
        }
        Step::Scalar(Scalar::None) | Step::Write { .. } | Step::Replace | Step::Keep => {}
    }
    Some(())
}
