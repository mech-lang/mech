//! Bounds of immutable scalar CLIF words, without source-type assumptions.
pub(super) mod condition;
pub(super) use crate::native_artifact::program::prepare::word_collections::bounds::Range;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Function, InstructionData as Data, Opcode as Op, Value, ValueDef};
use std::collections::BTreeMap;

const MAX_DEPTH: usize = 64;

fn word_mask(function: &Function, value: Value) -> Option<u64> {
    let ty = function.dfg.value_type(value);
    (ty.is_int() && !ty.is_vector() && ty.bits() > 0 && ty.bits() <= 64)
        .then(|| u64::MAX >> (64 - ty.bits()))
}

pub(super) fn literal(function: &Function, value: Value) -> Option<u64> {
    let value = function.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    let Data::UnaryImm {
        opcode: Op::Iconst,
        imm,
    } = function.dfg.insts[inst]
    else {
        return None;
    };
    Some(imm.bits() as u64 & word_mask(function, value)?)
}

/// Normalize integer addition modulo the word width. Callers use I64 words.
pub(super) fn constant_offset(f: &Function, mut value: Value) -> (Value, u64) {
    let mut offset = 0u64;
    loop {
        value = f.dfg.resolve_aliases(value);
        let step = match f.dfg.value_def(value) {
            ValueDef::Result(inst, 0) => match f.dfg.insts[inst] {
                Data::BinaryImm64 {
                    opcode: Op::IaddImm,
                    arg,
                    imm,
                } => Some((arg, imm.bits() as u64)),
                Data::Binary {
                    opcode: Op::Iadd,
                    args: [left, right],
                } => literal(f, right)
                    .map(|constant| (left, constant))
                    .or_else(|| literal(f, left).map(|constant| (right, constant))),
                _ => None,
            },
            _ => None,
        };
        let Some((next, constant)) = step else {
            return (value, offset);
        };
        offset = offset.wrapping_add(constant);
        value = next;
    }
}

#[cfg(test)]
mod tests;

fn compare(cc: IntCC, a: Range, b: Range) -> Option<bool> {
    use IntCC::*;
    match cc {
        Equal | NotEqual => {
            let equal = if a.1 < b.0 || b.1 < a.0 {
                Some(false)
            } else if a.0 == a.1 && b.0 == b.1 {
                Some(a.0 == b.0)
            } else {
                None
            };
            equal.map(|v| if cc == Equal { v } else { !v })
        }
        UnsignedLessThan => {
            if a.1 < b.0 {
                Some(true)
            } else if a.0 >= b.1 {
                Some(false)
            } else {
                None
            }
        }
        UnsignedLessThanOrEqual => {
            if a.1 <= b.0 {
                Some(true)
            } else if a.0 > b.1 {
                Some(false)
            } else {
                None
            }
        }
        UnsignedGreaterThan => compare(UnsignedLessThan, b, a),
        UnsignedGreaterThanOrEqual => compare(UnsignedLessThanOrEqual, b, a),
        // Signed/float interpretations never inherit unsigned source facts.
        _ => None,
    }
}

pub(super) struct Facts<'a> {
    function: &'a Function,
    cache: BTreeMap<Value, Range>,
    assumptions: Option<&'a BTreeMap<Value, Range>>,
    bound_values: bool,
    bitwise_constants: bool,
}
impl<'a> Facts<'a> {
    pub fn new(function: &'a Function) -> Self {
        Self {
            function,
            cache: BTreeMap::new(),
            assumptions: None,
            bound_values: false,
            bitwise_constants: false,
        }
    }

    pub(super) fn with_bitwise_constants(function: &'a Function) -> Self {
        Self {
            bitwise_constants: true,
            ..Self::new(function)
        }
    }

    pub fn selected(function: &'a Function, assumptions: &'a BTreeMap<Value, Range>) -> Self {
        Self {
            function,
            cache: BTreeMap::new(),
            assumptions: Some(assumptions),
            bound_values: false,
            bitwise_constants: false,
        }
    }

