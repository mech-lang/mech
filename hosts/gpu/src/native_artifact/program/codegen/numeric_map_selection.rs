//! Compact selected coordinates of a source-proved identity-complement map.
//! All output cells have already been initialized in disjoint phase scratch.
use crate::native_artifact::program::prepare::numeric_phase::masks::{Axis, Comparison, Mask};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use std::collections::BTreeMap;

pub(in crate::native_artifact::program::codegen) struct Selection {
    axis: Axis,
    rectangular: bool,
    rows: u64,
    columns: u64,
    first: Value,
    first_count: Value,
    second: Value,
    selected: Value,
    count: Value,
    singleton: bool,
    selector: u32,
    active: u32,
}
fn minimum(b: &mut FunctionBuilder<'_>, left: Value, right: Value) -> Value {
    let less = b.ins().icmp(IntCC::UnsignedLessThan, left, right);
    b.ins().select(less, left, right)
}
impl Selection {
    pub(in crate::native_artifact::program::codegen) fn new(
        b: &mut FunctionBuilder<'_>,
        mask: &Mask,
        captures: &BTreeMap<u32, Value>,
    ) -> Result<Self, NativeCompileError> {
        let bound = *captures
            .get(&mask.bound())
            .ok_or_else(|| backend("masked map lost its current captured bound"))?;
        let extent = match mask.axis() {
            Axis::Row => mask.rows(),
            Axis::Column => mask.columns(),
        };
        let limit = b.ins().iconst(types::I64, extent as i64);
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        let positive = b.ins().icmp_imm(IntCC::NotEqual, bound, 0);
        let prior = b.ins().iadd_imm(bound, -1);
        let prior = b.ins().select(positive, prior, zero);
        let clipped = minimum(b, bound, limit);
        let before = minimum(b, prior, limit);
        let inside = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, bound, limit);
        let inside = b.ins().band(positive, inside);
        let (first, first_count, second, second_count) = match mask.comparison() {
            Comparison::Eq => (
                b.ins().select(inside, prior, zero),
                b.ins().select(inside, one, zero),
                zero,
                zero,
            ),
            Comparison::Ne => {
                let tail = b.ins().isub(limit, clipped);
                (
                    zero,
                    b.ins().select(inside, prior, limit),
                    clipped,
                    b.ins().select(inside, tail, zero),
                )
            }
            Comparison::Lt => (zero, before, zero, zero),
            Comparison::Le => (zero, clipped, zero, zero),
            Comparison::Gt => {
                let remaining = b.ins().isub(limit, clipped);
                (clipped, remaining, zero, zero)
            }
            Comparison::Ge => {
                let remaining = b.ins().isub(limit, before);
                (before, remaining, zero, zero)
            }
        };
        let selected = b.ins().iadd(first_count, second_count);
        let count = if mask.rectangular() {
            let other = match mask.axis() {
                Axis::Row => mask.columns(),
                Axis::Column => mask.rows(),
            };
            b.ins().imul_imm(selected, other as i64)
        } else {
            selected
        };
        Ok(Self {
            axis: mask.axis(),
            rectangular: mask.rectangular(),
            rows: mask.rows(),
            columns: mask.columns(),
            first,
            first_count,
            second,
            selected,
            count,
            singleton: mask.comparison() == Comparison::Eq,
            selector: mask.selector(),
            active: mask.active(),
        })
    }
    pub(in crate::native_artifact::program::codegen) fn count(&self) -> Value {
        self.count
    }
    fn coordinate(&self, b: &mut FunctionBuilder<'_>, ordinal: Value) -> Value {
        if self.singleton {
            return b.ins().iadd(self.first, ordinal);
        }
        let in_first = b
            .ins()
            .icmp(IntCC::UnsignedLessThan, ordinal, self.first_count);
        let first = b.ins().iadd(self.first, ordinal);
        let remainder = b.ins().isub(ordinal, self.first_count);
        let second = b.ins().iadd(self.second, remainder);
        b.ins().select(in_first, first, second)
    }
    fn coordinates(&self, b: &mut FunctionBuilder<'_>, ordinal: Value) -> (Value, Value) {
        let divisor = match self.axis {
            Axis::Row if self.singleton => b.ins().iconst(types::I64, 1),
            Axis::Row => self.selected,
            Axis::Column => b.ins().iconst(types::I64, self.rows as i64),
        };
        // A nonempty selected loop proves a positive divisor. Eq has exactly
        // one selected coordinate on this reached path, otherwise the loop is empty.
        let group = b.ins().udiv(ordinal, divisor);
        let consumed = b.ins().imul(group, divisor);
        let within = b.ins().isub(ordinal, consumed);
        match self.axis {
            Axis::Row => (self.coordinate(b, within), group),
            Axis::Column => (within, self.coordinate(b, group)),
        }
    }
    pub(in crate::native_artifact::program::codegen) fn cursors(
        &self,
        b: &mut FunctionBuilder<'_>,
        ordinal: Value,
    ) -> (Value, Option<Value>) {
        if !self.rectangular {
            return (self.coordinate(b, ordinal), None);
        }
        let (row, column) = self.coordinates(b, ordinal);
        (column, Some(row))
    }
    pub(in crate::native_artifact::program::codegen) fn selected_edge(
        &self,
        site: u32,
    ) -> Option<u32> {
        (site == self.selector).then_some(self.active)
    }
    pub(in crate::native_artifact::program::codegen) fn output_index(
        &self,
        b: &mut FunctionBuilder<'_>,
        ordinal: Value,
    ) -> Value {
        if !self.rectangular {
            return self.coordinate(b, ordinal);
        }
        let (row, column) = self.coordinates(b, ordinal);
        let row = b.ins().imul_imm(row, self.columns as i64);
        b.ins().iadd(row, column)
    }
}
