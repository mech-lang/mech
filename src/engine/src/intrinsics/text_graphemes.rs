//! Shared text length semantics for maintained leaves and compiled generators.
//!
//! In ASCII the only multi-byte grapheme is CRLF. Use byte scans for that
//! representation; retain the canonical Unicode segmenter for every other
//! input. No alphabet, program identity, or cached owner assumption is involved.

#[path = "text_ascii.rs"]
mod ascii;
pub use ascii::{has_crlf, is_ascii_without_crlf};

/// Count extended graphemes without constructing an index or allocating.
/// Callers still admit the complete linear scan before invoking this function.
pub fn count(text: &str) -> usize {
    let flags = ascii::flags(text);
    if flags & ascii::NON_ASCII == 0 {
        if flags & ascii::CARRIAGE_RETURN == 0 {
            return text.len();
        }
        text.len()
            - text
                .as_bytes()
                .windows(2)
                .filter(|pair| *pair == b"\r\n")
                .count()
    } else {
        grapheme::Graphemes::from_usvs(text).iter().count()
    }
}
