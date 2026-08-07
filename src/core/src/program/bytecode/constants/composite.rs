//! Shared length-prefixed framing for recursive composite constants.

use crate::MResult;

use super::{ByteReader, checked_usize};

pub(super) fn read_child_payload<'a>(reader: &mut ByteReader<'a>, what: &str) -> MResult<&'a [u8]> {
    let length = checked_usize(
        u64::from(reader.read_u32(&format!("{what} length"))?),
        &format!("{what} length"),
    )?;
    reader.read_exact(length, what)
}
