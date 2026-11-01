//! Allocation-free byte representation proofs, shared by ordinary and native
//! text operations. These booleans grant no ownership or execution authority.
pub(super) const NON_ASCII: u8 = 0x80;
pub(super) const CARRIAGE_RETURN: u8 = 1;

// Keep one compiled reduction boundary: when inlined into grapheme counting,
// separate high-bit/CR users can turn the reduction into multiple live-outs
// that LLVM leaves scalar. Callers interpret the returned flags afterwards.
#[inline(never)]
pub(super) fn flags(text: &str) -> u8 {
    // Keep raw byte bits and all-bits comparison masks in separate reductions.
    // Mask/normalize their final totals once, not every vector lane. The CR
    // term is exactly zero or 0xff; wrapping subtraction is intentional.
    // Only actual String bytes are borrowed, with no target-specific reads,
    // extra allocation, early exit, or change to the caller's scan admission.
    let (raw, cr) = text.as_bytes().iter().fold((0u8, 0u8), |(raw, cr), &byte| {
        (raw | byte, cr | 0u8.wrapping_sub(u8::from(byte == b'\r')))
    });
    (raw & NON_ASCII) | u8::from(cr != 0)
}

/// Exact CRLF recognition, including Unicode, isolated CR and NUL. Callers
/// still pre-admit the complete representation scan, even on declined paths.
pub fn has_crlf(text: &str) -> bool {
    flags(text) & CARRIAGE_RETURN != 0 && has_pair(text)
}

/// In ASCII, CRLF is the only multi-byte extended grapheme. Combine its
/// absence proof with the ASCII scan; isolated CR remains fully admissible.
pub fn is_ascii_without_crlf(text: &str) -> bool {
    match flags(text) {
        0 => true,
        CARRIAGE_RETURN => !has_pair(text),
        _ => false,
    }
}

fn has_pair(text: &str) -> bool {
    text.as_bytes().windows(2).any(|pair| pair == b"\r\n")
}

#[cfg(test)]
#[path = "text_ascii_tests.rs"]
mod tests;
