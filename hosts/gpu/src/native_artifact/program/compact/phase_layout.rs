//! Optional layout profiles for diagnostic AArch64 phases.

use crate::native_artifact::codegen::compilation_inputs::CompilationInputs;
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

pub(super) fn profiles(compiler_inputs: &CompilationInputs) -> Option<String> {
    compiler_inputs.var("MECH_COMPACT_PHASE_BLOCK_LAYOUT").ok()
}

/// Pure comparison lowering also applies to checked roots and composed bodies.
/// Set it from the captured inputs before inspection or body compilation.
pub(super) fn apply_comparison_options(
    compiler_inputs: &CompilationInputs,
    context: &mut Context,
    symbol: &str,
) {
    if compiler_inputs
        .var("MECH_COMPACT_CCMP_IMMEDIATES")
        .is_ok_and(|targets| {
            targets
                .split(';')
                .any(|target| target == "all" || target == symbol)
        })
    {
        context.func.aarch64_ccmp_immediates = true;
    }
}

pub(super) fn apply_fp_spill_cache(
    compiler_inputs: &CompilationInputs,
    context: &mut Context,
    symbol: &str,
) -> io::Result<()> {
    context.func.aarch64_fp_spill_keep_pairs = compiler_inputs
        .var_os("MECH_COMPACT_FP_SPILL_KEEP_PAIRS")
        .is_some();
    if let Ok(selection) = compiler_inputs.var("MECH_COMPACT_FP_SPILL_SLOTS") {
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

fn read(
    compiler_inputs: &CompilationInputs,
    profiles: &str,
    symbol: &str,
) -> io::Result<Option<Profile>> {
    profiles
        .split(';')
        .filter_map(|entry| entry.split_once('@'))
        .find_map(|(name, path)| (name == symbol).then_some(path))
        .map(|path| {
            Ok(serde_json::from_slice(
                compiler_inputs.profile(Path::new(path))?,
            )?)
        })
        .transpose()
}

pub(super) fn apply(
    compiler_inputs: &CompilationInputs,
    context: &mut Context,
    symbol: &str,
) -> io::Result<()> {
    if let Some(profiles) = profiles(compiler_inputs)
        && let Some(profile) = read(compiler_inputs, &profiles, symbol)?
    {
        context.func.aarch64_block_layout = profile.order;
        context.func.aarch64_block_layout_literal_start = profile.literal_offset;
        context.func.aarch64_block_layout_thread_jumps = profile.thread_jumps;
    }
    Ok(())
}

/// Register weights refer to the full lowered VCode, rather than byte offsets
/// in a previously allocated machine body. Validation occurs after lowering.
pub(super) fn apply_register_weights(
    compiler_inputs: &CompilationInputs,
    context: &mut Context,
    symbol: &str,
) -> io::Result<()> {
    apply_comparison_options(compiler_inputs, context, symbol);
    #[derive(serde::Deserialize)]
    struct RegisterWeights {
        expected_vcode: String,
        block_frequencies: Vec<u32>,
    }
    if let Ok(profiles) = compiler_inputs.var("MECH_COMPACT_PHASE_REGISTER_WEIGHTS")
        && let Some(path) = profiles
            .split(';')
            .filter_map(|entry| entry.split_once('@'))
            .find_map(|(name, path)| (name == symbol).then_some(path))
    {
        let weights: RegisterWeights =
            serde_json::from_slice(compiler_inputs.profile(Path::new(path))?)?;
        context.func.regalloc_block_frequencies =
            Some((weights.expected_vcode, weights.block_frequencies));
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn phase_layout_consumes_captured_bytes_and_defers_profile_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("layout.json");
    std::fs::write(&path, br#"{"order":[0,8],"literal_offset":12}"#).unwrap();
    let capture = || {
        CompilationInputs::capture([(
            "MECH_COMPACT_PHASE_BLOCK_LAYOUT".into(),
            format!("selected@{}", path.display()).into(),
        )])
    };
    let original = capture();
    std::fs::write(&path, b"malformed").unwrap();
    let malformed = capture();
    std::fs::remove_file(&path).unwrap();
    let missing = capture();
    let mut context = Context::new();
    apply(&original, &mut context, "selected").unwrap();
    assert_eq!(context.func.aarch64_block_layout, [0, 8]);
    assert_eq!(context.func.aarch64_block_layout_literal_start, Some(12));
    apply(&malformed, &mut context, "irrelevant").unwrap();
    assert!(apply(&malformed, &mut context, "selected").is_err());
    apply(&missing, &mut context, "irrelevant").unwrap();
    assert_eq!(
        apply(&missing, &mut context, "selected")
            .unwrap_err()
            .kind(),
        io::ErrorKind::NotFound,
    );
}

pub(super) fn pad_object_tail(
    compiler_inputs: &CompilationInputs,
    profiles: &str,
    symbol: &str,
    code: &mut Box<[u8]>,
) -> io::Result<()> {
    if let Some(profile) = read(compiler_inputs, profiles, symbol)?
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
    compiler_inputs: &CompilationInputs,
    profile: &Path,
    alignment: u32,
    code: &mut Box<[u8]>,
) -> io::Result<()> {
    let tail: ObjectTail = serde_json::from_slice(compiler_inputs.profile(profile)?)?;
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

#[cfg(test)]
#[test]
fn comparison_options_use_captured_phase_selection() {
    let inputs = CompilationInputs::capture([(
        "MECH_COMPACT_CCMP_IMMEDIATES".into(),
        "first;selected".into(),
    )]);
    let mut selected = Context::new();
    let mut other = Context::new();
    apply_comparison_options(&inputs, &mut selected, "selected");
    apply_comparison_options(&inputs, &mut other, "other");
    assert!(selected.func.aarch64_ccmp_immediates);
    assert!(!other.func.aarch64_ccmp_immediates);
    let inputs =
        CompilationInputs::capture([("MECH_COMPACT_CCMP_IMMEDIATES".into(), "all".into())]);
    apply_comparison_options(&inputs, &mut other, "other");
    assert!(other.func.aarch64_ccmp_immediates);
}
