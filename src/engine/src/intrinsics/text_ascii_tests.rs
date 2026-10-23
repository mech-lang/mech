use super::{CARRIAGE_RETURN, NON_ASCII, flags, has_crlf, is_ascii_without_crlf};

fn check(text: &str) {
    let crlf = text.as_bytes().windows(2).any(|pair| pair == b"\r\n");
    assert_eq!(has_crlf(text), crlf);
    assert_eq!(is_ascii_without_crlf(text), text.is_ascii() && !crlf);
    assert_eq!(
        flags(text),
        if text.is_ascii() { 0 } else { NON_ASCII }
            | if text.as_bytes().contains(&b'\r') {
                CARRIAGE_RETURN
            } else {
                0
            }
    );
}

#[test]
fn combined_ascii_crlf_scan_matches_every_ascii_pair() {
    for first in 0..128u8 {
        for second in 0..128u8 {
            let bytes = [first, second];
            check(std::str::from_utf8(&bytes).unwrap());
        }
    }
    for text in ["", "\r", "\n", "\r\n", "\n\r", "\r\0\n", "\r\r", "\r\r\n"] {
        check(text);
    }
}

#[test]
fn combined_ascii_crlf_scan_keeps_every_tail_alignment_and_unicode_fallback() {
    for length in 0..=4096 {
        let storage = (0..length + 31)
            .map(|index| ((index * 73 + 19) & 127) as u8)
            .collect::<Vec<_>>();
        let storage = String::from_utf8(storage).unwrap();
        for offset in [0, 1, 7, 15, 16, 31] {
            check(&storage[offset..offset + length]);
        }
    }
    for offset in 0..=129 {
        for middle in ["\r\n", "\r\0\n", "λ", "é\r\n", "a\u{301}", "🧑‍🚀", "\u{2028}"] {
            let text = format!("{}{middle}{}", "x".repeat(offset), "y".repeat(137 - offset));
            check(&text);
        }
    }
}

#[test]
fn independent_masks_do_not_mix_low_bits_cr_and_non_ascii_across_batches() {
    for length in [0, 1, 63, 64, 65, 127, 128, 129, 4095, 4096, 4097] {
        for fill in ["\0", "\u{7f}", "a", "\r"] {
            let padding = fill.repeat(length);
            check(&padding);
            for text in [
                format!("é{padding}\r"),
                format!("\r{padding}é"),
                format!("é{padding}\r\n"),
                format!("\r{padding}\0\né"),
            ] {
                check(&text);
            }
        }
    }
}
