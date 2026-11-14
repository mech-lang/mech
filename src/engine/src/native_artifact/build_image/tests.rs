use super::*;
use serde::Serialize;

fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, Error> {
    let mut reader = BuildOwnedImageReader::from_image(bytes)?;
    let value = T::deserialize(&mut reader)?;
    reader.end()?;
    Ok(value)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Item {
    Unit,
    One(u128),
    Pair(i128, bool),
    Named { text: String, optional: Option<u64> },
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    item: Item,
    bytes: Vec<u8>,
    signed: i64,
    unsigned: u64,
    float: f64,
}

#[test]
fn typed_metadata_preserves_wire_types_and_full_width_scalars() {
    for item in [
        Item::Unit,
        Item::One(u128::MAX),
        Item::Pair(i128::MIN, true),
        Item::Named {
            text: "escaped\n\"é😀".into(),
            optional: Some(u64::MAX),
        },
        Item::Named {
            text: "".into(),
            optional: None,
        },
    ] {
        let expected = Record {
            item,
            bytes: vec![0, 1, 255],
            signed: i64::MIN,
            unsigned: u64::MAX,
            float: -0.0,
        };
        let json = serde_json::to_vec(&expected).unwrap();
        let image = encode_native_metadata_image(&json).unwrap();
        let actual: Record = decode(&image).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(actual.float.to_bits(), expected.float.to_bits());
        assert_eq!(serde_json::to_vec(&actual).unwrap(), json);
        assert_eq!(decode::<Record>(&json).unwrap(), expected);
    }
}

#[test]
fn records_borrow_the_image_and_decode_without_json_materialization() {
    #[derive(Deserialize)]
    struct Outer<'a> {
        #[serde(borrow)]
        nested: BuildOwnedImageRecord<'a>,
    }
    #[derive(Deserialize, PartialEq, Debug)]
    struct Inner {
        value: u128,
    }
    let json = format!("{{\"nested\":{{\"value\":{}}}}}", u128::MAX);
    let native = encode_native_metadata_image(json.as_bytes()).unwrap();
    let outer: Outer<'_> = decode(&native).unwrap();
    let BuildOwnedImageRecord::Native(bytes) = outer.nested else {
        panic!("native record was expanded")
    };
    assert!((bytes.as_ptr() as usize) >= native.as_ptr() as usize);
    assert!((bytes.as_ptr() as usize + bytes.len()) <= native.as_ptr() as usize + native.len());
    assert_eq!(
        outer.nested.decode::<Inner>().unwrap(),
        Inner { value: u128::MAX }
    );
    let outer: Outer<'_> = decode(json.as_bytes()).unwrap();
    assert!(matches!(outer.nested, BuildOwnedImageRecord::Json(_)));
    assert_eq!(
        outer.nested.decode::<Inner>().unwrap(),
        Inner { value: u128::MAX }
    );
}

#[test]
fn typed_metadata_rejects_duplicate_keys_invalid_widths_and_shapes() {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Small {
        _value: u8,
    }
    for json in [
        r#"{"_value":0,"_value":1}"#,
        r#"{"_value":256}"#,
        r#"{"_value":-1}"#,
        r#"{"_value":1,"extra":0}"#,
        "{}",
        "[]",
        r#"{"_value":1.0}"#,
        r#"{"_value":null}"#,
    ] {
        let native = encode_native_metadata_image(json.as_bytes()).unwrap();
        assert!(decode::<Small>(&native).is_err(), "{json}");
    }
    let native = encode_native_metadata_image(b"[1,2]").unwrap();
    assert!(decode::<[u8; 1]>(&native).is_err());
    let native = encode_native_metadata_image(b"18446744073709551616").unwrap();
    assert!(decode::<u64>(&native).is_err());
    assert_eq!(decode::<u128>(&native).unwrap(), 1u128 << 64);
}

