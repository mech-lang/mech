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
