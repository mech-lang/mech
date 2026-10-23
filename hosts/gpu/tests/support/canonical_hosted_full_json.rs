//! Reject duplicate JSON members before deriving policy launches. Otherwise a
//! Value round-trip could launder input the generated strict Launch rejects.
use serde::{
    Deserialize, Deserializer,
    de::{Error, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::fmt;

struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = Unique;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("JSON without duplicate object members")
            }
            fn visit_unit<E: Error>(self) -> Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_bool<E: Error>(self, value: bool) -> Result<Unique, E> {
                Ok(Unique(value.into()))
            }
            fn visit_i64<E: Error>(self, value: i64) -> Result<Unique, E> {
                Ok(Unique(value.into()))
            }
            fn visit_u64<E: Error>(self, value: u64) -> Result<Unique, E> {
                Ok(Unique(value.into()))
            }
            fn visit_f64<E: Error>(self, value: f64) -> Result<Unique, E> {
                Number::from_f64(value)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E: Error>(self, value: &str) -> Result<Unique, E> {
                Ok(Unique(value.into()))
            }
            fn visit_string<E: Error>(self, value: String) -> Result<Unique, E> {
                Ok(Unique(value.into()))
            }
            fn visit_seq<S: SeqAccess<'de>>(self, mut sequence: S) -> Result<Unique, S::Error> {
                let mut values = Vec::new();
                while let Some(Unique(value)) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(Unique(Value::Array(values)))
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Unique, M::Error> {
                let mut values = Map::new();
                while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                    if values.insert(key.clone(), value).is_some() {
                        return Err(M::Error::custom(format!("duplicate JSON member {key}")));
                    }
                }
                Ok(Unique(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice::<Unique>(bytes)
        .map(|unique| unique.0)
        .map_err(|e| e.to_string())
}

#[test]
fn launch_derivation_never_launders_duplicate_or_malformed_json() {
    for bytes in [
        r#"{"work":1,"work":2}"#,
        r#"{"library":{"work":null,"work":3}}"#,
        r#"{"grants":[{"target":"a","tar\u0067et":"b"}]}"#,
        r#"[{"x":1,"x":2}]"#,
        "{",
        "NaN",
        "[1,]",
    ] {
        assert!(decode(bytes.as_bytes()).is_err(), "{bytes}");
    }
    for bytes in [
        r#"{"s":"quote \"work\"","a":[null,true,false,-1,2,3.5],"o":{"é":"🧑‍🚀"}}"#,
        "null",
        "[]",
        "{}",
    ] {
        assert_eq!(
            decode(bytes.as_bytes()).unwrap(),
            serde_json::from_str::<Value>(bytes).unwrap()
        );
    }
}
