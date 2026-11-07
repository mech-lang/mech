use super::*;
use serde::Serialize;

fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8], canonical: bool) -> Result<T, Error> {
    let mut reader = BuildOwnedImageReader::from_image(bytes)?;
    if canonical {
        reader = reader.canonical();
    }
    let value = T::deserialize(&mut reader)?;
    reader.end()?;
    Ok(value)
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename = "__mech_private_positional_image_v1")]
struct Marked {
    unsigned: u128,
    signed: i128,
    optional: Option<String>,
    #[serde(default)]
    bytes: Vec<u8>,
    float: f64,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Named {
    unsigned: u128,
    signed: i128,
    optional: Option<String>,
    #[serde(default)]
    bytes: Vec<u8>,
    float: f64,
}

#[test]
fn positional_records_keep_full_width_values_defaults_order_and_eager_refusal() {
    let ordered = format!(
        "{{\"unsigned\":{},\"signed\":{},\"optional\":\"é😀\",\"bytes\":[0,1,127,255],\"float\":-0.0}}",
        u128::MAX,
        i128::MIN
    );
    let reordered = format!(
        "{{\"float\":-0.0,\"bytes\":[0,1,127,255],\"optional\":\"é😀\",\"signed\":{},\"unsigned\":{}}}",
        i128::MIN,
        u128::MAX
    );
    for text in [
        ordered,
        reordered,
        r#"{"unsigned":0,"signed":-1,"optional":null,"float":0.0}"#.into(),
        r#"{"unsigned":0,"signed":-1,"float":0.0}"#.into(),
    ] {
        let native = encode_native_metadata_image(text.as_bytes()).unwrap();
        for canonical in [false, true] {
            let named = decode::<Named>(&native, canonical);
            let marked = decode::<Marked>(&native, canonical);
            assert_eq!(named.is_ok(), marked.is_ok(), "{canonical} {text}");
            if let (Ok(named), Ok(marked)) = (named, marked) {
                assert_eq!(
                    serde_json::to_vec(&named).unwrap(),
                    serde_json::to_vec(&marked).unwrap()
                );
                assert_eq!(named.float.to_bits(), marked.float.to_bits());
            }
        }
        let marked = decode::<Marked>(text.as_bytes(), false).unwrap();
        let json = serde_json::to_vec(&marked).unwrap();
        let named = decode::<Named>(&json, false).unwrap();
        assert_eq!(
            json,
            serde_json::to_vec(&named).unwrap(),
            "marker changes no serialized field or value"
        );
    }
    for text in [
        r#"{"unsigned":0,"unsigned":1,"signed":-1,"optional":null,"float":0.0}"#,
        r#"{"unsigned":0,"signed":-1,"optional":null,"bytes":[],"other":0}"#,
        r#"{"unsigned":0,"signed":-1,"optional":null,"bytes":[256],"float":0.0}"#,
        r#"{"unsigned":-1,"signed":-1,"optional":null,"bytes":[],"float":0.0}"#,
        r#"{"unsigned":0,"signed":-1,"optional":null,"bytes":[],"float":"0"}"#,
        r#"{"unsigned":0,"signed":-1,"optional":null,"bytes":[],"float":0.0,"__mech_private_positional_image_v1":true}"#,
    ] {
        let native = encode_native_metadata_image(text.as_bytes()).unwrap();
        for b in [text.as_bytes(), native.as_slice()] {
            assert!(decode::<Named>(b, false).is_err());
            assert!(decode::<Marked>(b, false).is_err());
        }
    }
    let native = encode_native_metadata_image(
        br#"{"unsigned":0,"signed":-1,"optional":null,"bytes":[0,1,255,5],"float":-0.0}"#,
    )
    .unwrap();
    for end in 0..native.len() {
        assert!(
            decode::<Marked>(&native[..end], false).is_err(),
            "truncation {end}"
        );
    }
    let mut trailing = native.clone();
    trailing.push(0);
    assert!(decode::<Marked>(&trailing, false).is_err());
    let mut count = native.clone();
    count[13..17].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode::<Marked>(&count, false).is_err());
}

#[derive(Debug, PartialEq)]
struct Route<const OPTED_IN: bool> {
    sequence: bool,
    values: [u64; 2],
}
impl<'de, const OPTED_IN: bool> Deserialize<'de> for Route<OPTED_IN> {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        struct Read<const OPT: bool>;
        impl<'de, const OPT: bool> Visitor<'de> for Read<OPT> {
            type Value = Route<OPT>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("two typed fields")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
                assert!(
                    OPT,
                    "ordinary map-only visitors must never receive a positional call"
                );
                Ok(Route {
                    sequence: true,
                    values: [
                        a.next_element()?
                            .ok_or_else(|| de::Error::custom("missing first"))?,
                        a.next_element()?
                            .ok_or_else(|| de::Error::custom("missing second"))?,
                    ],
                })
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Pair {
                    first: u64,
                    second: u64,
                }
                let p = Pair::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(Route {
                    sequence: false,
                    values: [p.first, p.second],
                })
            }
        }
        reader.deserialize_struct(
            if OPTED_IN {
                STRUCT
            } else {
                "ordinary_map_visitor"
            },
            &["first", "second"],
            Read::<OPTED_IN>,
        )
    }
}
#[test]
fn positional_records_require_code_opt_in_and_an_exact_checked_envelope() {
    for (text, sequence) in [
        (r#"{"first":3,"second":7}"#, true),
        (r#"{"second":7,"first":3}"#, false),
    ] {
        let image = encode_native_metadata_image(text.as_bytes()).unwrap();
        for canonical in [false, true] {
            assert_eq!(
                decode::<Route<true>>(&image, canonical).unwrap(),
                Route {
                    sequence,
                    values: [3, 7]
                }
            );
            assert_eq!(
                decode::<Route<false>>(&image, canonical).unwrap(),
                Route {
                    sequence: false,
                    values: [3, 7]
                }
            );
        }
    }
    for text in [
        r#"{"first":3,"first":7}"#,
        r#"{"first":3,"other":7}"#,
        r#"{"first":3}"#,
        r#"{"first":3,"second":7,"other":1}"#,
    ] {
        let image = encode_native_metadata_image(text.as_bytes()).unwrap();
        assert!(decode::<Route<true>>(&image, false).is_err());
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename = "__mech_private_positional_image_v1")]
enum Choice {
    Unit,
    Named { first: u64, second: Option<u64> },
}
#[test]
fn positional_enum_records_preserve_variants_fields_and_fallbacks() {
    for text in [
        r#""Unit""#,
        r#"{"Named":{"first":7,"second":null}}"#,
        r#"{"Named":{"second":9,"first":7}}"#,
        r#"{"Named":{"first":7}}"#,
    ] {
        let expected: Choice = serde_json::from_str(text).unwrap();
        let native = encode_native_metadata_image(text.as_bytes()).unwrap();
        assert_eq!(decode::<Choice>(&native, false).unwrap(), expected);
    }
    for text in [
        r#"{"Named":{"first":7,"first":8}}"#,
        r#"{"Named":{"first":7,"other":8}}"#,
        r#"{"Named":{"first":7,"second":"8"}}"#,
        r#"{"Named":{"first":7,"second":8},"Unit":null}"#,
    ] {
        let native = encode_native_metadata_image(text.as_bytes()).unwrap();
        assert!(decode::<Choice>(&native, false).is_err());
    }
}
