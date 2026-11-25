//! Optional layout profiles for diagnostic AArch64 phases.

use cranelift_codegen::Context;
use std::{io, path::Path};

#[derive(serde::Deserialize)]
struct Profile {
    order: Vec<u32>,
    literal_offset: Option<u32>,
    #[serde(default)]
    thread_jumps: bool,
    object_tail: Option<ObjectTail>,
}

#[derive(serde::Deserialize)]
struct ObjectTail {
    code_bytes: usize,
    reference_code_bytes: usize,
    alignment: u8,
    #[serde(default)]
    nop_fill: bool,
    #[serde(default)]
    exact_root_extent: bool,
}

pub(super) fn profiles() -> Option<String> {
    std::env::var("MECH_COMPACT_PHASE_BLOCK_LAYOUT").ok()
}

/// Pure comparison lowering also applies to checked roots and composed bodies.
/// Set it before profile inspection and before any direct body compilation.
pub(super) fn apply_comparison_options(context: &mut Context, symbol: &str) {
    if std::env::var("MECH_COMPACT_CCMP_IMMEDIATES").is_ok_and(|targets| {
        targets
            .split(';')
            .any(|target| target == "all" || target == symbol)
    }) {
        context.func.aarch64_ccmp_immediates = true;
    }
}

pub(super) fn apply_fp_spill_cache(context: &mut Context, symbol: &str) -> io::Result<()> {
    context.func.aarch64_fp_spill_keep_pairs =
        std::env::var_os("MECH_COMPACT_FP_SPILL_KEEP_PAIRS").is_some();
    if let Ok(selection) = std::env::var("MECH_COMPACT_FP_SPILL_SLOTS") {
        for entry in selection.split(';') {
            let (name, count) = entry
                .split_once('@')
                .ok_or_else(|| io::Error::other("expected phase@spill-cell-count"))?;
            let count = count.parse::<u8>().map_err(io::Error::other)?;
            if name == "all" || name == symbol {
                context.func.aarch64_fp_spill_slots = count;
            }
        }
    }
    Ok(())
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
        context.func.aarch64_block_layout_thread_jumps = profile.thread_jumps;
    }
    Ok(())
}

/// Register weights refer to the full lowered VCode, rather than byte offsets
/// in a previously allocated machine body. Validation occurs after lowering.
pub(super) fn apply_register_weights(context: &mut Context, symbol: &str) -> io::Result<()> {
    apply_comparison_options(context, symbol);
    #[derive(serde::Deserialize)]
    struct RegisterWeights {
        expected_vcode: String,
        block_frequencies: Vec<u32>,
    }
    if let Ok(profiles) = std::env::var("MECH_COMPACT_PHASE_REGISTER_WEIGHTS")
        && let Some(path) = profiles
            .split(';')
            .filter_map(|entry| entry.split_once('@'))
            .find_map(|(name, path)| (name == symbol).then_some(path))
    {
        let weights: RegisterWeights = serde_json::from_slice(&std::fs::read(path)?)?;
        context.func.regalloc_block_frequencies =
            Some((weights.expected_vcode, weights.block_frequencies));
    }
    Ok(())
}

pub(super) fn pad_object_tail(
    profiles: &str,
    symbol: &str,
    code: &mut Box<[u8]>,
) -> io::Result<()> {
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
        // The block layout verifies the leaf's transfers and literal pool.
        // Pad beyond that buffer, leaving code offsets and accounting intact.
        let extent = code.len();
        append_padding(code, extent, padding);
    }
    Ok(())
}

pub(super) fn pad_checked_root_tail(
    profile: &Path,
    alignment: u32,
    code: &mut Box<[u8]>,
) -> io::Result<()> {
    let tail: ObjectTail = serde_json::from_slice(&std::fs::read(profile)?)?;
    let alignment = alignment as usize;
    let padding = tail
        .padding(code.len())
        .filter(|_| {
            alignment >= 4
                && alignment.is_power_of_two()
                && alignment <= usize::from(tail.alignment)
                && (!tail.exact_root_extent
                    || tail.reference_code_bytes - code.len() < usize::from(tail.alignment))
        })
        .ok_or_else(|| io::Error::other("object tail profile does not match the checked root"))?;
    // The measured tail normally follows the rounded function extent.
    // An exact profile restores the reference extent directly.
    let bytes = code.len();
    let extent = if tail.exact_root_extent {
        bytes
    } else {
        bytes.next_multiple_of(alignment)
    };
    if tail.nop_fill {
        append_padding(code, bytes, padding + (extent - bytes));
    } else {
        append_padding(code, extent, padding);
    }
    Ok(())
}

fn append_padding(code: &mut Box<[u8]>, extent: usize, padding: usize) {
    const NOP: [u8; 4] = 0xd503_201f_u32.to_le_bytes();
    if padding != 0 {
        let mut bytes = std::mem::take(code).into_vec();
        bytes.resize(extent, 0);
        for _ in 0..padding / NOP.len() {
            bytes.extend_from_slice(&NOP);
        }
        *code = bytes.into_boxed_slice();
    }
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
            nop_fill: false,
            exact_root_extent: false,
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
