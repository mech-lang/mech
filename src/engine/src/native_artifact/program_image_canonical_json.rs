//! Strict reader for compiler-owned canonical JSON. It borrows the existing
//! tree and checks wire structure while decoding, without serializing a copy.
use serde::{
    Deserialize, Deserializer,
    de::{
        DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess,
        Visitor, value::BorrowedStrDeserializer,
    },
};
use serde_json::{Error, Value};

pub(crate) fn decode<T: DeserializeOwned>(image: &Value) -> Result<T, String> {
    <T as Deserialize>::deserialize(Canonical(image)).map_err(|error| error.to_string())
}

fn invalid() -> Error {
    serde::de::Error::custom("build image differs from its canonical wire structure")
}

struct Canonical<'a>(&'a Value);
impl<'de> Deserializer<'de> for Canonical<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Array(_) => self.deserialize_seq(visitor),
            Value::Object(_) => self.deserialize_map(visitor),
            value => value.deserialize_any(visitor),
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.0.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.0.is_null() {
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
        visitor.visit_seq(Sequence(self.0.as_array().ok_or_else(invalid)?.iter()))
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        length: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        if self.0.as_array().is_none_or(|array| array.len() != length) {
            return Err(invalid());
        }
        self.deserialize_seq(visitor)
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
            entries: self.0.as_object().ok_or_else(invalid)?.iter(),
            value: None,
        })
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let object = self.0.as_object().ok_or_else(invalid)?;
        if object.len() != fields.len() || fields.iter().any(|field| !object.contains_key(*field)) {
            return Err(invalid());
        }
        self.deserialize_map(visitor)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.0 {
            Value::String(name) => visitor.visit_enum(Enumeration {
                name,
                payload: None,
            }),
            Value::Object(object) if object.len() == 1 => {
                let (name, payload) = object.iter().next().unwrap();
                visitor.visit_enum(Enumeration {
                    name,
                    payload: Some(payload),
                })
            }
            _ => Err(invalid()),
        }
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
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string identifier }
}

struct Sequence<'a>(core::slice::Iter<'a, Value>);
impl<'de> SeqAccess<'de> for Sequence<'de> {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        self.0
            .next()
            .map(|value| seed.deserialize(Canonical(value)))
            .transpose()
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.0.len())
    }
}
struct Object<'a> {
    entries: serde_json::map::Iter<'a>,
    value: Option<&'a Value>,
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
                seed.deserialize(BorrowedStrDeserializer::<Error>::new(key))
            })
            .transpose()
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        seed.deserialize(Canonical(self.value.take().ok_or_else(invalid)?))
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.entries.len())
    }
}
struct Enumeration<'a> {
    name: &'a str,
    payload: Option<&'a Value>,
}
impl<'de> EnumAccess<'de> for Enumeration<'de> {
    type Error = Error;
    type Variant = Self;
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self), Error> {
        let name = seed.deserialize(BorrowedStrDeserializer::<Error>::new(self.name))?;
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
        seed.deserialize(Canonical(self.payload.ok_or_else(invalid)?))
    }
    fn tuple_variant<V: Visitor<'de>>(self, length: usize, visitor: V) -> Result<V::Value, Error> {
        Canonical(self.payload.ok_or_else(invalid)?).deserialize_tuple(length, visitor)
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        Canonical(self.payload.ok_or_else(invalid)?).deserialize_struct("variant", fields, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    enum Item {
        Unit,
        One(u64),
        Pair(u8, bool),
        Named { value: u64, optional: Option<u64> },
    }
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Record {
        item: Item,
        optional: Option<u64>,
    }
    #[test]
    fn canonical_json_preserves_derived_wire_forms_and_full_width_numbers() {
        for item in [
            Item::Unit,
            Item::One(u64::MAX),
            Item::Pair(2, true),
            Item::Named {
                value: 7,
                optional: None,
            },
        ] {
            let record = Record {
                item,
                optional: None,
            };
            assert_eq!(
                decode::<Record>(&serde_json::to_value(&record).unwrap()).unwrap(),
                record
            );
        }
    }
    #[test]
    fn canonical_json_rejects_ignored_fields_missing_options_and_alternate_enum_forms() {
        for image in [
            serde_json::json!({"item": "Unit"}),
            serde_json::json!({"item": "Unit", "optional": null, "extra": 0}),
            serde_json::json!({"item": {"Unit": null}, "optional": null}),
            serde_json::json!({"item": {"Named": {"value": 7}}, "optional": null}),
            serde_json::json!({"item": {"Named": {"value": 7, "optional": null, "extra": 0}}, "optional": null}),
            serde_json::json!({"item": {"Named": [7, null]}, "optional": null}),
            serde_json::json!({"item": {"Pair": [2, true, 3]}, "optional": null}),
            serde_json::json!(["Unit", null]),
        ] {
            assert!(decode::<Record>(&image).is_err(), "{image}");
        }
    }
}
