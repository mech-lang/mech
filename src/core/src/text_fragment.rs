//! Borrowed UTF-8 and byte facts, without ownership or execution authority.
use core::str::Utf8Error;

/// Text validated from the borrowed bytes, with facts for those exact bytes.
/// Private fields prevent safe callers from supplying unchecked hints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Utf8Fragment<'a> {
    text: &'a str,
    ascii: bool,
    line_feeds: usize,
}

impl<'a> Utf8Fragment<'a> {
    /// Validate actual bytes and retain their exact ASCII/LF facts. ASCII
    /// proves UTF-8 directly; every other input uses the standard validator.
    /// No allocation, read beyond the slice, source assumption or permission
    /// is involved. Callers retain their own scan-work admission obligations.
    pub fn decode(bytes: &'a [u8]) -> Result<Self, Utf8Error> {
        let (bits, line_feeds) = byte_facts(bytes);
        let ascii = bits & 0x80 == 0;
        let text = if ascii {
            // SAFETY: every actual byte is below 128, hence valid UTF-8.
            unsafe { core::str::from_utf8_unchecked(bytes) }
        } else {
            core::str::from_utf8(bytes)?
        };
        Ok(Self {
            text,
            ascii,
            line_feeds,
        })
    }

    pub const fn text(self) -> &'a str {
        self.text
    }
    pub const fn is_ascii(self) -> bool {
        self.ascii
    }
    pub const fn line_feeds(self) -> usize {
        self.line_feeds
    }
}

// Retain one reduction boundary. Each inner LF sum has at most 128 terms,
// so predicates stay byte-wide without wrapping before the scalar widening.
#[inline(never)]
fn byte_facts(bytes: &[u8]) -> (u8, usize) {
    let mut bits = 0u8;
    let mut line_feeds = 0usize;
    for chunk in bytes.chunks(128) {
        let (raw, count) = chunk.iter().fold((0u8, 0u8), |(raw, count), &byte| {
            (raw | byte, count + u8::from(byte == b'\n'))
        });
        bits |= raw;
        line_feeds += usize::from(count);
    }
    (bits, line_feeds)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "no_std")]
    use alloc::{string::String, vec, vec::Vec};
    #[cfg(not(feature = "no_std"))]
    use std::{string::String, vec, vec::Vec};

    fn verify(bytes: &[u8]) {
        match (core::str::from_utf8(bytes), Utf8Fragment::decode(bytes)) {
            (Ok(text), Ok(fragment)) => {
                assert_eq!(fragment.text(), text);
                assert_eq!(fragment.text().as_ptr(), bytes.as_ptr());
                assert_eq!(fragment.is_ascii(), text.is_ascii());
                assert_eq!(
                    fragment.line_feeds(),
                    bytes.iter().filter(|&&b| b == b'\n').count()
                );
            }
            (Err(expected), Err(actual)) => {
                assert_eq!(actual.valid_up_to(), expected.valid_up_to());
                assert_eq!(actual.error_len(), expected.error_len());
            }
            other => panic!("validation differs: {bytes:?}: {other:?}"),
        }
    }

    #[test]
    fn utf8_fragment_facts_match_standard_validation_and_all_single_bytes() {
        for byte in 0..=255u8 {
            verify(&[byte]);
            verify(&[b'a', b'\n', byte, b'\r', b'\n']);
        }
        for bytes in [
            &b"\xf0\x9f\xa6"[..],
            &b"\xe0\x80\x80"[..],
            &b"\xed\xa0\x80"[..],
            &b"\xf4\x90\x80\x80"[..],
            &b"x\r\n\0"[..],
            "é漢🦎\r\n".as_bytes(),
        ] {
            for end in 0..=bytes.len() {
                verify(&bytes[..end]);
            }
        }
    }

    #[test]
    fn utf8_fragment_facts_keep_dense_counts_alignments_and_vector_tails() {
        for length in (0..=257).chain([4095, 4096, 4097, 65535, 65536]) {
            for period in [1, 2, 3, 7, 16, 63, 64, 65, 128] {
                let bytes = (0..length + 32)
                    .map(|i| match i % period {
                        0 => b'\n',
                        1 => b'\r',
                        2 => 0,
                        _ => b'x',
                    })
                    .collect::<Vec<_>>();
                for start in 0..32 {
                    verify(&bytes[start..start + length]);
                }
            }
        }
        for count in [1, 7, 31, 64, 127, 128, 129, 1024] {
            let text = "é\r\n漢\n🧑‍🚀".repeat(count);
            verify(text.as_bytes());
        }
        verify(&vec![b'\n'; 128 * 1024]);
        verify(String::new().as_bytes());
    }
}
