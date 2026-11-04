//! Definite assignment and unsigned ranges for one acyclic scalar traversal.
//! Facts are rebuilt for every source path; no invocation values are cached.
use mech_engine::native_artifact::{
    NativeMatchValue as Value, NativeScalarKind as Kind, NativeScalarOp as Op,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Range(pub u64, pub u64);
impl Range {
    pub(super) fn kind(kind: Kind) -> Option<Self> {
        Some(Self(
            0,
            if kind == Kind::Bool {
                1
            } else {
                kind.unsigned_maximum()?
            },
        ))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Compare {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Predicate(Compare, u32, u32);
#[derive(Clone, Debug, Default)]
pub(super) struct State {
    pub ranges: BTreeMap<u32, Range>,
    predicates: BTreeMap<u32, Predicate>,
    ge: BTreeSet<(u32, u32)>,
}
impl State {
    pub(super) fn merge(&mut self, other: &Self) {
        self.ranges.retain(|value, range| {
            if let Some(next) = other.ranges.get(value) {
                *range = Range(range.0.min(next.0), range.1.max(next.1));
                true
            } else {
                false
            }
        });
        self.predicates
            .retain(|value, p| other.predicates.get(value) == Some(p));
        self.ge.retain(|pair| other.ge.contains(pair));
    }
    fn define(&mut self, value: u32, range: Range) {
        self.predicates
            .retain(|key, p| *key != value && p.1 != value && p.2 != value);
        self.ge.retain(|&(a, b)| a != value && b != value);
        self.ranges.insert(value, range);
    }
    pub(super) fn copy(&mut self, source: Value, output: Value) -> Option<()> {
        let range = *self.ranges.get(&source.get())?;
        let predicate = self.predicates.get(&source.get()).copied();
        self.define(output.get(), range);
        if let Some(predicate) =
            predicate.filter(|p| source == output || (p.1 != output.get() && p.2 != output.get()))
        {
            self.predicates.insert(output.get(), predicate);
        }
        Some(())
    }
    pub(super) fn primitive(&mut self, op: Op, inputs: &[Value], output: Value) -> Option<()> {
        use Op::*;
        let args = inputs
            .iter()
            .map(|v| self.ranges.get(&v.get()).copied())
            .collect::<Option<Vec<_>>>()?;
        let (_, kind) = op.signature();
        let maximum = Range::kind(kind)?.1;
        let predicate = match op {
            U64Eq | U8Eq | U16Eq | U32Eq | BoolEq => Some(Compare::Eq),
            U64Ne | U8Ne | U16Ne | U32Ne | BoolNe => Some(Compare::Ne),
            U64Lt | U8Lt | U16Lt | U32Lt => Some(Compare::Lt),
            U64Le | U8Le | U16Le | U32Le => Some(Compare::Le),
            U64Gt | U8Gt | U16Gt => Some(Compare::Gt),
            U64Ge | U8Ge | U16Ge => Some(Compare::Ge),
            _ => None,
        };
        let range = if predicate.is_some() {
            Range(0, 1)
        } else {
            match op {
                U8ToU16 | U8ToU32 | U8ToU64 | U16ToU8 | U16ToU32 | U16ToU64 | U32ToU8
                | U32ToU16 | U32ToU64 | U64ToU8 | U64ToU16 | U64ToU32
                    if args[0].1 <= maximum =>
                {
                    args[0]
                }
                U64Add | U32Add | U8Add => Range(
                    args[0].0.checked_add(args[1].0)?,
                    args[0].1.checked_add(args[1].1)?,
                ),
                U64Sub | U32Sub
                    if args[0].0 >= args[1].1
                        || self.ge.contains(&(inputs[0].get(), inputs[1].get())) =>
                {
                    Range(
                        args[0].0.saturating_sub(args[1].1),
                        args[0].1.checked_sub(args[1].0)?,
                    )
                }
                U64Mul => Range(
                    args[0].0.checked_mul(args[1].0)?,
                    args[0].1.checked_mul(args[1].1)?,
                ),
                U64Div if args[1].0 != 0 => Range(args[0].0 / args[1].1, args[0].1 / args[1].0),
                U64Rem if args[1].0 != 0 => Range(0, args[0].1.min(args[1].1 - 1)),
                BoolAnd | BoolOr | BoolXor | BoolNot => Range(0, 1),
                U64BitAnd => Range(0, args[0].1.min(args[1].1)),
                U64BitOr | U64BitXor | U64BitNot | U64ShlWrap | U64ShrWrap | U64WrappingAdd
                | U64WrappingSub | U64WrappingMul => Range(0, u64::MAX),
                U64Clz | U64Ctz => Range(0, 64),
                _ => return None,
            }
        };
        if range.0 > range.1 || range.1 > maximum {
            return None;
        }
        self.define(output.get(), range);
        if let Some(compare) = predicate.filter(|_| !inputs.contains(&output)) {
            self.predicates.insert(
                output.get(),
                Predicate(compare, inputs[0].get(), inputs[1].get()),
            );
        }
        Some(())
    }
    fn equality(&mut self, left: u32, right: u32, equal: bool) -> Option<()> {
        let a = *self.ranges.get(&left)?;
        let b = *self.ranges.get(&right)?;
        if equal {
            let range = Range(a.0.max(b.0), a.1.min(b.1));
            if range.0 > range.1 {
                return None;
            }
            self.ranges.insert(left, range);
            self.ranges.insert(right, range);
            self.ge.extend([(left, right), (right, left)]);
        } else {
            if a.0 == a.1 && b.0 == b.1 && a == b {
                return None;
            }
            if b.0 == b.1 {
                let range = Range(
                    if a.0 == b.0 { a.0.checked_add(1)? } else { a.0 },
                    if a.1 == b.1 { a.1.checked_sub(1)? } else { a.1 },
                );
                if range.0 > range.1 {
                    return None;
                }
                self.ranges.insert(left, range);
            }
        }
        Some(())
    }
    fn less(&mut self, left: u32, right: u32, yes: bool) -> Option<()> {
        let a = *self.ranges.get(&left)?;
        let b = *self.ranges.get(&right)?;
        let (a, b) = if yes {
            (
                Range(a.0, a.1.min(b.1.checked_sub(1)?)),
                Range(b.0.max(a.0.checked_add(1)?), b.1),
            )
        } else {
            self.ge.insert((left, right));
            (Range(a.0.max(b.0), a.1), Range(b.0, b.1.min(a.1)))
        };
        if a.0 > a.1 || b.0 > b.1 {
            return None;
        }
        self.ranges.insert(left, a);
        self.ranges.insert(right, b);
        Some(())
    }
    pub(super) fn boolean(mut self, value: Value, yes: bool) -> Option<Self> {
        let range = *self.ranges.get(&value.get())?;
        let selected = u64::from(yes);
        if selected < range.0 || selected > range.1 {
            return None;
        }
        self.ranges.insert(value.get(), Range(selected, selected));
        if let Some(Predicate(compare, a, b)) = self.predicates.get(&value.get()).copied() {
            match compare {
                Compare::Eq => self.equality(a, b, yes)?,
                Compare::Ne => self.equality(a, b, !yes)?,
                Compare::Lt => self.less(a, b, yes)?,
                Compare::Le => self.less(b, a, !yes)?,
                Compare::Gt => self.less(b, a, yes)?,
                Compare::Ge => self.less(a, b, !yes)?,
            }
        }
        Some(self)
    }
    pub(super) fn literal(mut self, left: Value, right: Value, equal: bool) -> Option<Self> {
        let right_range = *self.ranges.get(&right.get())?;
        self.equality(left.get(), right.get(), equal)?;
        if right_range.0 == right_range.1 && right_range.1 <= 1 {
            let range = *self.ranges.get(&left.get())?;
            if range.0 == range.1 && range.1 <= 1 {
                return self.boolean(left, range.1 != 0);
            }
        }
        Some(self)
    }
}
