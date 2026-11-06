//! Four source candidates, with native unsigned low halves and no cost array.
use super::*;
use crate::native_artifact::codegen::common::ssa_words::Words;
use cranelift_codegen::ir::{ConstantData, Endianness, InstBuilder, MemFlags, Value, types};
use cranelift_frontend::FunctionBuilder;
use std::collections::BTreeMap;

pub(super) fn constant(b: &mut FunctionBuilder<'_>, values: [u32; 4]) -> Value {
    let bytes = values
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect::<Vec<_>>();
    let id = b.func.dfg.constants.insert(ConstantData::from(bytes));
    b.ins().vconst(types::I32X4, id)
}

pub(super) struct Inputs {
    words: BTreeMap<usize, Value>,
    bases: BTreeMap<Window, Value>,
}
impl Inputs {
    pub fn new(
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        proof: &Proof,
        shadow: usize,
        cost: &Cost,
    ) -> Self {
        let mut out = Self {
            words: BTreeMap::new(),
            bases: BTreeMap::new(),
        };
        out.capture(b, words, proof, shadow, cost);
        out
    }
    fn capture(
        &mut self,
        b: &mut FunctionBuilder<'_>,
        words: &Words,
        proof: &Proof,
        shadow: usize,
        cost: &Cost,
    ) {
        match cost {
            Cost::Word(slot) => {
                self.words.entry(*slot).or_insert_with(|| {
                    let value = words.read(b, *slot);
                    let value = b.ins().ireduce(types::I32, value);
                    value
                });
            }
            Cost::Read(w) => {
                self.bases.entry(*w).or_insert_with(|| {
                    let view = shadow + proof.view(w.array);
                    let base = words.read(b, view);
                    let ordinal = if w.reverse {
                        let length = words.read(b, view + 1);
                        b.ins().iadd_imm(length, (w.delta as i64).wrapping_neg())
                    } else {
                        b.ins().iconst(types::I64, w.delta as i64)
                    };
                    let shift = if proof.arrays[w.array].width == 8 {
                        3
                    } else {
                        2
                    };
                    let offset = b.ins().ishl_imm(ordinal, shift);
                    // The current entry already admits every reached read.
                    // This is integer address geometry, with no load at the
                    // adjusted base and no retained authority across entries.
                    b.ins().iadd(base, offset)
                });
            }
            Cost::Add(a, c) | Cost::And(a, c) | Cost::Or(a, c) | Cost::Xor(a, c) => {
                self.capture(b, words, proof, shadow, a);
                self.capture(b, words, proof, shadow, c);
            }
            _ => {}
        }
    }
    pub fn emit(
        &self,
        b: &mut FunctionBuilder<'_>,
        proof: &Proof,
        cost: &Cost,
        k: Value,
        indices: Value,
    ) -> Value {
        match cost {
            Cost::Constant(value) => constant(b, [*value; 4]),
            Cost::Word(slot) => b.ins().splat(types::I32X4, self.words[slot]),
            Cost::Index => indices,
            Cost::Read(w) => self.read(b, proof, *w, k),
            Cost::Add(a, c) | Cost::And(a, c) | Cost::Or(a, c) | Cost::Xor(a, c) => {
                let a = self.emit(b, proof, a, k, indices);
                let c = self.emit(b, proof, c, k, indices);
                match cost {
                    Cost::Add(..) => b.ins().iadd(a, c),
                    Cost::And(..) => b.ins().band(a, c),
                    Cost::Or(..) => b.ins().bor(a, c),
                    _ => b.ins().bxor(a, c),
                }
            }
        }
    }
    fn pointer(&self, b: &mut FunctionBuilder<'_>, proof: &Proof, w: Window, k: Value) -> Value {
        let shift = if proof.arrays[w.array].width == 8 {
            3
        } else {
            2
        };
        let offset = b.ins().ishl_imm(k, shift);
        let base = self.bases[&w];
        if w.reverse {
            b.ins().isub(base, offset)
        } else {
            b.ins().iadd(base, offset)
        }
    }
    fn read(&self, b: &mut FunctionBuilder<'_>, proof: &Proof, w: Window, k: Value) -> Value {
        let pointer = self.pointer(b, proof, w, k);
        let width = proof.arrays[w.array].width;
        let pointer = if w.reverse {
            b.ins().iadd_imm(pointer, -3 * width as i64)
        } else {
            pointer
        };
        // Every lane lies in the original initialized window. Alignment is not
        // inferred from allocation capacity or the vector load's width.
        let flags = MemFlags::new().with_notrap();
        let first = b.ins().load(types::I8X16, flags, pointer, 0);
        let bytes = if width == 8 {
            let second = b.ins().load(types::I8X16, flags, pointer, 16);
            let mask = if w.reverse {
                [24u8, 25, 26, 27, 16, 17, 18, 19, 8, 9, 10, 11, 0, 1, 2, 3]
            } else {
                [0u8, 1, 2, 3, 8, 9, 10, 11, 16, 17, 18, 19, 24, 25, 26, 27]
            };
            let mask = b
                .func
                .dfg
                .immediates
                .push(ConstantData::from(mask.to_vec()));
            b.ins().shuffle(first, second, mask)
        } else if w.reverse {
            let mask = b.func.dfg.immediates.push(ConstantData::from(vec![
                12u8, 13, 14, 15, 8, 9, 10, 11, 4, 5, 6, 7, 0, 1, 2, 3,
            ]));
            b.ins().shuffle(first, first, mask)
        } else {
            first
        };
        b.ins().bitcast(
            types::I32X4,
            MemFlags::new().with_endianness(Endianness::Little),
            bytes,
        )
    }

    pub fn scalar(
        &self,
        b: &mut FunctionBuilder<'_>,
        proof: &Proof,
        cost: &Cost,
        k: Value,
    ) -> Value {
        match cost {
            Cost::Constant(value) => b.ins().iconst(types::I32, i64::from(*value)),
            Cost::Word(slot) => self.words[slot],
            Cost::Index => b.ins().ireduce(types::I32, k),
            Cost::Read(w) => {
                let pointer = self.pointer(b, proof, *w, k);
                b.ins()
                    .load(types::I32, MemFlags::new().with_notrap(), pointer, 0)
            }
            Cost::Add(a, c) | Cost::And(a, c) | Cost::Or(a, c) | Cost::Xor(a, c) => {
                let a = self.scalar(b, proof, a, k);
                let c = self.scalar(b, proof, c, k);
                match cost {
                    Cost::Add(..) => b.ins().iadd(a, c),
                    Cost::And(..) => b.ins().band(a, c),
                    Cost::Or(..) => b.ins().bor(a, c),
                    _ => b.ins().bxor(a, c),
                }
            }
        }
    }
}
