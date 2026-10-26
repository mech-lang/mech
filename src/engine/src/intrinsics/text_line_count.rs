//! Allocation-free cardinality of the ordinary `str::lines` iterator.

/// Count LF delimiters and a possible unterminated suffix. CRLF changes each
/// line's contents, not its cardinality; other Unicode separators are ordinary
/// payload, and LF cannot occur inside a multibyte UTF-8 code point.
///
/// Callers admit the complete byte scan before entering. Bounded partial sums
/// keep the byte predicates narrow instead of widening every lane to usize.
#[inline(never)]
pub(crate) fn count(text: &str) -> usize {
    let bytes = text.as_bytes();
    let delimiters = bytes
        .chunks(128)
        .map(|chunk| {
            // At most 128 zero-or-one terms: neither an intermediate nor the
            // final u8 sum can wrap, including a chunk made entirely of LF.
            // Only the bounded scalar total widens; no target-specific code,
            // input-class dispatch, extra allocation or wider byte read.
            chunk
                .iter()
                .map(|&byte| u8::from(byte == b'\n'))
                .sum::<u8>() as usize
        })
        .sum::<usize>();
    // If the suffix exists, its final non-LF byte means delimiters < len.
    // The sum therefore cannot overflow for any valid slice.
    delimiters + usize::from(bytes.last().is_some_and(|&byte| byte != b'\n'))
}

#[cfg(test)]
mod tests {
    use super::count;

    #[test]
    fn matches_standard_empty_crlf_and_unicode_line_semantics() {
        for (text, expected) in [
            ("", 0),
            ("\n", 1),
            ("\n\n", 2),
            ("\r", 1),
            ("\r\n", 1),
            ("\r\r\n", 1),
            ("a\nb", 2),
            ("a\nb\n", 2),
            ("a\r\nb\r\n", 2),
            ("a\r\n\r\nb\r", 3),
            ("\0\n\0", 2),
            ("é\n🧑‍🚀\r\n終", 3),
            ("\u{0085}\u{2028}\u{2029}", 1),
        ] {
            assert_eq!(text.lines().count(), expected);
            assert_eq!(count(text), expected, "{text:?}");
        }
    }

    #[test]
    fn matches_standard_iterator_for_exhaustive_short_utf8_strings() {
        let alphabet = ['a', '\0', '\r', '\n', 'é', '🦀', '\u{0085}', '\u{2028}'];
        for length in 0..=6 {
            for mut ordinal in 0..alphabet.len().pow(length) {
                let mut text = String::new();
                for _ in 0..length {
                    text.push(alphabet[ordinal % alphabet.len()]);
                    ordinal /= alphabet.len();
                }
                assert_eq!(count(&text), text.lines().count(), "{text:?}");
            }
        }
    }

    #[test]
    fn matches_standard_iterator_across_vector_lengths_alignments_and_tails() {
        for length in (0..=257).chain([4095, 4096, 4097]) {
            for period in [1, 2, 3, 7, 16, 63, 64, 65] {
                let text = (0..length + 32)
                    .map(|i| match i % period {
                        0 => '\n',
                        1 => '\r',
                        2 => '\0',
                        _ => 'x',
                    })
                    .collect::<String>();
                for offset in 0..32 {
                    let slice = &text[offset..offset + length];
                    assert_eq!(
                        count(slice),
                        slice.lines().count(),
                        "length={length}, period={period}, offset={offset}"
                    );
                }
            }
        }
    }

    #[test]
    fn narrow_batches_never_wrap_dense_delimiters_or_lose_unterminated_tails() {
        for length in [127, 128, 129, 255, 256, 257, 32768, 65535, 65536, 131071] {
            let mut text = "\n".repeat(length);
            assert_eq!(count(&text), length);
            text.push('x');
            assert_eq!(count(&text), length + 1);
            text.push('\r');
            assert_eq!(count(&text), length + 1);
            text.push('\n');
            assert_eq!(count(&text), length + 1);
            text.push('é');
            assert_eq!(count(&text), length + 2);
            assert_eq!(count(&text), text.lines().count());
        }
    }
}
