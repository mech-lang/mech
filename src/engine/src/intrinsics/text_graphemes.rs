//! Shared text length semantics for maintained leaves and compiled generators.
//!
//! In ASCII the only multi-byte grapheme is CRLF. Use byte scans for that
//! representation; retain the canonical Unicode segmenter for every other
//! input. No alphabet, program identity, or cached owner assumption is involved.

/// Count extended graphemes without constructing an index or allocating.
/// Callers still admit the complete linear scan before invoking this function.
pub fn count(text: &str) -> usize {
    if text.is_ascii() {
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
