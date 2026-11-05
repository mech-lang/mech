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
