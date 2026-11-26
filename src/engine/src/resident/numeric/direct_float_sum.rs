//! Ordered reductions over current exact F64 banks after complete admission.
//! Independent outputs share loads; no reduction is reassociated or cached.
use super::*;

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CONSTRUCTIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(super) fn without<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            DISABLED.with(|v| v.set(self.0));
        }
    }
    let _reset = Reset(DISABLED.with(|v| v.replace(true)));
    run()
}
#[cfg(test)]
pub(super) fn constructions() -> usize {
    CONSTRUCTIONS.with(|v| v.replace(0))
}

/// Four independent accumulators retain exactly the same ordered additions
/// as four ordinary numeric_sum calls. Their positive-zero seeds also cover
/// empty axes. Partial blocks use the same scalar term order.
struct Sums<'a> {
    cells: &'a [F64Bits],
    rows: usize,
    columns: usize,
    column: bool,
    count: usize,
    next: usize,
    block: [f64; 4],
}
impl Iterator for Sums<'_> {
    type Item = F64Bits;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.count {
            return None;
        }
        let lane = self.next % 4;
        if lane == 0 {
            let base = self.next;
            self.block = [0.0; 4];
            if self.count - base >= 4 {
                // Keep each recurrence explicit so the host compiler can use
                // independent SIMD lanes without a horizontal reduction.
                if self.column {
                    for at in 0..self.columns {
                        self.block[0] += self.cells[base * self.columns + at].to_f64();
                        self.block[1] += self.cells[(base + 1) * self.columns + at].to_f64();
                        self.block[2] += self.cells[(base + 2) * self.columns + at].to_f64();
                        self.block[3] += self.cells[(base + 3) * self.columns + at].to_f64();
                    }
                } else {
                    for row in 0..self.rows {
                        let at = row * self.columns + base;
                        self.block[0] += self.cells[at].to_f64();
                        self.block[1] += self.cells[at + 1].to_f64();
                        self.block[2] += self.cells[at + 2].to_f64();
                        self.block[3] += self.cells[at + 3].to_f64();
                    }
                }
            } else {
                for lane in 0..self.count - base {
                    let output = base + lane;
                    if self.column {
                        for at in 0..self.columns {
                            self.block[lane] += self.cells[output * self.columns + at].to_f64();
                        }
                    } else {
                        for row in 0..self.rows {
                            self.block[lane] += self.cells[row * self.columns + output].to_f64();
                        }
                    }
                }
            }
        }
        self.next += 1;
        Some(F64Bits::from_f64(self.block[lane]))
    }
}

pub(super) fn try_construct(
    kernel: &BoundResidentKernel,
    shape: &ShapeInstance,
    rows: usize,
    columns: usize,
    column: bool,
    operand: &BorrowedNumericOperand<'_>,
) -> Option<Result<mech_core::Value, ResidentKernelError>> {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) {
        return None;
    }
    let BorrowedNumericOperand::F64(cells) = operand else {
        return None;
    };
    if rows.checked_mul(columns)? != cells.len() {
        return Some(Err(ResidentKernelError::InvalidShape));
    }
    // Raw floating addition may choose a different operand's NaN payload
    // after host instruction selection. Keep ordinary numeric_sum for every
    // input NaN rather than changing canonical bits or payload precedence.
    // The unchanged complete preflight covers this typed read pass too.
    if cells.iter().any(|value| value.to_f64().is_nan()) {
        return None;
    }
    let count = if column { rows } else { columns };
    let mut sums = Sums {
        cells,
        rows,
        columns,
        column,
        count,
        next: 0,
        block: [0.0; 4],
    };
    let next = packed_snapshot_output::try_construct_f64(kernel, shape, count, |_| {
        sums.next().ok_or(ResidentKernelError::InvalidShape)
    })?;
    #[cfg(test)]
    CONSTRUCTIONS.with(|v| v.set(v.get() + 1));
    Some(next)
}
