use crate::native_artifact::program::runtime::maps::key_domain::*;

#[test]
fn singleton_nonzero_code_and_unused_codes_remain_distinct() {
    let mut alphabet = [0; 2];
    add_text(&mut alphabet, "a").unwrap();
    let singleton = Domain::from_alphabet(alphabet).unwrap();
    assert_eq!((singleton.bits(), singleton.shift()), (1, Some(0)));
    let (encode, decode) = singleton.packing().tables();
    assert_eq!(encode[usize::from(b'a')], 1);
    assert_eq!(decode[1], b'a');
    assert_eq!(decode[0], u8::MAX);

    add_text(&mut alphabet, "bc").unwrap();
    let three = Domain::from_alphabet(alphabet).unwrap();
    assert_eq!((three.bits(), three.shift()), (2, Some(0)));
    let (encode, decode) = three.packing().tables();
    assert_eq!(&decode[..4], &[u8::MAX, b'a', b'b', b'c']);
    for byte in b"abc" {
        assert_eq!(decode[usize::from(encode[usize::from(*byte)])], *byte);
    }
}

#[test]
fn noninjective_bit_windows_fall_back_to_all_ascii_codes() {
    let mut alphabet = [0; 2];
    add_text(&mut alphabet, "\x01\x02\x04\x08").unwrap();
    let sparse = Domain::from_alphabet(alphabet).unwrap();
    assert_eq!(
        (sparse.count(), sparse.bits(), sparse.shift()),
        (4, 2, None)
    );
    assert_eq!(sparse.packing(), Domain::ascii());
    let (encode, decode) = sparse.packing().tables();
    for byte in 0..128u8 {
        assert_eq!(encode[usize::from(byte)], byte);
        assert_eq!(decode[usize::from(byte)], byte);
    }
}

#[test]
fn finite_domains_are_lossless_and_choose_injective_windows_only() {
    for text in [
        "ACGT",
        "ab:!?",
        "a",
        "\0a",
        "ABCXYZ",
        " \t\n",
        "\x01\x02\x04\x08",
    ] {
        let mut alphabet = [0; 2];
        add_text(&mut alphabet, text).unwrap();
        let domain = Domain::from_alphabet(alphabet).unwrap();
        let (encode, decode) = domain.tables();
        assert_eq!(
            domain.count(),
            text.bytes()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
        for byte in 0..128u8 {
            if domain.contains(byte) {
                assert!(encode[usize::from(byte)] < (1u8 << domain.bits));
                assert_eq!(decode[usize::from(encode[usize::from(byte)])], byte);
            } else {
                assert_eq!(encode[usize::from(byte)], u8::MAX);
            }
        }
    }
    let dna = Domain::from_alphabet([
        0,
        (1 << (b'A' - 64)) | (1 << (b'C' - 64)) | (1 << (b'G' - 64)) | (1 << (b'T' - 64)),
    ])
    .unwrap();
    assert_eq!((dna.bits(), dna.shift()), (2, Some(1)));
    let mut sparse = [0; 2];
    add_text(&mut sparse, "\x01\x02\x04\x08").unwrap();
    assert_eq!(Domain::from_alphabet(sparse).unwrap().shift(), None);
    assert_eq!(
        Domain::ascii().tables(),
        (
            core::array::from_fn(|i| i as u8),
            core::array::from_fn(|i| i as u8)
        )
    );
}
