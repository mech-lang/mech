//! JSON-compatible map-key vocabulary without invoking a JSON token parser.
use super::*;
pub(super) struct Key<'a>(pub &'a str);

fn number(text: &str) -> Result<(), Error> {
    let bytes = text.as_bytes();
    let mut at = usize::from(bytes.first() == Some(&b'-'));
    match bytes.get(at) {
        Some(b'0') => at += 1,
        Some(b'1'..=b'9') => {
            at += 1;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
        }
        _ => return Err(error("native metadata map key is not a number")),
    }
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return Err(error("native metadata map key fraction is empty"));
        }
    }
    if bytes
        .get(at)
        .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
        at += 1;
        if bytes
            .get(at)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            at += 1;
        }
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return Err(error("native metadata map key exponent is empty"));
        }
    }
    if at != bytes.len() {
        return Err(error("native metadata map key has trailing text"));
    }
    Ok(())
}

macro_rules! integer {
    ($($method:ident => $visit:ident : $ty:ty),* $(,)?) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
            number(self.0)?;
            visitor.$visit(self.0.parse::<$ty>().map_err(|_| error("native metadata map key is outside its integer type"))?)
        }
    )*};
}
impl<'de> Deserializer<'de> for Key<'de> {
    type Error = Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_borrowed_str(self.0)
    }
    integer! {
        deserialize_i8 => visit_i8: i8, deserialize_i16 => visit_i16: i16, deserialize_i32 => visit_i32: i32,
        deserialize_i64 => visit_i64: i64, deserialize_i128 => visit_i128: i128,
        deserialize_u8 => visit_u8: u8, deserialize_u16 => visit_u16: u16, deserialize_u32 => visit_u32: u32,
        deserialize_u64 => visit_u64: u64, deserialize_u128 => visit_u128: u128,
    }
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_bool(
            self.0
                .parse()
                .map_err(|_| error("native metadata map key is not Bool"))?,
        )
    }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        number(self.0)?;
        let value: f32 = self
            .0
            .parse()
            .map_err(|_| error("invalid native metadata float key"))?;
        if !value.is_finite() {
            return Err(error("native metadata float key is not finite"));
        }
        visitor.visit_f32(value)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        number(self.0)?;
        let value: f64 = self
            .0
            .parse()
            .map_err(|_| error("invalid native metadata float key"))?;
        if !value.is_finite() {
            return Err(error("native metadata float key is not finite"));
        }
        visitor.visit_f64(value)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        visitor.visit_newtype_struct(self)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        de::value::BorrowedStrDeserializer::<Error>::new(self.0)
            .deserialize_enum(name, variants, visitor)
    }
    serde::forward_to_deserialize_any! {
        char str string bytes byte_buf option unit unit_struct seq tuple tuple_struct map struct identifier ignored_any
    }
}
