//! Allocation-free, ordered LF byte offsets for ordinary UTF-8 text.
use core::simd::{Simd, cmp::SimdPartialEq};

/// Visit each LF once, stopping immediately if the consumer refuses an offset.
/// Callers admit the full byte scan and any output storage before entering.
/// LF cannot occur inside a multibyte UTF-8 code point. CRLF trimming and an
/// unterminated final line remain the caller's ordinary line semantics.
#[inline]
pub(crate) fn try_for_each_lf<E>(
    text: &str,
    mut visit: impl FnMut(usize) -> Result<(), E>,
) -> Result<(), E> {
    let bytes = text.as_bytes();
    let mut blocks = bytes.chunks_exact(64);
    for (block, bytes) in blocks.by_ref().enumerate() {
        // Portable SIMD uses only this complete slice, with no alignment or
        // target-instruction assumption and no read beyond its 64 bytes.
        let mut mask = Simd::<u8, 64>::from_slice(bytes)
            .simd_eq(Simd::splat(b'\n'))
            .to_bitmask();
        while mask != 0 {
            visit(block * 64 + mask.trailing_zeros() as usize)?;
            mask &= mask - 1;
        }
    }
    let tail = blocks.remainder();
    for (index, byte) in tail.iter().enumerate() {
        if *byte == b'\n' {
            visit(bytes.len() - tail.len() + index)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod line_offset_tests {
    use super::try_for_each_lf;

    fn check(text: &str) {
        let expected = text
            .match_indices('\n')
            .map(|(at, _)| at)
            .collect::<Vec<_>>();
        let mut actual = Vec::new();
        try_for_each_lf(text, |at| {
            actual.push(at);
            Ok::<_, ()>(())
        })
        .unwrap();
        assert_eq!(actual, expected, "{text:?}");
    }

    #[test]
    fn every_alignment_length_and_tail_preserves_order() {
        for length in (0..=257).chain([4095, 4096, 4097, 65535, 65536, 65537]) {
            for period in [1, 2, 3, 7, 16, 63, 64, 65, 257] {
                let text = (0..length + 64)
                    .map(|at| match at % period {
                        0 => '\n',
                        1 => '\r',
                        2 => '\0',
                        _ => 'x',
                    })
                    .collect::<String>();
                for offset in 0..64 {
                    check(&text[offset..offset + length]);
                }
            }
        }
    }

    #[test]
    fn every_single_lf_position_and_no_lf_are_exact() {
        for length in 0..=257 {
            check(&"x".repeat(length));
            for at in 0..length {
                let mut bytes = vec![b'x'; length];
                bytes[at] = b'\n';
                check(core::str::from_utf8(&bytes).unwrap());
            }
        }
    }

    #[test]
    fn unicode_crlf_and_non_lf_separators_remain_payload() {
        for pattern in [
            "é猫🦎e\u{301}\n日本語\r\n",
            "\r\r\n\0\r",
            "\u{0085}\u{2028}\u{2029}",
            "\n\n",
            "",
        ] {
            for count in 0..=129 {
                let text = pattern.repeat(count);
                check(&text);
                for at in text.char_indices().map(|(at, _)| at).take(65) {
                    check(&text[at..]);
                }
            }
        }
    }

    #[test]
    fn consumer_error_stops_in_each_block_and_tail() {
        let text = "\n".repeat(137);
        for stop in 0..text.len() {
            let mut visited = Vec::new();
            let result = try_for_each_lf(&text, |at| {
                visited.push(at);
                if at == stop { Err(at) } else { Ok(()) }
            });
            assert_eq!(result, Err(stop));
            assert_eq!(visited, (0..=stop).collect::<Vec<_>>());
        }
    }
}
