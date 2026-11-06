//! Normalize only the low U32 ring of a source-proved total computation.
use super::*;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(super) enum Cost {
    Constant(u32),
    Word(usize),
    Index,
    Read(Window),
    Add(Box<Self>, Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Xor(Box<Self>, Box<Self>),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(super) enum Item {
    Constant(u64),
    Word(usize),
    Index,
    Length(usize),
    Scalar(Op, Vec<Self>),
}

pub(super) struct Normalize<'a> {
    pub graph: &'a Graph,
    pub candidate: &'a Candidate,
    pub windows: &'a [Window],
    pub fuel: usize,
}

impl Normalize<'_> {
    fn node(&mut self, id: usize) -> Option<&Node> {
        self.fuel = self.fuel.checked_sub(1)?;
        self.graph.nodes.get(id)
    }

    fn capture(&self, word: usize) -> bool {
        !self.graph.written.contains(&word)
    }

    pub fn cost(&mut self, id: usize) -> Option<Cost> {
        Some(match self.node(id)?.clone() {
            Node::Constant(value) => Cost::Constant(value as u32),
            Node::Initial(word) if word == self.candidate.induction => Cost::Index,
            Node::Initial(word) if self.capture(word) => Cost::Word(word),
            Node::Read { array, index } => {
                let affine = self.affine(index, array)?;
                let window = self.windows.iter().find(|w| {
                    w.array == array
                        && affine
                            == if w.reverse {
                                (-1, 1, 1u64.wrapping_sub(w.delta))
                            } else {
                                (1, 0, w.delta.wrapping_add(1))
                            }
                })?;
                Cost::Read(*window)
            }
            Node::Scalar(Op::U32ToU64 | Op::U64ToU32, args) if args.len() == 1 => {
                self.cost(args[0])?
            }
            Node::Scalar(op, args) if args.len() == 2 => {
                let a = Box::new(self.cost(args[0])?);
                let b = Box::new(self.cost(args[1])?);
                match op {
                    Op::U64Add | Op::U64WrappingAdd => Cost::Add(a, b),
                    Op::U64BitAnd if *a == Cost::Constant(u32::MAX) => *b,
                    Op::U64BitAnd if *b == Cost::Constant(u32::MAX) => *a,
                    Op::U64BitAnd => Cost::And(a, b),
                    Op::U64BitOr => Cost::Or(a, b),
                    Op::U64BitXor => Cost::Xor(a, b),
                    _ => return None,
                }
            }
            _ => return None,
        })
    }

    /// (induction coefficient, current-array-length coefficient, word constant).
    /// Equality is a ring identity; window admission separately proves safety.
    fn affine(&mut self, id: usize, array: usize) -> Option<(i8, i8, u64)> {
        match self.node(id)?.clone() {
            Node::Initial(word) if word == self.candidate.induction => Some((1, 0, 0)),
            Node::Length(source) if source == array => Some((0, 1, 0)),
            Node::Constant(value) => Some((0, 0, value)),
            Node::Scalar(op, args)
                if args.len() == 2
                    && matches!(
                        op,
                        Op::U64Add | Op::U64WrappingAdd | Op::U64Sub | Op::U64WrappingSub
                    ) =>
            {
                let (a, b) = (self.affine(args[0], array)?, self.affine(args[1], array)?);
                let sign = if matches!(op, Op::U64Sub | Op::U64WrappingSub) {
                    -1
                } else {
                    1
                };
                Some((
                    a.0.checked_add(b.0.checked_mul(sign)?)?,
                    a.1.checked_add(b.1.checked_mul(sign)?)?,
                    if sign == 1 {
                        a.2.wrapping_add(b.2)
                    } else {
                        a.2.wrapping_sub(b.2)
                    },
                ))
            }
            _ => None,
        }
    }

    pub fn item(&mut self, id: usize) -> Option<Item> {
        Some(match self.node(id)?.clone() {
            Node::Constant(value) => Item::Constant(value),
            Node::Initial(word) if word == self.candidate.induction => Item::Index,
            Node::Initial(word) if self.capture(word) => Item::Word(word),
            Node::Length(array) => Item::Length(array),
            Node::Scalar(op, args)
                if matches!(
                    op,
                    Op::U64Add
                        | Op::U64Sub
                        | Op::U64WrappingAdd
                        | Op::U64WrappingSub
                        | Op::U64WrappingMul
                        | Op::U64BitAnd
                        | Op::U64BitOr
                        | Op::U64BitXor
                        | Op::U64BitNot
                        | Op::U64ShlWrap
                        | Op::U64ShrWrap
                        | Op::U32ToU64
                        | Op::U64ToU32
                ) =>
            {
                Item::Scalar(
                    op,
                    args.into_iter()
                        .map(|id| self.item(id))
                        .collect::<Option<Vec<_>>>()?,
                )
            }
            _ => return None,
        })
    }
}

impl Cost {
    pub fn reads(&self, out: &mut Vec<Window>) {
        match self {
            Self::Read(window) => out.push(*window),
            Self::Add(a, b) | Self::And(a, b) | Self::Or(a, b) | Self::Xor(a, b) => {
                a.reads(out);
                b.reads(out);
            }
            _ => {}
        }
    }
}
