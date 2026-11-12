//! Contiguous unsigned byte elements with the same scalar wire semantics.
//! These views borrow Object data and never construct execution witnesses.
use super::*;

pub(super) fn array<'de, V: Visitor<'de>>(
    payload: &'de [u8],
    depth: usize,
    canonical: bool,
    visitor: V,
    length: Option<usize>,
) -> Result<V::Value, Error> {
    let count = payload
        .get(..4)
        .ok_or_else(|| error("truncated packed metadata array"))?;
    let count = u32::from_le_bytes(count.try_into().unwrap()) as usize;
    let bytes = &payload[4..];
    if count != bytes.len() {
        return Err(error("packed metadata array count differs from its bytes"));
    }
    if length.is_some_and(|length| length != count) {
        return Err(error("native metadata tuple length differs"));
    }
    let mut fields = Elements {
        bytes,
        depth,
        canonical,
    };
    let value = visitor.visit_seq(&mut fields)?;
    if !fields.bytes.is_empty() {
        return Err(error("native metadata collection has trailing fields"));
    }
    Ok(value)
}

struct Elements<'de> {
    bytes: &'de [u8],
    depth: usize,
    canonical: bool,
}
impl<'de> SeqAccess<'de> for &mut Elements<'de> {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        let Some((byte, remaining)) = self.bytes.split_first() else {
            return Ok(None);
        };
        self.bytes = remaining;
        let mut reader = Reader(Some(Byte {
            byte,
            depth: self.depth,
            canonical: self.canonical,
        }));
        let value = seed.deserialize(&mut reader)?;
        reader.end()?;
        Ok(Some(value))
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.bytes.len())
    }
}

pub(super) struct Byte<'de> {
    pub(super) byte: &'de u8,
    pub(super) depth: usize,
    pub(super) canonical: bool,
}
impl Byte<'_> {
    fn check_depth(&self) -> Result<(), Error> {
        if self.depth > MAX_DEPTH {
            Err(error("native metadata nesting exceeds its limit"))
        } else {
            Ok(())
        }
    }
    fn wrong_kind<T>(self) -> Result<T, Error> {
        self.check_depth()?;
        Err(error("native metadata collection has a different kind"))
    }
}
impl<'de> Deserializer<'de> for Byte<'de> {
    type Error = Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.check_depth()?;
        // Native U64 nodes also deliver visit_u64, even for small values.
        // Preserve that distinction from Bool, signed and floating-point data.
        visitor.visit_u64(u64::from(*self.byte))
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.check_depth()?;
        let mut reader = Reader(Some(Self {
            depth: self.depth + 1,
            ..self
        }));
        let value = visitor.visit_some(&mut reader)?;
        reader.end()?;
        Ok(value)
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.check_depth()?;
        if name == RAW_RECORD {
            return visitor.visit_borrowed_bytes(std::slice::from_ref(self.byte));
        }
        let mut reader = Reader(Some(Self {
            depth: self.depth + 1,
            ..self
        }));
        let value = visitor.visit_newtype_struct(&mut reader)?;
        reader.end()?;
        Ok(value)
    }
    fn deserialize_unit<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
        self.check_depth()?;
        Err(error("native metadata unit differs"))
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.deserialize_unit(visitor)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
        self.wrong_kind()
    }
    fn deserialize_tuple<V: Visitor<'de>>(self, _: usize, _: V) -> Result<V::Value, Error> {
        self.wrong_kind()
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: usize,
        _: V,
    ) -> Result<V::Value, Error> {
        self.wrong_kind()
    }
    fn deserialize_map<V: Visitor<'de>>(self, _: V) -> Result<V::Value, Error> {
        self.wrong_kind()
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Error> {
        self.wrong_kind()
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Error> {
        self.wrong_kind()
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.canonical {
            return Err(error("canonical native metadata has an ignored field"));
        }
        self.check_depth()?;
        visitor.visit_unit()
    }
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64
        char str string bytes byte_buf identifier
    }
}

struct Reader<'de>(Option<Byte<'de>>);
impl Reader<'_> {
    fn end(&self) -> Result<(), Error> {
        if self.0.is_some() {
            Err(error("unconsumed native metadata record"))
        } else {
            Ok(())
        }
    }
}
macro_rules! forward {
    ($($method:ident($($arg:ident: $ty:ty),*);)*) => {$(
        fn $method<V: Visitor<'de>>(self, $($arg: $ty,)* visitor: V) -> Result<V::Value, Error> {
            self.0.take().ok_or_else(|| error("native metadata record already consumed"))?
                .$method($($arg,)* visitor)
        }
    )*};
}
impl<'de> Deserializer<'de> for &mut Reader<'de> {
    type Error = Error;
    forward! {
        deserialize_any(); deserialize_bool(); deserialize_i8(); deserialize_i16(); deserialize_i32(); deserialize_i64(); deserialize_i128();
        deserialize_u8(); deserialize_u16(); deserialize_u32(); deserialize_u64(); deserialize_u128(); deserialize_f32(); deserialize_f64();
        deserialize_char(); deserialize_str(); deserialize_string(); deserialize_bytes(); deserialize_byte_buf(); deserialize_option(); deserialize_unit();
        deserialize_unit_struct(name: &'static str); deserialize_newtype_struct(name: &'static str);
        deserialize_seq(); deserialize_tuple(length: usize);
        deserialize_tuple_struct(name: &'static str, length: usize); deserialize_map();
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]); deserialize_identifier(); deserialize_ignored_any();
    }
}