    // A copied current SSA word is an atom in a later loop visit. Its earlier
    // definition must not be reinterpreted using that visit's new phi values.
    pub fn bound(function: &'a Function, values: &'a BTreeMap<Value, Range>) -> Self {
        let mut facts = Self::selected(function, values);
        facts.bound_values = true;
        facts
    }

    pub fn condition(&mut self, value: Value) -> Option<bool> {
        let range = self.range(value, MAX_DEPTH)?;
        if range.0 > 0 {
            Some(true)
        } else if range.1 == 0 {
            Some(false)
        } else {
            None
        }
    }

    pub fn canonical_boolean(&mut self, value: Value) -> bool {
        self.range(value, MAX_DEPTH)
            .is_some_and(|range| range.1 <= 1)
    }

    pub fn value_range(&mut self, value: Value) -> Option<Range> {
        self.range(value, MAX_DEPTH)
    }

    fn range(&mut self, value: Value, depth: usize) -> Option<Range> {
        let value = self.function.dfg.resolve_aliases(value);
        let mask = word_mask(self.function, value)?;
        if let Some(&range) = self.cache.get(&value) {
            return Some(range);
        }
        let unknown = Range(0, mask);
        if self.bound_values {
            if let Some(&r) = self.assumptions.and_then(|v| v.get(&value)) {
                return Some(if r.0 <= r.1 && r.1 <= mask {
                    r
                } else {
                    unknown
                });
            }
        }
        // Analysis exhaustion loses a fact; it never rejects a program.
        if depth == 0 {
            return Some(unknown);
        }
        let found = self.definition(value, mask, depth - 1).unwrap_or(unknown);
        let found = if found.0 <= found.1 && found.1 <= mask {
            found
        } else {
            unknown
        };
        let found = self
            .assumptions
            .and_then(|known| known.get(&value))
            .map_or(found, |known| {
                let intersection = Range(found.0.max(known.0), found.1.min(known.1));
                if intersection.0 <= intersection.1 {
                    intersection
                } else {
                    found
                }
            });
        self.cache.insert(value, found);
        Some(found)
    }

    fn definition(&mut self, value: Value, mask: u64, depth: usize) -> Option<Range> {
        let ValueDef::Result(inst, 0) = self.function.dfg.value_def(value) else {
            return None;
        };
        match self.function.dfg.insts[inst] {
            Data::UnaryImm {
                opcode: Op::Iconst,
                imm,
                ..
            } => {
                let n = imm.bits() as u64 & mask;
                Some(Range(n, n))
            }
            Data::Binary {
                opcode: opcode @ (Op::Band | Op::Bor),
                args,
            } if opcode == Op::Band || self.bitwise_constants => {
                let left = self.range(args[0], depth)?;
                let right = self.range(args[1], depth)?;
                if self.bitwise_constants && left.0 == left.1 && right.0 == right.1 {
                    let value = if opcode == Op::Band {
                        left.0 & right.0
                    } else {
                        left.0 | right.0
                    };
                    return Some(Range(value, value));
                }
                (opcode == Op::Band).then(|| left.bit_and(right))
            }
            Data::BinaryImm64 {
                opcode: Op::BandImm,
                arg,
                imm,
            } => {
                let n = imm.bits() as u64 & mask;
                Some(self.range(arg, depth)?.bit_and(Range(n, n)))
            }
            Data::Unary {
                opcode: Op::Uextend,
                arg,
            } => self.range(arg, depth),
            Data::Unary {
                opcode: Op::Ireduce,
                arg,
            } => {
                let source = self.range(arg, depth)?;
                (source.1 <= mask).then_some(source)
            }
            Data::IntCompare { cond, args, .. } => {
                let a = self.range(args[0], depth)?;
                let b = self.range(args[1], depth)?;
                Some(compare(cond, a, b).map_or(Range(0, 1), |v| Range(u64::from(v), u64::from(v))))
            }
            Data::IntCompareImm { cond, arg, imm, .. } => {
                let n = imm.bits() as u64 & word_mask(self.function, arg)?;
                let a = self.range(arg, depth)?;
                Some(
                    compare(cond, a, Range(n, n))
                        .map_or(Range(0, 1), |v| Range(u64::from(v), u64::from(v))),
                )
            }
            _ => None,
        }
    }
}
