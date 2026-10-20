//! Allocation-free byte representation proofs, shared by ordinary and native
//! text operations. These booleans grant no ownership or execution authority.
pub(super) const NON_ASCII: u8 = 0x80;
pub(super) const CARRIAGE_RETURN: u8 = 1;

#[inline]
pub(super) fn flags(text: &str) -> u8 {
    // A full reduction (not an early-exit byte search) lets the compiler use
    // ordinary vector instructions. Only actual String bytes are borrowed;
    // no unaligned/wide pointer reads, target assumptions or extra scratch.
    text.as_bytes().iter().fold(0, |status, &byte| {
        status | (byte & NON_ASCII) | u8::from(byte == b'\r')
    })
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
