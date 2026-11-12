//! Current-traversal value recipes, never array ownership or bounds authority.
//! A nonzero unsigned modular increment cannot equal the same current element.
use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum Version {
    #[default]
    Entry,
    Write(usize),
    Join(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Value {
    Read {
        array: usize,
        index: Id,
        version: Version,
    },
    Scalar(Op, Vec<Id>),
    Select {
        condition: Id,
        yes: Id,
        no: Id,
    },
}

#[derive(Default)]
pub(super) struct Values {
    nodes: BTreeMap<Id, Value>,
    ambiguous: BTreeSet<Id>,
}

impl Values {
    pub(super) fn value(&self, id: Id) -> Option<&Value> {
        self.nodes.get(&id)
    }
    pub fn record(&mut self, id: Id, value: Value) {
        // Every recipe is a DAG. Multiple partitions or a later predecessor
        // may disagree; no reaching-value claim survives that disagreement.
        let inputs = match &value {
            Value::Read { index, .. } => vec![*index],
            Value::Scalar(_, args) => args.clone(),
            Value::Select { condition, yes, no } => vec![*condition, *yes, *no],
        };
        if inputs.iter().any(|&input| input >= id)
            || self.nodes.get(&id).is_some_and(|old| *old != value)
        {
            self.invalidate(id);
        } else if !self.ambiguous.contains(&id) {
            self.nodes.insert(id, value);
        }
    }

    pub fn invalidate(&mut self, id: Id) {
        self.nodes.remove(&id);
        self.ambiguous.insert(id);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Form {
    Constant(u64),
    Element {
        array: usize,
        index: Id,
        version: Version,
        delta: u64,
    },
}

impl State {
    /// This proof is used only after the original write bounds check succeeds.
    /// It cannot remove that check or any earlier arithmetic/assertion failure.
    pub(super) fn changes(
        &self,
        terms: &Terms,
        array: usize,
        index: Id,
        incoming: Id,
        width: u8,
    ) -> bool {
        let mask = match width {
            1 => u8::MAX as u64,
            2 => u16::MAX as u64,
            4 => u32::MAX as u64,
            8 => u64::MAX,
            _ => return false,
        };
        let select = |condition| self.write_condition(terms, condition, array, index);
        // Visit every needed expression once, in definition order. No recursive
        // expansion, instruction budget change or new analysis depth cap.
        let mut needed = BTreeSet::new();
        let mut pending = vec![incoming];
        while let Some(id) = pending.pop() {
            if !needed.insert(id) || self.constant(terms, id).is_some() {
                continue;
            }
            match terms.changes.nodes.get(&id) {
                Some(Value::Scalar(_, args)) => pending.extend(args),
                Some(Value::Select { condition, yes, no }) => match select(*condition) {
                    Some(true) => pending.push(*yes),
                    Some(false) => pending.push(*no),
                    None => pending.extend([*yes, *no]),
                },
                _ => {}
            }
        }
        let mut forms = BTreeMap::<Id, Option<Form>>::new();
        for id in needed {
            let get = |id| forms.get(&id).copied().flatten();
            let form = if let Some(value) = self.constant(terms, id) {
                Some(Form::Constant(value & mask))
            } else {
                match terms.changes.nodes.get(&id) {
                    Some(Value::Read {
                        array,
                        index,
                        version,
                    }) => Some(Form::Element {
                        array: *array,
                        index: *index,
                        version: *version,
                        delta: 0,
                    }),
                    Some(Value::Scalar(op, args)) => modular(*op, args, mask, get),
                    Some(Value::Select { condition, yes, no }) => match select(*condition) {
                        Some(true) => get(*yes),
                        Some(false) => get(*no),
                        None => get(*yes).filter(|value| Some(*value) == get(*no)),
                    },
                    None => None,
                }
            };
            forms.insert(id, form);
        }
        matches!(forms.get(&incoming), Some(Some(Form::Element {
            array: loaded_array, index: loaded_index, version, delta,
        })) if *loaded_array == array && same_index(terms, *loaded_index, index)
            && *version == self.content_version && *delta != 0)
    }

    fn write_condition(
        &self,
        terms: &Terms,
        mut condition: Id,
        array: usize,
        index: Id,
    ) -> Option<bool> {
        let mut invert = false;
        while let Expr::Not(value) = terms.terms[condition] {
            invert = !invert;
            condition = value;
        }
        let value = match terms.terms[condition] {
            Expr::Compare(relation, a, b) => {
                if let Some(value) = self.truth(terms, relation, a, b) {
                    Some(value)
                } else {
                    // Successful one-based wrapping(a+1) is in 1..=length.
                    // Hence the exact earlier default-read guard a<length held.
                    let (Expr::Arithmetic(Arithmetic::Add, x, y)
                    | Expr::Wrapping(Arithmetic::Add, x, y)) = terms.terms[index]
                    else {
                        return None;
                    };
                    (relation == Relation::Lt
                        && terms.terms[b] == Expr::ArrayLength(array)
                        && ((x == a && terms.value(y) == Some(1))
                            || (y == a && terms.value(x) == Some(1))))
                    .then_some(true)
                }
            }
            _ => self.constant(terms, condition).map(|v| v != 0),
        }?;
        Some(value != invert)
    }
}

// Mathematical and wrapping addition have identical word results on the
// mathematical operation's successful edge. Do not equate arbitrary expressions.
pub(super) fn same_index(terms: &Terms, a: Id, b: Id) -> bool {
    if a == b {
        return true;
    }
    let word = |id| match terms.terms[id] {
        Expr::Arithmetic(op, a, b) | Expr::Wrapping(op, a, b) => Some((op, a, b)),
        _ => None,
    };
    match (word(a), word(b)) {
        (Some((op, x, y)), Some((other, u, v))) if op == other => {
            (x == u && y == v) || (op == Arithmetic::Add && x == v && y == u)
        }
        _ => false,
    }
}

fn modular(op: Op, args: &[Id], mask: u64, get: impl Fn(Id) -> Option<Form>) -> Option<Form> {
    use Op::*;
    match op {
        // All these unsigned casts preserve the word on their successful edge.
        // The original narrowing check still executes before the write.
        U8ToU16 | U8ToU32 | U8ToU64 | U16ToU8 | U16ToU32 | U16ToU64 | U32ToU8 | U32ToU16
        | U32ToU64 | U64ToU8 | U64ToU16 | U64ToU32 => get(args[0]),
        U64BitAnd => {
            let (a, b) = (get(args[0]), get(args[1]));
            match (a, b) {
                (Some(Form::Constant(a)), Some(Form::Constant(b))) => Some(Form::Constant(a & b)),
                (Some(value), Some(Form::Constant(bits)))
                | (Some(Form::Constant(bits)), Some(value))
                    if bits == mask =>
                {
                    Some(value)
                }
                _ => None,
            }
        }
        U16Add | U32Add | U64Add | U64WrappingAdd | U16Sub | U32Sub | U64Sub | U64WrappingSub => {
            let subtract = matches!(op, U16Sub | U32Sub | U64Sub | U64WrappingSub);
            let (mut a, mut b) = (get(args[0])?, get(args[1])?);
            if !subtract && matches!(a, Form::Constant(_)) {
                std::mem::swap(&mut a, &mut b);
            }
            let Form::Constant(b) = b else {
                return None;
            };
            let apply = |a: u64| {
                if subtract {
                    a.wrapping_sub(b) & mask
                } else {
                    a.wrapping_add(b) & mask
                }
            };
            Some(match a {
                Form::Constant(a) => Form::Constant(apply(a)),
                Form::Element {
                    array,
                    index,
                    version,
                    delta,
                } => Form::Element {
                    array,
                    index,
                    version,
                    delta: apply(delta),
                },
            })
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "write_changes_tests.rs"]
mod tests;
