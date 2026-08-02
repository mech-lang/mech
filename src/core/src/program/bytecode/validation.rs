use crate::MResult;

#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use super::invalid;

pub(crate) const MAX_TYPE_RECURSION: usize = 256;

pub(crate) fn checked_usize(value: u64, what: &str) -> MResult<usize> {
    usize::try_from(value).map_err(|_| {
        invalid::<()>(format!("{what} does not fit in memory address space")).unwrap_err()
    })
}

pub(crate) fn owned_utf8(bytes: &[u8], what: &str) -> MResult<String> {
    let value = core::str::from_utf8(bytes)
        .map_err(|_| invalid::<()>(format!("invalid UTF-8 in {what}")).unwrap_err())?;
    let mut owned = String::new();
    owned
        .try_reserve_exact(bytes.len())
        .map_err(|_| invalid::<()>(format!("unable to allocate {what}")).unwrap_err())?;
    owned.push_str(value);
    Ok(owned)
}

pub(crate) fn align_up(value: u64, alignment: u64) -> MResult<u64> {
    if !matches!(alignment, 1 | 2 | 4 | 8 | 16) {
        return invalid(format!("invalid alignment {alignment}"));
    }
    value
        .checked_add(alignment - 1)
        .map(|sum| sum / alignment * alignment)
        .ok_or_else(|| invalid::<()>("alignment overflow").unwrap_err())
}

pub(crate) fn write_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn write_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

pub(crate) fn write_string(output: &mut Vec<u8>, value: &str) -> MResult<()> {
    let length = u32::try_from(value.len())
        .map_err(|_| invalid::<()>("string is too large for bytecode v1").unwrap_err())?;
    write_u32(output, length);
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

pub(crate) struct ByteReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> ByteReader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub(crate) fn read_exact(&mut self, length: usize, what: &str) -> MResult<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| invalid::<()>(format!("{what} length overflow")).unwrap_err())?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| invalid::<()>(format!("truncated {what}")).unwrap_err())?;
        self.position = end;
        Ok(bytes)
    }

    pub(crate) fn read_u8(&mut self, what: &str) -> MResult<u8> {
        Ok(self.read_exact(1, what)?[0])
    }

    pub(crate) fn read_u16(&mut self, what: &str) -> MResult<u16> {
        Ok(u16::from_le_bytes(
            self.read_exact(2, what)?.try_into().unwrap(),
        ))
    }

    pub(crate) fn read_u32(&mut self, what: &str) -> MResult<u32> {
        Ok(u32::from_le_bytes(
            self.read_exact(4, what)?.try_into().unwrap(),
        ))
    }

    pub(crate) fn read_u64(&mut self, what: &str) -> MResult<u64> {
        Ok(u64::from_le_bytes(
            self.read_exact(8, what)?.try_into().unwrap(),
        ))
    }

    pub(crate) fn read_string(&mut self, what: &str) -> MResult<String> {
        let length = checked_usize(
            u64::from(self.read_u32(&format!("{what} length"))?),
            &format!("{what} length"),
        )?;
        self.read_utf8(length, what)
    }

    pub(crate) fn read_utf8(&mut self, length: usize, what: &str) -> MResult<String> {
        owned_utf8(self.read_exact(length, what)?, what)
    }
}
