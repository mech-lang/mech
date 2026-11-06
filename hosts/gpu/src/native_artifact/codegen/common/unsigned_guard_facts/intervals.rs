//! Bounded unsigned value queries at one CFG location. Facts describe the
//! actual SSA values and must dominate the use; no memory contents are assumed.
use super::*;
use cranelift_codegen::dominator_tree::DominatorTree;

type Facts = BTreeMap<Value, Vec<(Block, u64)>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Range {
    pub low: u64,
    pub high: u64,
}
impl Range {
    fn full() -> Self {
        Self {
            low: 0,
            high: u64::MAX,
        }
    }
    fn exact(value: u64) -> Self {
        Self {
            low: value,
            high: value,
        }
    }
}

pub(super) struct Query<'a> {
    pub function: &'a Function,
    pub domtree: &'a DominatorTree,
    pub known: &'a BTreeMap<Value, u64>,
    pub upper: &'a Facts,
    pub lower: &'a Facts,
    pub at: Block,
}
impl Query<'_> {
    pub fn operand(&self, operand: Operand) -> Range {
        match operand {
            Operand::Constant(value) => Range::exact(value),
            Operand::Value(value) => {
                let mut steps = 128;
                self.value(value, 16, &mut steps)
            }
        }
    }

    fn value(&self, value: Value, depth: usize, steps: &mut usize) -> Range {
        let f = self.function;
        let value = f.dfg.resolve_aliases(value);
        let ty = f.dfg.value_type(value);
        if !ty.is_int() || ty.bits() > 64 {
            return Range::full();
        }
        let bits = ty.bits();
        let mask = if bits == 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let mut range = Range {
            low: 0,
            high: intrinsic_upper_bound(f, value, 1),
        };
        if let Some(D::UnaryImm {
            opcode: O::Iconst,
            imm,
        }) = definition(f, value)
        {
            range = Range::exact(imm.bits() as u64 & mask);
        } else if depth != 0 && *steps != 0 {
            *steps -= 1;
            if let Some(data) = definition(f, value) {
                let args = data.arguments(&f.dfg.value_lists);
                let mut get = |i: usize| self.value(args[i], depth - 1, steps);
                let inferred = match *data {
                    D::BinaryImm64 { opcode, arg, imm } => {
                        let a = self.value(arg, depth - 1, steps);
                        Self::binary(opcode, a, Range::exact(imm.bits() as u64 & mask), bits)
                    }
                    D::Binary { opcode, .. } => {
                        let a = get(0);
                        let b = get(1);
                        Self::binary(opcode, a, b, bits)
                    }
                    D::Unary {
                        opcode: O::Uextend, ..
                    } => Some(get(0)),
                    D::Unary {
                        opcode: O::Ireduce, ..
                    } => {
                        let source = get(0);
                        (source.high <= mask).then_some(source)
                    }
                    D::Ternary {
                        opcode: O::Select, ..
                    } => {
                        let a = get(1);
                        let b = get(2);
                        Some(Range {
                            low: a.low.min(b.low),
                            high: a.high.max(b.high),
                        })
                    }
                    _ => None,
                };
                if let Some(inferred) = inferred {
                    range.low = range.low.max(inferred.low);
                    range.high = range.high.min(inferred.high);
                }
            }
        }
        if let Some(&high) = self.known.get(&value) {
            range.high = range.high.min(high);
        }
        for &(origin, high) in self.upper.get(&value).into_iter().flatten() {
            if self.domtree.block_dominates(origin, self.at) {
                range.high = range.high.min(high);
            }
        }
        for &(origin, low) in self.lower.get(&value).into_iter().flatten() {
            if self.domtree.block_dominates(origin, self.at) {
                range.low = range.low.max(low);
            }
        }
        // Contradictory facts can arise on an unreachable success edge.
        // Decline the query instead of using an empty interval as a proof.
        if range.low > range.high {
            Range::full()
        } else {
            range
        }
    }

    fn binary(opcode: O, a: Range, b: Range, bits: u32) -> Option<Range> {
        let mask = if bits == 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let cover = |v: u64| {
            if v == 0 {
                0
            } else {
                u64::MAX >> v.leading_zeros()
            }
        };
        let (low, high) = match opcode {
            O::Iadd | O::IaddImm => (a.low.checked_add(b.low)?, a.high.checked_add(b.high)?),
            O::Imul | O::ImulImm => (a.low.checked_mul(b.low)?, a.high.checked_mul(b.high)?),
            O::Umulhi => (
                ((u128::from(a.low) * u128::from(b.low)) >> bits) as u64,
                ((u128::from(a.high) * u128::from(b.high)) >> bits) as u64,
            ),
            O::Isub if a.low >= b.high => (a.low - b.high, a.high - b.low),
            O::Band | O::BandImm => (0, a.high.min(b.high)),
            O::Bor | O::BorImm | O::Bxor | O::BxorImm => (0, cover(a.high) | cover(b.high)),
            O::Udiv | O::UdivImm if b.low == b.high && b.low != 0 => {
                (a.low / b.low, a.high / b.low)
            }
            O::Urem | O::UremImm if b.low == b.high && b.low != 0 => (0, a.high.min(b.low - 1)),
            O::Ushr | O::UshrImm if b.low == b.high && b.low < u64::from(bits) => {
                (a.low >> b.low, a.high >> b.low)
            }
            O::Ishl | O::IshlImm if b.low == b.high && b.low < u64::from(bits) => {
                (0, (cover(a.high) << b.low) & mask)
            }
            _ => return None,
        };
        (high <= mask).then_some(Range { low, high })
    }
}

pub(super) fn decide(cond: IntCC, a: Range, b: Range) -> Option<bool> {
    let (yes, no) = match cond {
        IntCC::UnsignedLessThan => (a.high < b.low, a.low >= b.high),
        IntCC::UnsignedLessThanOrEqual => (a.high <= b.low, a.low > b.high),
        IntCC::UnsignedGreaterThan => (a.low > b.high, a.high <= b.low),
        IntCC::UnsignedGreaterThanOrEqual => (a.low >= b.high, a.high < b.low),
        IntCC::Equal => (a.low == a.high && a == b, a.high < b.low || b.high < a.low),
        IntCC::NotEqual => return decide(IntCC::Equal, a, b).map(|v| !v),
        _ => return None,
    };
    if yes {
        Some(true)
    } else if no {
        Some(false)
    } else {
        None
    }
}
