//! Shared matrix payload sizing for bytecode-v1 constant decoding.

use crate::MResult;

use super::{checked_usize, invalid};

pub(super) fn element_count(rows: u32, cols: u32) -> MResult<(usize, usize, usize)> {
    let row_count = checked_usize(u64::from(rows), "matrix row count")?;
    let column_count = checked_usize(u64::from(cols), "matrix column count")?;
    let element_count = row_count
        .checked_mul(column_count)
        .ok_or_else(|| invalid::<()>("matrix element count overflow").unwrap_err())?;
    Ok((row_count, column_count, element_count))
}
