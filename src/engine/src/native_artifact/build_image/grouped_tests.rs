//! Grouped keys are a checked data layout, never a source-proof shortcut.
use super::*;

#[derive(Debug, PartialEq)]
struct Probe<const PRIVATE: bool> {
    values: [u64; 2],
    sequence: bool,
}
impl<'de, const PRIVATE: bool> Deserialize<'de> for Probe<PRIVATE> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Read<const PRIVATE: bool>;
        impl<'de, const PRIVATE: bool> Visitor<'de> for Read<PRIVATE> {
            type Value = Probe<PRIVATE>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("the exact two-field private record")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let first = a
                    .next_element()?
                    .ok_or_else(|| de::Error::custom("missing a"))?;
                let second = a
                    .next_element()?
                    .ok_or_else(|| de::Error::custom("missing b"))?;
                Ok(Probe {
                    values: [first, second],
                    sequence: true,
                })
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                let mut first = None;
                let mut second = None;
                while let Some(key) = a.next_key::<String>()? {
                    let slot = match key.as_str() {
                        "a" => &mut first,
                        "b" => &mut second,
                        _ => return Err(de::Error::custom("unknown field")),
                    };
                    if slot.is_some() {
                        return Err(de::Error::custom("duplicate field"));
                    }
                    *slot = Some(a.next_value()?);
                }
                Ok(Probe {
                    values: [
                        first.ok_or_else(|| de::Error::custom("missing a"))?,
                        second.ok_or_else(|| de::Error::custom("missing b"))?,
                    ],
                    sequence: false,
                })
            }
        }
        d.deserialize_struct(
            if PRIVATE {
                "mech.native.image.Probe"
            } else {
                "ordinary.Probe"
            },
            &["a", "b"],
            Read::<PRIVATE>,
        )
    }
}
fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8], canonical: bool) -> Result<T, Error> {
    let mut d = BuildOwnedImageReader::from_image(bytes)?;
    if canonical {
        d = d.canonical();
    }
    let result = T::deserialize(&mut d)?;
    d.end()?;
    Ok(result)
}
#[test]
fn grouped_metadata_uses_single_pass_private_sequences_and_keeps_legacy_maps() {
    let json = br#"{"a":1,"b":18446744073709551615}"#;
    let grouped = encode_native_metadata_image(json).unwrap();
    let packed = encode::interleaved_image(json).unwrap();
    let legacy = encode::legacy_image(json).unwrap();
    assert!(grouped.starts_with(b"MECHTVI4"));
    assert!(packed.starts_with(b"MECHTVI3"));
    for canonical in [false, true] {
        for (bytes, sequence) in [
            (&grouped[..], true),
            (&packed[..], false),
            (&legacy[..], false),
        ] {
            let private = decode::<Probe<true>>(bytes, canonical).unwrap();
            assert_eq!(private.values, [1, u64::MAX]);
            assert_eq!(private.sequence, sequence);
            assert!(!decode::<Probe<false>>(bytes, canonical).unwrap().sequence);
            let raw = decode::<BuildOwnedImageRecord<'_>>(bytes, canonical).unwrap();
            assert_eq!(raw.decode::<Probe<true>>().unwrap().sequence, sequence);
        }
        let reversed = encode_native_metadata_image(br#"{"b":9,"a":3}"#).unwrap();
        let value = decode::<Probe<true>>(&reversed, canonical).unwrap();
        assert_eq!(value.values, [3, 9]);
        assert!(!value.sequence);
    }
}
#[test]
fn grouped_metadata_preserves_optional_duplicate_unknown_and_full_width_fields() {
    #[derive(Debug, PartialEq, Deserialize)]
    #[serde(deny_unknown_fields, rename = "mech.native.image.Row")]
    struct Row {
        a: u64,
        b: Option<u128>,
    }
    for json in [
        r#"{"a":18446744073709551615,"b":340282366920938463463374607431768211455}"#,
        r#"{"a":0,"b":null}"#,
        r#"{"b":1,"a":2}"#,
        r#"{"a":2}"#,
        r#"{"a":1,"a":2,"b":3}"#,
        r#"{"a":1,"b":2,"extra":0}"#,
        r#"{"a":true,"b":null}"#,
        r#"{"a":1,"b":-1}"#,
    ] {
        let grouped = encode_native_metadata_image(json.as_bytes()).unwrap();
        let packed = encode::interleaved_image(json.as_bytes()).unwrap();
        for canonical in [false, true] {
            let actual = decode::<Row>(&grouped, canonical);
            let expected = decode::<Row>(&packed, canonical);
            assert_eq!(
                actual.is_ok(),
                expected.is_ok(),
                "{json} canonical={canonical}"
            );
            if let (Ok(a), Ok(b)) = (actual, expected) {
                assert_eq!(a, b);
            }
        }
    }
}
#[test]
fn grouped_metadata_rejects_every_truncation_invalid_key_block_and_unconsumed_child() {
    let good = encode_native_metadata_image(br#"{"a":1,"b":2}"#).unwrap();
    for cut in 0..good.len() {
        assert!(
            decode::<Probe<true>>(&good[..cut], false).is_err(),
            "cut {cut}"
        );
    }
    for at in [9usize, 13, 17, 21] {
        let mut bad = good.clone();
        bad[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode::<Probe<true>>(&bad, false).is_err(), "offset {at}");
    }
    let mut bad = good.clone();
    bad[25] = 255; // first one-byte field name
    assert!(decode::<Probe<true>>(&bad, false).is_err());
    let mut trailing = good.clone();
    trailing.push(0);
    assert!(decode::<Probe<true>>(&trailing, false).is_err());
    #[derive(Debug)]
    struct Partial;
    impl<'de> Deserialize<'de> for Partial {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct Read;
            impl<'de> Visitor<'de> for Read {
                type Value = Partial;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("partial")
                }
                fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Partial, A::Error> {
                    a.next_element::<u64>()?;
                    Ok(Partial)
                }
            }
            d.deserialize_struct("mech.native.image.Partial", &["a", "b"], Read)
        }
    }
    assert!(decode::<Partial>(&good, false).is_err());
}
