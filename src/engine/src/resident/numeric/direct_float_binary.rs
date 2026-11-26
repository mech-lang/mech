//! Typed F64 elementwise arithmetic after the maintained complete preflight.
//! Current exact Core backing and broadcasting geometry select the kernel.
//! There are no cached operands, pointers, results or admission capabilities.
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

fn strides(mode: u64, columns: usize) -> Option<(usize, usize)> {
    Some(match mode {
        BINARY_BROADCAST_SCALAR => (0, 0),
        BINARY_BROADCAST_EXACT => (columns, 1),
        BINARY_BROADCAST_COLUMN => (1, 0),
        BINARY_BROADCAST_ROW => (0, 1),
        _ => return None,
    })
}

pub(super) fn try_construct(
    kernel: &BoundResidentKernel,
    shape: &ShapeInstance,
    rows: usize,
    columns: usize,
    arithmetic: SemanticArithmetic,
    left: &BorrowedNumericOperand<'_>,
    left_mode: u64,
    right: &BorrowedNumericOperand<'_>,
    right_mode: u64,
) -> Option<Result<mech_core::Value, ResidentKernelError>> {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) {
        return None;
    }
    // BorrowedNumericOperand selected these only after current schema, shape,
    // backing and full source length checks. Dense F64 is column-major and
    // retains its existing route; these exact Core slices are row-major.
    let (BorrowedNumericOperand::F64(left), BorrowedNumericOperand::F64(right)) = (left, right)
    else {
        return None;
    };
    let (lr, lc) = strides(left_mode, columns)?;
    let (rr, rc) = strides(right_mode, columns)?;
    let count = rows.checked_mul(columns)?;
    // Match the operation once, keeping the actual loop monomorphic. No
    // generic numeric draft or operation dispatch occurs for an element.
    macro_rules! construct {
        ($apply:expr) => {{
            let result = packed_snapshot_output::try_construct_f64(kernel, shape, count, |index| {
                let row = index / columns;
                let column = index % columns;
                let left = left
                    .get(row * lr + column * lc)
                    .ok_or(ResidentKernelError::InvalidShape)?
                    .to_f64();
                let right = right
                    .get(row * rr + column * rc)
                    .ok_or(ResidentKernelError::InvalidShape)?
                    .to_f64();
                Ok(F64Bits::from_f64(($apply)(left, right)))
            });
            #[cfg(test)]
            if result.is_some() {
                CONSTRUCTIONS.with(|v| v.set(v.get() + 1));
            }
            result
        }};
    }
    match arithmetic {
        SemanticArithmetic::Add => construct!(|left: f64, right: f64| left + right),
        SemanticArithmetic::Subtract => construct!(|left: f64, right: f64| left - right),
        SemanticArithmetic::Multiply => construct!(|left: f64, right: f64| left * right),
        SemanticArithmetic::Divide => construct!(|left: f64, right: f64| left / right),
        _ => None,
    }
}
