//! Compact coordinates from source-proved identity complements. Evaluation and
//! publication share this exact mapping; complete or selected-only raw scratch
//! is governed separately by the phase emitter's initialization certificate.
use crate::native_artifact::program::prepare::numeric_phase::masks::{
    Axis, Comparison, Mask,
    domains::{Constraint, Domain},
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;
use std::collections::BTreeMap;

struct AxisSelection {
    first: Value,
    first_count: Value,
    second: Value,
    selected: Value,
    singleton: bool,
    full: bool,
    split: bool,
}
enum Coordinates {
    Single {
        axis: Axis,
        range: AxisSelection,
    },
    Rectangle {
        row: AxisSelection,
        column: AxisSelection,
    },
}
pub(in crate::native_artifact::program::codegen) struct Selection {
    coordinates: Coordinates,
    rectangular: bool,
    rows: u64,
    columns: u64,
    count: Value,
    edge: Option<(u32, u32)>,
}
fn minimum(b: &mut FunctionBuilder<'_>, left: Value, right: Value) -> Value {
    let less = b.ins().icmp(IntCC::UnsignedLessThan, left, right);
    b.ins().select(less, left, right)
}
impl AxisSelection {
    fn new(b: &mut FunctionBuilder<'_>, comparison: Comparison, bound: Value, extent: u64) -> Self {
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
        let (first, first_count, second, second_count) = match comparison {
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
        Self {
            first,
            first_count,
            second,
            selected,
            singleton: comparison == Comparison::Eq,
            full: false,
            split: comparison == Comparison::Ne,
        }
    }
    fn from_constraint(
        b: &mut FunctionBuilder<'_>,
        constraint: Option<Constraint>,
        extent: u64,
        captures: &BTreeMap<u32, Value>,
    ) -> Result<Self, NativeCompileError> {
        if let Some(constraint) = constraint {
            let bound = *captures
                .get(&constraint.bound())
                .ok_or_else(|| backend("compound map lost its current captured bound"))?;
            return Ok(Self::new(b, constraint.comparison(), bound, extent));
        }
        let zero = b.ins().iconst(types::I64, 0);
        let selected = b.ins().iconst(types::I64, extent as i64);
        Ok(Self {
            first: zero,
            first_count: selected,
            second: zero,
            selected,
            singleton: false,
            full: true,
            split: false,
        })
    }
    fn intervals(&self, b: &mut FunctionBuilder<'_>) -> Vec<(Value, Value)> {
        let mut out = vec![(self.first, self.first_count)];
        if self.split {
            let tail = b.ins().isub(self.selected, self.first_count);
            out.push((self.second, tail));
        }
        out
    }
    fn divisor(&self, b: &mut FunctionBuilder<'_>) -> Value {
        if self.singleton {
            b.ins().iconst(types::I64, 1)
        } else {
            self.selected
        }
    }
    fn coordinate(&self, b: &mut FunctionBuilder<'_>, ordinal: Value) -> Value {
        if self.full {
            return ordinal;
        }
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
        let range = AxisSelection::new(b, mask.comparison(), bound, extent);
        let count = if mask.rectangular() {
            let other = match mask.axis() {
                Axis::Row => mask.columns(),
                Axis::Column => mask.rows(),
            };
            b.ins().imul_imm(range.selected, other as i64)
        } else {
            range.selected
        };
        Ok(Self {
            coordinates: Coordinates::Single {
                axis: mask.axis(),
                range,
            },
            rectangular: mask.rectangular(),
            rows: mask.rows(),
            columns: mask.columns(),
            count,
            edge: Some((mask.selector(), mask.active())),
        })
    }
    pub(in crate::native_artifact::program::codegen) fn compound(
        b: &mut FunctionBuilder<'_>,
        domain: &Domain,
        old: Option<&Mask>,
        captures: &BTreeMap<u32, Value>,
    ) -> Result<Self, NativeCompileError> {
        let (coordinates, count) = if domain.rectangular() {
            let row = AxisSelection::from_constraint(
                b,
                domain.constraint(Axis::Row),
                domain.rows(),
                captures,
            )?;
            let column = AxisSelection::from_constraint(
                b,
                domain.constraint(Axis::Column),
                domain.columns(),
                captures,
            )?;
            let count = b.ins().imul(row.selected, column.selected);
            (Coordinates::Rectangle { row, column }, count)
        } else {
            let (axis, extent) = if domain.columns() == 1 {
                (Axis::Row, domain.rows())
            } else {
                (Axis::Column, domain.columns())
            };
            let range =
                AxisSelection::from_constraint(b, domain.constraint(axis), extent, captures)?;
            let count = range.selected;
            (Coordinates::Single { axis, range }, count)
        };
        // Only an already proved old edge implied by the strict refinement may
        // be forced. A union such as Lt|Eq retains its original source branches.
        let edge = old
            .filter(|mask| domain.strictly_refines(Some(mask)))
            .map(|mask| (mask.selector(), mask.active()));
        Ok(Self {
            coordinates,
            rectangular: domain.rectangular(),
            rows: domain.rows(),
            columns: domain.columns(),
            count,
            edge,
        })
    }
    /// Exact disjoint zero-based source boxes for independent cell evaluation.
    /// Axis0 is the source element (column in a rectangle), axis1 the inner row.
    /// This does not change selected publication or its initialization certificate.
    pub(in crate::native_artifact::program::codegen) fn windows(
        &self,
        b: &mut FunctionBuilder<'_>,
    ) -> Vec<([Value; 2], [Value; 2])> {
        let zero = b.ins().iconst(types::I64, 0);
        let one = b.ins().iconst(types::I64, 1);
        if !self.rectangular {
            let Coordinates::Single { range, .. } = &self.coordinates else {
                unreachable!()
            };
            return range
                .intervals(b)
                .into_iter()
                .map(|(first, count)| ([first, zero], [count, one]))
                .collect();
        }
        let rows = b.ins().iconst(types::I64, self.rows as i64);
        let columns = b.ins().iconst(types::I64, self.columns as i64);
        let (row, column) = match &self.coordinates {
            Coordinates::Single {
                axis: Axis::Row,
                range,
            } => (range.intervals(b), vec![(zero, columns)]),
            Coordinates::Single {
                axis: Axis::Column,
                range,
            } => (vec![(zero, rows)], range.intervals(b)),
            Coordinates::Rectangle { row, column } => (row.intervals(b), column.intervals(b)),
        };
        row.into_iter()
            .flat_map(|(r, nr)| column.iter().map(move |&(c, nc)| ([c, r], [nc, nr])))
            .collect()
    }
    pub(in crate::native_artifact::program::codegen) fn count(&self) -> Value {
        self.count
    }
    fn coordinates(&self, b: &mut FunctionBuilder<'_>, ordinal: Value) -> (Value, Value) {
        let (divisor, row, column) = match &self.coordinates {
            Coordinates::Single {
                axis: Axis::Row,
                range,
            } => (range.divisor(b), Some(range), None),
            Coordinates::Single {
                axis: Axis::Column,
                range,
            } => (
                b.ins().iconst(types::I64, self.rows as i64),
                None,
                Some(range),
            ),
            Coordinates::Rectangle { row, column } => (row.divisor(b), Some(row), Some(column)),
        };
        // A reached nonempty loop implies both selected axes are positive.
        // The exact original source order remains column-outer, row-inner.
        let group = b.ins().udiv(ordinal, divisor);
        let consumed = b.ins().imul(group, divisor);
        let within = b.ins().isub(ordinal, consumed);
        (
            row.map_or(within, |row| row.coordinate(b, within)),
            column.map_or(group, |column| column.coordinate(b, group)),
        )
    }
    pub(in crate::native_artifact::program::codegen) fn cursors(
        &self,
        b: &mut FunctionBuilder<'_>,
        ordinal: Value,
    ) -> (Value, Option<Value>) {
        if !self.rectangular {
            let Coordinates::Single { range, .. } = &self.coordinates else {
                unreachable!()
            };
            return (range.coordinate(b, ordinal), None);
        }
        let (row, column) = self.coordinates(b, ordinal);
        (column, Some(row))
    }
    pub(in crate::native_artifact::program::codegen) fn selected_edge(
        &self,
        site: u32,
    ) -> Option<u32> {
        self.edge
            .and_then(|(selector, active)| (site == selector).then_some(active))
    }
    pub(in crate::native_artifact::program::codegen) fn output_index(
        &self,
        b: &mut FunctionBuilder<'_>,
        ordinal: Value,
    ) -> Value {
        if !self.rectangular {
            let Coordinates::Single { range, .. } = &self.coordinates else {
                unreachable!()
            };
            return range.coordinate(b, ordinal);
        }
        let (row, column) = self.coordinates(b, ordinal);
        let row = b.ins().imul_imm(row, self.columns as i64);
        b.ins().iadd(row, column)
    }
}
