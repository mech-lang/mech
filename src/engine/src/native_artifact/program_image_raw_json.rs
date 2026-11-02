//! Strict canonical wire decoding without rounding integer tokens through Value.
use serde::{
    Deserialize, Deserializer,
    de::{
        DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
        Visitor, value::StringDeserializer,
    },
};
use serde_json::{Error, value::RawValue};
use std::collections::BTreeMap;

pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    let raw: &RawValue = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    <T as Deserialize>::deserialize(Canonical { raw, depth: 0 }).map_err(|e| e.to_string())
}
fn invalid() -> Error {
    serde::de::Error::custom("build image differs from its canonical wire structure")
}
#[derive(Clone, Copy)]
struct Canonical<'a> {
    raw: &'a RawValue,
    depth: usize,
}
impl<'a> Canonical<'a> {
    fn object(self) -> Result<BTreeMap<String, &'a RawValue>, Error> {
        if self.depth >= 128 {
            return Err(invalid());
        }
        serde_json::from_str(self.raw.get())
    }
    fn sequence(self) -> Result<Vec<&'a RawValue>, Error> {
        if self.depth >= 128 {
            return Err(invalid());
        }
        serde_json::from_str(self.raw.get())
    }
}
macro_rules! primitive {
    ($($method:ident),+ $(,)?) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            if self.depth > 128 { return Err(invalid()); }
            let mut reader = serde_json::Deserializer::from_str(self.raw.get());
            let value = reader.$method(visitor)?;
            reader.end()?;
            Ok(value)
        }
    )+};
}
impl<'de> Deserializer<'de> for Canonical<'de> {
    type Error = Error;
    primitive! { deserialize_bool, deserialize_i8, deserialize_i16, deserialize_i32,
    deserialize_i64, deserialize_i128, deserialize_u8, deserialize_u16, deserialize_u32,
    deserialize_u64, deserialize_u128, deserialize_f32, deserialize_f64, deserialize_char,
    deserialize_str, deserialize_string, deserialize_identifier }
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.raw.get().as_bytes().first() {
            Some(b'[') => self.deserialize_seq(visitor),
            Some(b'{') => self.deserialize_map(visitor),
            _ => {
                if self.depth > 128 {
                    return Err(invalid());
                }
                let mut reader = serde_json::Deserializer::from_str(self.raw.get());
                let value = reader.deserialize_any(visitor)?;
                reader.end()?;
                Ok(value)
            }
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.raw.get() == "null" {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.raw.get() == "null" {
            visitor.visit_unit()
        } else {
            Err(invalid())
        }
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_seq(Sequence {
            entries: self.sequence()?.into_iter(),
            depth: self.depth + 1,
        })
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        length: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        let entries = self.sequence()?;
        if entries.len() != length {
            return Err(invalid());
        }
        visitor.visit_seq(Sequence {
            entries: entries.into_iter(),
            depth: self.depth + 1,
        })
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        length: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_tuple(length, visitor)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_map(Object {
            entries: self.object()?.into_iter(),
            value: None,
            depth: self.depth + 1,
        })
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let entries = self.object()?;
        if entries.len() != fields.len() || fields.iter().any(|key| !entries.contains_key(*key)) {
            return Err(invalid());
        }
        visitor.visit_map(Object {
            entries: entries.into_iter(),
            value: None,
            depth: self.depth + 1,
        })
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let (name, payload) = if self.raw.get().starts_with('"') {
            (serde_json::from_str(self.raw.get())?, None)
        } else {
            let entries = self.object()?;
            if entries.len() != 1 {
                return Err(invalid());
            }
            let (name, value) = entries.into_iter().next().ok_or_else(invalid)?;
            (name, Some(value))
        };
        visitor.visit_enum(Enumeration {
            name,
            payload,
            depth: self.depth + 1,
        })
    }
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.deserialize_seq(visitor)
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
        Err(invalid())
    }
}
struct Sequence<'a> {
    entries: std::vec::IntoIter<&'a RawValue>,
    depth: usize,
}
impl<'de> SeqAccess<'de> for Sequence<'de> {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        self.entries
            .next()
            .map(|raw| {
                seed.deserialize(Canonical {
                    raw,
                    depth: self.depth,
                })
            })
            .transpose()
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}
struct Object<'a> {
    entries: std::collections::btree_map::IntoIter<String, &'a RawValue>,
    value: Option<&'a RawValue>,
    depth: usize,
}
impl<'de> MapAccess<'de> for Object<'de> {
    type Error = Error;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        self.entries
            .next()
            .map(|(key, value)| {
                self.value = Some(value);
                seed.deserialize(StringDeserializer::<Error>::new(key))
            })
            .transpose()
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        seed.deserialize(Canonical {
            raw: self.value.take().ok_or_else(invalid)?,
            depth: self.depth,
        })
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}
struct Enumeration<'a> {
    name: String,
    payload: Option<&'a RawValue>,
    depth: usize,
}
impl<'de> EnumAccess<'de> for Enumeration<'de> {
    type Error = Error;
    type Variant = Self;
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self), Error> {
        let name = seed.deserialize(StringDeserializer::<Error>::new(self.name.clone()))?;
        Ok((name, self))
    }
}
impl<'de> VariantAccess<'de> for Enumeration<'de> {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error> {
        if self.payload.is_none() {
            Ok(())
        } else {
            Err(invalid())
        }
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(Canonical {
            raw: self.payload.ok_or_else(invalid)?,
            depth: self.depth,
        })
    }
    fn tuple_variant<V: Visitor<'de>>(self, length: usize, visitor: V) -> Result<V::Value, Error> {
        Canonical {
            raw: self.payload.ok_or_else(invalid)?,
            depth: self.depth,
        }
        .deserialize_tuple(length, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        Canonical {
            raw: self.payload.ok_or_else(invalid)?,
            depth: self.depth,
        }
        .deserialize_struct("variant", fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    enum Item {
        Unit,
        Unsigned(u128),
        Signed(i128),
        Pair(u8, bool),
        Named { value: u128, optional: Option<i128> },
    }
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Record {
        item: Item,
        optional: Option<u128>,
    }
    #[test]
    fn raw_canonical_json_preserves_all_128_bit_tokens() {
        for item in [
            Item::Unit,
            Item::Unsigned(u64::MAX as u128 + 1),
            Item::Unsigned(u64::MAX as u128 + 2),
            Item::Unsigned(u128::MAX),
            Item::Signed(i64::MIN as i128 - 1),
            Item::Signed(i128::MIN),
            Item::Signed(i128::MAX),
            Item::Pair(2, true),
            Item::Named {
                value: u128::MAX,
                optional: Some(i128::MIN),
            },
        ] {
            let record = Record {
                item,
                optional: None,
            };
            let bytes = serde_json::to_vec(&record).unwrap();
            assert_eq!(decode::<Record>(&bytes).unwrap(), record);
        }
    }
    #[test]
    fn raw_canonical_json_preserves_strict_wire_structure() {
        for bytes in [
            r#"{"item":"Unit"}"#,
            r#"{"item":"Unit","optional":null,"extra":0}"#,
            r#"{"item":{"Unit":null},"optional":null}"#,
            r#"{"item":{"Named":{"value":7}},"optional":null}"#,
            r#"{"item":{"Named":{"value":7,"optional":null,"extra":0}},"optional":null}"#,
            r#"{"item":{"Named":[7,null]},"optional":null}"#,
            r#"{"item":{"Pair":[2,true,3]},"optional":null}"#,
            r#"["Unit",null]"#,
            r#"{"item":{"Unsigned":340282366920938463463374607431768211456},"optional":null}"#,
            r#"{"item":{"Signed":-170141183460469231731687303715884105729},"optional":null}"#,
            r#"{"item":{"Unsigned":18446744073709551616.0},"optional":null}"#,
            r#"{"item":{"Unsigned":-1},"optional":null}"#,
        ] {
            assert!(decode::<Record>(bytes.as_bytes()).is_err(), "{bytes}");
        }
        let deep = format!("{}0{}", "[".repeat(129), "]".repeat(129));
        assert!(decode::<serde_json::Value>(deep.as_bytes()).is_err());
    }
}
