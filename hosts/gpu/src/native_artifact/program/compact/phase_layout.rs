//! Optional layout profiles for diagnostic AArch64 phases.

use cranelift_codegen::Context;
use std::io;

#[derive(serde::Deserialize)]
struct Profile {
    order: Vec<u32>,
    literal_offset: Option<u32>,
    object_tail: Option<ObjectTail>,
}

#[derive(serde::Deserialize)]
struct ObjectTail {
    code_bytes: usize,
    reference_code_bytes: usize,
    alignment: u8,
}

pub(super) fn profiles() -> Option<String> {
    std::env::var("MECH_COMPACT_PHASE_BLOCK_LAYOUT").ok()
}

fn read(profiles: &str, symbol: &str) -> io::Result<Option<Profile>> {
    profiles
        .split(';')
        .filter_map(|entry| entry.split_once('@'))
        .find_map(|(name, path)| (name == symbol).then_some(path))
        .map(|path| Ok(serde_json::from_slice(&std::fs::read(path)?)?))
        .transpose()
}

pub(super) fn apply(context: &mut Context, symbol: &str) -> io::Result<()> {
    if let Some(profiles) = profiles()
        && let Some(profile) = read(&profiles, symbol)?
    {
        context.func.aarch64_block_layout = profile.order;
        context.func.aarch64_block_layout_literal_start = profile.literal_offset;
    }
    Ok(())
}

pub(super) fn pad_object_tail(
    profiles: &str,
    symbol: &str,
    code: &mut Box<[u8]>,
) -> io::Result<()> {
    const NOP: [u8; 4] = 0xd503_201f_u32.to_le_bytes();
    if let Some(profile) = read(profiles, symbol)?
        && let Some(tail) = profile.object_tail
    {
        if profile.order.is_empty() {
            return Err(io::Error::other(
                "object tail padding requires a block layout",
            ));
        }
        let padding = tail.padding(code.len()).ok_or_else(|| {
            io::Error::other(format!(
                "object tail padding profile does not match {symbol}"
            ))
        })?;
        if padding != 0 {
            // The block layout verifies the leaf's transfers and literal pool.
            // Append whole NOPs beyond that buffer, leaving code offsets, relocations
            // and phase accounting unchanged.
            let mut bytes = std::mem::take(code).into_vec();
            for _ in 0..padding / NOP.len() {
                bytes.extend_from_slice(&NOP);
            }
            *code = bytes.into_boxed_slice();
        }
    }
    Ok(())
}

impl ObjectTail {
    fn padding(&self, code_bytes: usize) -> Option<usize> {
        let alignment = usize::from(self.alignment);
        (self.code_bytes == code_bytes
            && code_bytes % 4 == 0
            && self.reference_code_bytes >= code_bytes
            && self.reference_code_bytes % 4 == 0
            && alignment >= 4
            && alignment.is_power_of_two())
        .then(|| (self.reference_code_bytes - code_bytes) % alignment)
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectTail;

    #[test]
    fn restores_reference_extent_residue_and_rejects_stale_size() {
        let tail = ObjectTail {
            code_bytes: 496,
            reference_code_bytes: 592,
            alignment: 64,
        };
        assert_eq!(tail.padding(496), Some(32));
        assert_eq!(tail.padding(512), None);
        assert_eq!(
            ObjectTail {
                alignment: 24,
                ..tail
            }
            .padding(496),
            None
        );
    }
}
