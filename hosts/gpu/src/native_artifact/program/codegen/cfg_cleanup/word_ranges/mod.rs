//! Bounds of immutable scalar CLIF words, without source-type assumptions.
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
}
impl<'a> Facts<'a> {
    pub fn new(function: &'a Function) -> Self {
        Self {
            function,
            cache: BTreeMap::new(),
            assumptions: None,
        }
    }

    pub fn selected(function: &'a Function, assumptions: &'a BTreeMap<Value, Range>) -> Self {
        Self {
            function,
            cache: BTreeMap::new(),
            assumptions: Some(assumptions),
        }
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

    fn range(&mut self, value: Value, depth: usize) -> Option<Range> {
        let value = self.function.dfg.resolve_aliases(value);
        let mask = word_mask(self.function, value)?;
        if let Some(&range) = self.cache.get(&value) {
            return Some(range);
        }
        let unknown = Range(0, mask);
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
                opcode: Op::Band,
                args,
            } => Some(
                self.range(args[0], depth)?
                    .bit_and(self.range(args[1], depth)?),
            ),
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