#[test]
fn typed_metadata_rejects_truncation_trailing_bytes_invalid_counts_and_text() {
    let original = encode_native_metadata_image(br#"{"x":[1,"text",null,true,false]}"#).unwrap();
    for end in 0..original.len() {
        assert!(
            decode::<serde_json::Value>(&original[..end]).is_err(),
            "end={end}"
        );
    }
    let mut changed = original.clone();
    changed.push(0);
    assert!(decode::<serde_json::Value>(&changed).is_err());
    let mut changed = original.clone();
    changed[8] = 255;
    assert!(decode::<serde_json::Value>(&changed).is_err());
    let mut changed = original;
    changed[13..17].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode::<serde_json::Value>(&changed).is_err());
    let mut changed = encode_native_metadata_image(br#""x""#).unwrap();
    changed[13] = 255;
    assert!(decode::<String>(&changed).is_err());
    let mut changed = encode_native_metadata_image(b"1.0").unwrap();
    changed[13..21].copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(decode::<f64>(&changed).is_err());
}

#[test]
fn typed_metadata_rejects_multiple_enum_tags_and_noncanonical_unit_payload() {
    for json in [
        r#"{"Unit":null}"#,
        r#"{"Unit":null,"One":1}"#,
        r#"{"Pair":[1,true,2]}"#,
    ] {
        let image = encode_native_metadata_image(json.as_bytes()).unwrap();
        assert!(decode::<Item>(&image).is_err());
    }
}

#[test]
fn canonical_native_constants_require_explicit_fields_and_options() {
    #[derive(Deserialize)]
    struct Optional {
        _value: Option<u64>,
    }
    for json in [b"{}".as_slice(), br#"{"_value":null,"extra":0}"#] {
        let image = encode_native_metadata_image(json).unwrap();
        let mut reader = BuildOwnedImageReader::from_image(&image)
            .unwrap()
            .canonical();
        assert!(Optional::deserialize(&mut reader).is_err());
    }
    let image = encode_native_metadata_image(br#"{"_value":null}"#).unwrap();
    let mut reader = BuildOwnedImageReader::from_image(&image)
        .unwrap()
        .canonical();
    assert!(Optional::deserialize(&mut reader).is_ok());
    reader.end().unwrap();
}

#[test]
fn typed_metadata_map_keys_match_the_original_json_vocabulary() {
    use std::collections::BTreeMap;
    for values in [
        BTreeMap::from([(0u128, 1u8), (u128::MAX, 2)]),
        BTreeMap::new(),
    ] {
        let json = serde_json::to_vec(&values).unwrap();
        let image = encode_native_metadata_image(&json).unwrap();
        assert_eq!(decode::<BTreeMap<u128, u8>>(&image).unwrap(), values);
    }
    let values = BTreeMap::from([(i128::MIN, 1u8), (i128::MAX, 2)]);
    let json = serde_json::to_vec(&values).unwrap();
    let image = encode_native_metadata_image(&json).unwrap();
    assert_eq!(decode::<BTreeMap<i128, u8>>(&image).unwrap(), values);
    for key in ["-0", "01", "+1", "1e0", "-", "18446744073709551616"] {
        let json = format!("{{\"{key}\":1}}");
        let image = encode_native_metadata_image(json.as_bytes()).unwrap();
        let expected = serde_json::from_str::<BTreeMap<u64, u8>>(&json);
        let actual = decode::<BTreeMap<u64, u8>>(&image);
        assert_eq!(actual.is_ok(), expected.is_ok(), "{key}");
        if let (Ok(actual), Ok(expected)) = (actual, expected) {
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn typed_metadata_compact_integer_widths_preserve_boundary_values() {
    for value in [
        0,
        1,
        255,
        256,
        65535,
        65536,
        u32::MAX as u64,
        u32::MAX as u64 + 1,
        u64::MAX,
    ] {
        let image = encode_native_metadata_image(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(decode::<u64>(&image).unwrap(), value);
    }
    for value in [
        -1,
        -128,
        -129,
        -32768,
        -32769,
        i32::MIN as i64,
        i32::MIN as i64 - 1,
        i64::MIN,
    ] {
        let image = encode_native_metadata_image(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(decode::<i64>(&image).unwrap(), value);
    }
}

#[test]
fn native_struct_keys_preserve_reordered_fields_and_reject_duplicates() {
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Pair {
        a: Option<u64>,
        b: u64,
    }
    for json in [br#"{"a":null,"b":2}"#.as_slice(), br#"{"b":2,"a":null}"#] {
        let bytes = encode_native_metadata_image(json).unwrap();
        let mut reader = BuildOwnedImageReader::from_image(&bytes)
            .unwrap()
            .canonical();
        assert_eq!(
            Pair::deserialize(&mut reader).unwrap(),
            Pair { a: None, b: 2 }
        );
        reader.end().unwrap();
    }
    for json in [
        br#"{"a":null,"a":null}"#.as_slice(),
        br#"{"a":null,"extra":2}"#,
        br#"{"b":2}"#,
    ] {
        let bytes = encode_native_metadata_image(json).unwrap();
        let mut reader = BuildOwnedImageReader::from_image(&bytes)
            .unwrap()
            .canonical();
        assert!(Pair::deserialize(&mut reader).is_err());
    }
}

#[test]
fn native_children_reject_visitors_that_return_without_consuming_the_record() {
    struct Ignored;
    impl<'de> Deserialize<'de> for Ignored {
        fn deserialize<D: Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
            Ok(Self)
        }
    }
    #[derive(Deserialize)]
    struct Wrapper(#[allow(dead_code)] Ignored);
    #[derive(Deserialize)]
    enum Variant {
        Value(Ignored),
    }
    let value = encode_native_metadata_image(b"1").unwrap();
    assert!(decode::<Option<Ignored>>(&value).is_err());
    assert!(decode::<Wrapper>(&value).is_err());
    let array = encode_native_metadata_image(b"[1]").unwrap();
    assert!(decode::<Vec<Ignored>>(&array).is_err());
    let object = encode_native_metadata_image(br#"{"a":1}"#).unwrap();
    assert!(decode::<std::collections::BTreeMap<String, Ignored>>(&object).is_err());
    let variant = encode_native_metadata_image(br#"{"Value":1}"#).unwrap();
    assert!(decode::<Variant>(&variant).is_err());
}

#[test]
fn native_children_preserve_canonical_shape_checks_through_options_and_newtypes() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Fields {
        a: Option<u64>,
        b: u64,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    struct Wrapper(Fields);
    for json in [
        br#"[{"a":null,"b":2}]"#.as_slice(),
        br#"[{"b":2,"a":null}]"#,
    ] {
        let bytes = encode_native_metadata_image(json).unwrap();
        let mut reader = BuildOwnedImageReader::from_image(&bytes)
            .unwrap()
            .canonical();
        assert_eq!(
            Vec::<Option<Wrapper>>::deserialize(&mut reader).unwrap(),
            vec![Some(Wrapper(Fields { a: None, b: 2 }))]
        );
        reader.end().unwrap();
    }
    for json in [
        br#"[{"b":2}]"#.as_slice(),
        br#"[{"a":null,"a":null}]"#,
        br#"[{"a":null,"b":2,"extra":0}]"#,
    ] {
        let bytes = encode_native_metadata_image(json).unwrap();
        let mut reader = BuildOwnedImageReader::from_image(&bytes)
            .unwrap()
            .canonical();
        assert!(Vec::<Option<Wrapper>>::deserialize(&mut reader).is_err());
    }
}

fn decode_canonical<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, Error> {
    let mut reader = BuildOwnedImageReader::from_image(bytes)?.canonical();
    let value = T::deserialize(&mut reader)?;
    reader.end()?;
    Ok(value)
}

#[test]
fn canonical_structs_preserve_all_field_orders_and_nested_full_width_values() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Fields {
        optional: Option<u64>,
        unsigned: u64,
        text: String,
        item: Item,
        wide: u128,
    }
    let entries = [
        r#""optional":null"#,
        r#""unsigned":18446744073709551615"#,
        r#""text":"é😀""#,
        r#""item":{"Named":{"optional":null,"text":"nested"}}"#,
        r#""wide":340282366920938463463374607431768211455"#,
    ];
    let expected = Fields {
        optional: None,
        unsigned: u64::MAX,
        text: "é😀".into(),
        item: Item::Named {
            text: "nested".into(),
            optional: None,
        },
        wide: u128::MAX,
    };
    fn permutations(at: usize, order: &mut [usize], visit: &mut impl FnMut(&[usize])) {
        if at == order.len() {
            visit(order);
            return;
        }
        for i in at..order.len() {
            order.swap(at, i);
            permutations(at + 1, order, visit);
            order.swap(at, i);
        }
    }
    let mut checked = 0;
    permutations(0, &mut [0, 1, 2, 3, 4], &mut |order| {
        let json = format!(
            "{{{}}}",
            order
                .iter()
                .map(|&i| entries[i])
                .collect::<Vec<_>>()
                .join(",")
        );
        let native = encode_native_metadata_image(json.as_bytes()).unwrap();
        assert_eq!(decode_canonical::<Fields>(&native).unwrap(), expected);
        checked += 1;
    });
    assert_eq!(checked, 120);
}

// This custom struct visitor deliberately accepts duplicate/unknown map keys.
// Canonical field-set validation must come from the reader, not serde derive.
#[derive(Debug)]
struct PermissiveStruct<const N: usize>(std::collections::BTreeMap<String, u64>);
impl<'de, const N: usize> Deserialize<'de> for PermissiveStruct<N> {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        static NAMES: [&str; 65] = [
            "f00", "f01", "f02", "f03", "f04", "f05", "f06", "f07", "f08", "f09", "f10", "f11",
            "f12", "f13", "f14", "f15", "f16", "f17", "f18", "f19", "f20", "f21", "f22", "f23",
            "f24", "f25", "f26", "f27", "f28", "f29", "f30", "f31", "f32", "f33", "f34", "f35",
            "f36", "f37", "f38", "f39", "f40", "f41", "f42", "f43", "f44", "f45", "f46", "f47",
            "f48", "f49", "f50", "f51", "f52", "f53", "f54", "f55", "f56", "f57", "f58", "f59",
            "f60", "f61", "f62", "f63", "f64",
        ];
        struct Read<const N: usize>;
        impl<'de, const N: usize> Visitor<'de> for Read<N> {
            type Value = PermissiveStruct<N>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a struct with a declared field set")
            }
            fn visit_map<A: MapAccess<'de>>(self, fields: A) -> Result<Self::Value, A::Error> {
                std::collections::BTreeMap::deserialize(de::value::MapAccessDeserializer::new(
                    fields,
                ))
                .map(PermissiveStruct)
            }
        }
        reader.deserialize_struct("PermissiveStruct", &NAMES[..N], Read::<N>)
    }
}

#[test]
fn canonical_struct_field_sets_cover_empty_full_word_and_wide_fallback_boundaries() {
    fn check<const N: usize>() {
        for reverse in [false, true] {
            let mut entries: Vec<_> = (0..N).map(|i| format!("\"f{i:02}\":{i}")).collect();
            if reverse {
                entries.reverse();
            }
            let json = format!("{{{}}}", entries.join(","));
            let native = encode_native_metadata_image(json.as_bytes()).unwrap();
            let value = decode_canonical::<PermissiveStruct<N>>(&native).unwrap().0;
            assert_eq!(value.len(), N);
            assert!(
                value
                    .iter()
                    .all(|(key, value)| key == &format!("f{value:02}"))
            );
            if N != 0 {
                for replacement in ["\"unknown\":0", "\"f00\":0"] {
                    let mut broken = entries.clone();
                    // In ascending order replace the final field, including bit63;
                    // in reverse order replace its first entry with an early key.
                    broken[if reverse { 0 } else { N - 1 }] = replacement.into();
                    if N == 1 && replacement == "\"f00\":0" {
                        continue;
                    }
                    let native = encode_native_metadata_image(
                        format!("{{{}}}", broken.join(",")).as_bytes(),
                    )
                    .unwrap();
                    assert!(
                        decode_canonical::<PermissiveStruct<N>>(&native).is_err(),
                        "N={N}, reverse={reverse}, replacement={replacement}"
                    );
                }
            }
        }
    }
    check::<0>();
    check::<1>();
    check::<2>();
    check::<63>();
    check::<64>();
    check::<65>();
}

#[test]
fn canonical_structs_reject_visitors_that_stop_before_every_field_value() {
    struct Early<const MODE: u8>;
    impl<'de, const MODE: u8> Deserialize<'de> for Early<MODE> {
        fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
            struct Read<const MODE: u8>;
            impl<'de, const MODE: u8> Visitor<'de> for Read<MODE> {
                type Value = Early<MODE>;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("a deliberately incomplete struct visitor")
                }
                fn visit_map<A: MapAccess<'de>>(
                    self,
                    mut fields: A,
                ) -> Result<Self::Value, A::Error> {
                    if MODE == 1 {
                        let _ = fields.next_key::<String>()?;
                    }
                    if MODE == 2 {
                        let _ = fields.next_entry::<String, u64>()?;
                    }
                    Ok(Early)
                }
            }
            reader.deserialize_struct("Early", &["a", "b"], Read::<MODE>)
        }
    }
    for json in [br#"{"a":1,"b":2}"#.as_slice(), br#"{"b":2,"a":1}"#] {
        let native = encode_native_metadata_image(json).unwrap();
        assert!(decode_canonical::<Early<0>>(&native).is_err());
        assert!(decode_canonical::<Early<1>>(&native).is_err());
        assert!(decode_canonical::<Early<2>>(&native).is_err());
    }
}

#[test]
fn canonical_structs_reject_truncation_invalid_keys_counts_and_child_lengths() {
    let original =
        encode_native_metadata_image(br#"{"f00":0,"f01":18446744073709551615}"#).unwrap();
    for end in 0..original.len() {
        assert!(
            decode_canonical::<PermissiveStruct<2>>(&original[..end]).is_err(),
            "end={end}"
        );
    }
    let mut changed = original.clone();
    changed.push(0);
    assert!(decode_canonical::<PermissiveStruct<2>>(&changed).is_err());
    let mut changed = original.clone();
    changed[13..17].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_canonical::<PermissiveStruct<2>>(&changed).is_err());
    let mut changed = original.clone();
    changed[21] = 255;
    assert!(decode_canonical::<PermissiveStruct<2>>(&changed).is_err());
    let mut changed = original;
    changed[25..29].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_canonical::<PermissiveStruct<2>>(&changed).is_err());
}

#[test]
fn streamed_noncanonical_struct_fields_match_json_missing_duplicate_and_unknown_policies() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Loose {
        f00: Option<u64>,
        f01: Option<u64>,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Strict {
        f00: Option<u64>,
        f01: Option<u64>,
    }
    fn check<T: for<'de> Deserialize<'de> + std::fmt::Debug + PartialEq>(json: &str) {
        let expected = serde_json::from_str::<T>(json);
        let native = encode_native_metadata_image(json.as_bytes()).unwrap();
        let actual = decode::<T>(&native);
        assert_eq!(actual.is_ok(), expected.is_ok(), "{json}");
        if let (Ok(actual), Ok(expected)) = (actual, expected) {
            assert_eq!(actual, expected, "{json}");
        }
    }
    for json in [
        "{}",
        r#"{"f00":0}"#,
        r#"{"f01":null}"#,
        r#"{"f00":1,"f01":2}"#,
        r#"{"f01":2,"f00":1}"#,
        r#"{"f00":0,"f00":1}"#,
        r#"{"f00":null,"f00":null}"#,
        r#"{"unknown":9,"f00":1}"#,
        r#"{"unknown":9,"f00":1,"f01":2}"#,
        r#"{"f01":2,"unknown":{"nested":[1,null,"é😀"]},"f00":1}"#,
        r#"{"unknown":1,"unknown":2}"#,
        r#"{"é😀":3,"f00":1,"f01":2}"#,
        r#"{"\u006600":3,"f01":2}"#,
        r#"{"f00":true,"f01":2}"#,
        r#"{"f00":18446744073709551616,"f01":2}"#,
    ] {
        check::<Loose>(json);
        check::<Strict>(json);
        // This custom visitor accepts arbitrary names and last-value-wins
        // duplicates, even when its declared struct has no fields.
        for count in [0, 2, 65] {
            let expected = serde_json::from_str::<std::collections::BTreeMap<String, u64>>(json);
            let native = encode_native_metadata_image(json.as_bytes()).unwrap();
            let actual = match count {
                0 => decode::<PermissiveStruct<0>>(&native).map(|v| v.0),
                2 => decode::<PermissiveStruct<2>>(&native).map(|v| v.0),
                65 => decode::<PermissiveStruct<65>>(&native).map(|v| v.0),
                _ => unreachable!(),
            };
            assert_eq!(actual.is_ok(), expected.is_ok(), "count={count}, {json}");
            if let (Ok(actual), Ok(expected)) = (actual, expected) {
                assert_eq!(actual, expected);
            }
        }
    }
}

#[test]
fn streamed_struct_fields_still_reject_invalid_bytes_and_incomplete_visitors() {
    let original = encode_native_metadata_image(br#"{"f01":2,"f00":1}"#).unwrap();
    for end in 0..original.len() {
        assert!(
            decode::<PermissiveStruct<2>>(&original[..end]).is_err(),
            "end={end}"
        );
    }
    let mut changed = original.clone();
    changed[21] = 255;
    assert!(decode::<PermissiveStruct<2>>(&changed).is_err());
    let mut changed = original.clone();
    changed[25..29].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode::<PermissiveStruct<2>>(&changed).is_err());
    struct Early;
    impl<'de> Deserialize<'de> for Early {
        fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
            struct Read;
            impl<'de> Visitor<'de> for Read {
                type Value = Early;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("a deliberately incomplete struct visitor")
                }
                fn visit_map<A: MapAccess<'de>>(self, mut fields: A) -> Result<Early, A::Error> {
                    let _ = fields.next_entry::<String, u64>()?;
                    Ok(Early)
                }
            }
            reader.deserialize_struct("Early", &["f00", "f01"], Read)
        }
    }
    assert!(decode::<Early>(&original).is_err());
}

#[path = "packed_tests.rs"]
mod packed;

#[test]
fn checked_bulk_byte_views_borrow_exact_packed_payload_and_reject_corruption() {
    for values in [vec![], vec![0], vec![255, 0, 17], (0..=255u8).collect()] {
        let json = serde_json::to_vec(&values).unwrap();
        let image = encode_native_metadata_image(&json).unwrap();
        let record: BuildOwnedImageRecord<'_> = decode(&image).unwrap();
        let bytes = record.unsigned_byte_payload().unwrap();
        assert_eq!(bytes.as_ref(), &values);
        assert_eq!(
            matches!(bytes, std::borrow::Cow::Borrowed(_)),
            values.len() >= 4
        );
        if values.len() >= 4 {
            assert!(bytes.as_ptr() as usize >= image.as_ptr() as usize);
            assert!(bytes.as_ptr() as usize + bytes.len() <= image.as_ptr() as usize + image.len());
        }
        assert_eq!(
            BuildOwnedImageRecord::Json(std::str::from_utf8(&json).unwrap())
                .unsigned_byte_payload()
                .unwrap()
                .as_ref(),
            &values
        );
    }
    let image = encode_native_metadata_image(b"[0,1,2,255]").unwrap();
    let node = &image[8..];
    for end in 0..node.len() {
        assert!(
            BuildOwnedImageRecord::Native(&node[..end])
                .unsigned_byte_payload()
                .is_err()
        );
    }
    for offset in [0, 1, 5] {
        let mut changed = node.to_vec();
        changed[offset] ^= 0xff;
        assert!(
            BuildOwnedImageRecord::Native(&changed)
                .unsigned_byte_payload()
                .is_err()
        );
    }
    let mut extended = node.to_vec();
    extended.push(0);
    assert!(
        BuildOwnedImageRecord::Native(&extended)
            .unsigned_byte_payload()
            .is_err()
    );
    for wrong in [
        "null",
        "true",
        "1",
        "[256]",
        "[-1]",
        "[1.0]",
        "[true]",
        "[0,1,2,null]",
        "{}",
    ] {
        let image = encode_native_metadata_image(wrong.as_bytes()).unwrap();
        assert!(
            BuildOwnedImageRecord::Native(&image[8..])
                .unsigned_byte_payload()
                .is_err(),
            "{wrong}"
        );
    }
}
