//! Typed, length-delimited build metadata. This is data, never code authority.
//! The compiler resolves JSON tokens once; linked applications borrow the
//! resulting read-only Object data. Existing private wire constructors still
//! check schemas/factories and retain their unchanged-build confinement duties.
use serde::{
    Deserialize, Deserializer,
    de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor},
};
use serde_json::{Error, value::RawValue};

#[path = "build_image/encode.rs"]
mod encode;
#[path = "build_image/key.rs"]
mod key;
#[path = "build_image/packed.rs"]
mod packed;
pub use encode::encode_native_metadata_image;

pub const NATIVE_METADATA_IMAGE_MAGIC: &[u8; 8] = b"MECHTVI3";
const LEGACY_NATIVE_METADATA_IMAGE_MAGIC: &[u8; 8] = b"MECHTVI1";
pub(crate) const RAW_RECORD: &str = "mech.build-owned-image.record.v1";
const HEADER: usize = 5;
const MAX_DEPTH: usize = 128;
const MAX_BYTES: usize = 64 * 1024 * 1024;
const NULL: u8 = 0;
const FALSE: u8 = 1;
const TRUE: u8 = 2;
const U64: u8 = 3;
const I64: u8 = 4;
const U128: u8 = 5;
const I128: u8 = 6;
const F64: u8 = 7;
const TEXT: u8 = 8;
const ARRAY: u8 = 9;
const OBJECT: u8 = 10;
const BYTE_ARRAY: u8 = 11;

fn error(message: &str) -> Error {
    <Error as de::Error>::custom(message)
}

/// Borrowed, unexpanded metadata record. No variant grants execution or
/// semantic-proof authority. Native records are length delimited, not pointers
/// into Rust object layouts, and never carry invocation-local state.
#[derive(Clone, Copy, Debug)]
pub enum BuildOwnedImageRecord<'a> {
    Json(&'a str),
    Native(&'a [u8]),
    /// One unsigned numeric element borrowed from a packed native array. The
    /// parent supplies its element boundary; no synthetic node is allocated.
    UnsignedByte(&'a u8),
}
impl<'a> BuildOwnedImageRecord<'a> {
    pub fn reader(self) -> BuildOwnedImageReader<'a> {
        match self {
            Self::Json(text) => BuildOwnedImageReader {
                source: Source::Json(serde_json::Deserializer::from_str(text)),
                depth: 0,
                canonical: false,
            },
            Self::Native(bytes) => BuildOwnedImageReader::native(bytes, 0),
            Self::UnsignedByte(byte) => BuildOwnedImageReader {
                source: Source::UnsignedByte(Some(byte)),
                depth: 0,
                canonical: false,
            },
        }
    }
    pub fn decode<T: Deserialize<'a>>(self) -> Result<T, Error> {
        let mut reader = self.reader();
        let value = T::deserialize(&mut reader)?;
        reader.end()?;
        Ok(value)
    }
}
impl<'de: 'a, 'a> Deserialize<'de> for BuildOwnedImageRecord<'a> {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        struct Record<'a>(std::marker::PhantomData<&'a ()>);
        impl<'de: 'a, 'a> Visitor<'de> for Record<'a> {
            type Value = BuildOwnedImageRecord<'a>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a borrowed build-image record")
            }
            fn visit_borrowed_str<E: de::Error>(self, text: &'de str) -> Result<Self::Value, E> {
                Ok(BuildOwnedImageRecord::Json(text))
            }
            fn visit_borrowed_bytes<E: de::Error>(
                self,
                bytes: &'de [u8],
            ) -> Result<Self::Value, E> {
                // A complete native node has at least HEADER bytes. Only the
                // packed-element reader supplies this one-byte raw span.
                Ok(match bytes {
                    [byte] => BuildOwnedImageRecord::UnsignedByte(byte),
                    _ => BuildOwnedImageRecord::Native(bytes),
                })
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                reader: D,
            ) -> Result<Self::Value, D::Error> {
                <&RawValue>::deserialize(reader).map(|raw| BuildOwnedImageRecord::Json(raw.get()))
            }
        }
        reader.deserialize_newtype_struct(RAW_RECORD, Record(std::marker::PhantomData))
    }
}

enum Source<'a> {
    Json(serde_json::Deserializer<serde_json::de::StrRead<'a>>),
    Native(Option<&'a [u8]>),
    UnsignedByte(Option<&'a u8>),
}
/// A reader for the same private wire vocabulary in inspectable JSON or
/// compiler-emitted typed Object data. It owns no metadata buffer or live cache.
pub struct BuildOwnedImageReader<'a> {
    source: Source<'a>,
    depth: usize,
    canonical: bool,
}
impl<'a> BuildOwnedImageReader<'a> {
    /// Require every declared struct/variant field, including explicit null
    /// options, as the existing canonical constant reader does. This does not
    /// validate a semantic schema or construct a native execution proof.
    pub(crate) fn canonical(mut self) -> Self {
        self.canonical = true;
        self
    }
    pub fn from_image(bytes: &'a [u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BYTES {
            return Err(error("build metadata exceeds its size limit"));
        }
        if let Some(bytes) = bytes
            .strip_prefix(NATIVE_METADATA_IMAGE_MAGIC)
            .or_else(|| bytes.strip_prefix(LEGACY_NATIVE_METADATA_IMAGE_MAGIC))
        {
            Ok(Self::native(bytes, 0))
        } else {
            let text = std::str::from_utf8(bytes).map_err(|_| error("build JSON is not UTF8"))?;
            Ok(BuildOwnedImageRecord::Json(text).reader())
        }
    }
    fn native(bytes: &'a [u8], depth: usize) -> Self {
        Self {
            source: Source::Native(Some(bytes)),
            depth,
            canonical: false,
        }
    }
    pub fn end(&mut self) -> Result<(), Error> {
        match &mut self.source {
            Source::Json(reader) => reader.end(),
            Source::Native(None) => Ok(()),
            Source::Native(Some(_)) => Err(error("unconsumed native metadata record")),
            Source::UnsignedByte(None) => Ok(()),
            Source::UnsignedByte(Some(_)) => Err(error("unconsumed native metadata record")),
        }
    }
}

struct Node<'a> {
    bytes: &'a [u8],
    depth: usize,
    canonical: bool,
}
impl<'a> Node<'a> {
    fn decode<T: DeserializeSeed<'a>>(self, seed: T) -> Result<T::Value, Error> {
        let mut reader = NativeReader(Some(self));
        let value = seed.deserialize(&mut reader)?;
        reader.end()?;
        Ok(value)
    }
    fn parts(&self) -> Result<(u8, &'a [u8]), Error> {
        if self.depth > MAX_DEPTH {
            return Err(error("native metadata nesting exceeds its limit"));
        }
        if self.bytes.len() < HEADER {
            return Err(error("truncated native metadata node"));
        }
        let tag = self.bytes[0];
        let length = u32::from_le_bytes(self.bytes[1..HEADER].try_into().unwrap()) as usize;
        if tag > BYTE_ARRAY || length != self.bytes.len() - HEADER {
            return Err(error("native metadata node length or kind differs"));
        }
        Ok((tag, &self.bytes[HEADER..]))
    }
    fn fields(&self, expected: u8) -> Result<Fields<'a>, Error> {
        let (tag, payload) = self.parts()?;
        if tag != expected {
            return Err(error("native metadata collection has a different kind"));
        }
        self.members(tag, payload)
    }
    fn members(&self, tag: u8, payload: &'a [u8]) -> Result<Fields<'a>, Error> {
        if payload.len() < 4 {
            return Err(error("native metadata collection has a different kind"));
        }
        let count = u32::from_le_bytes(payload[..4].try_into().unwrap()) as usize;
        let bytes = &payload[4..];
        let minimum = if tag == OBJECT { HEADER + 4 } else { HEADER };
        if count > bytes.len() / minimum {
            return Err(error("native metadata item count exceeds its bytes"));
        }
        Ok(Fields {
            bytes,
            remaining: count,
            pending_key: false,
            depth: self.depth + 1,
            canonical: self.canonical,
            known_keys: None,
        })
    }
    fn array<V: Visitor<'a>>(self, visitor: V, length: Option<usize>) -> Result<V::Value, Error> {
        let (tag, payload) = self.parts()?;
        if tag == BYTE_ARRAY {
            return packed::array(payload, self.depth + 1, self.canonical, visitor, length);
        }
        if tag != ARRAY {
            return Err(error("native metadata collection has a different kind"));
        }
        let mut fields = self.members(ARRAY, payload)?;
        if length.is_some_and(|length| length != fields.remaining) {
            return Err(error("native metadata tuple length differs"));
        }
        let value = visitor.visit_seq(&mut fields)?;
        fields.end()?;
        Ok(value)
    }
    fn object<V: Visitor<'a>>(self, visitor: V) -> Result<V::Value, Error> {
        let mut fields = self.fields(OBJECT)?;
        let value = visitor.visit_map(&mut fields)?;
        fields.end()?;
        Ok(value)
    }
}

// Child records are already native. Keep their consumption state without
// carrying a JSON deserializer or branching over the source format per field.
// A visitor must still consume its child, even when it otherwise returns Ok.
struct NativeReader<'a>(Option<Node<'a>>);
impl NativeReader<'_> {
    fn end(&self) -> Result<(), Error> {
        if self.0.is_some() {
            Err(error("unconsumed native metadata record"))
        } else {
            Ok(())
        }
    }
}

struct Fields<'a> {
    bytes: &'a [u8],
    remaining: usize,
    pending_key: bool,
    depth: usize,
    canonical: bool,
    known_keys: Option<StructKeys>,
}

enum StructKeys {
    Ordered(&'static [&'static str]),
    // Noncanonical struct visitors retain their existing policy for missing,
    // duplicate and unknown fields. Match known names without a probe pass;
    // unknown names still require ordinary UTF8 validation and visitor handling.
    Declared(&'static [&'static str]),
    // Canonical struct names come from the deserializing type, never from an
    // image flag. A bounded set checks presence and duplicates as we consume
    // each field, without reparsing the entire record for every declared name.
    Canonical {
        names: &'static [&'static str],
        seen: u64,
    },
}
impl<'a> Fields<'a> {
    fn end(&self) -> Result<(), Error> {
        if self.remaining != 0 || self.pending_key || !self.bytes.is_empty() {
            Err(error("native metadata collection has trailing fields"))
        } else if let Some(StructKeys::Canonical { names, seen }) = &self.known_keys
            && seen.count_ones() as usize != names.len()
        {
            Err(error("canonical native metadata fields differ"))
        } else {
            Ok(())
        }
    }
    fn node(&mut self) -> Result<&'a [u8], Error> {
        if self.bytes.len() < HEADER {
            return Err(error("truncated native metadata child"));
        }
        let length = u32::from_le_bytes(self.bytes[1..HEADER].try_into().unwrap()) as usize;
        let end = HEADER
            .checked_add(length)
            .ok_or_else(|| error("native metadata child length overflows"))?;
        let value = self
            .bytes
            .get(..end)
            .ok_or_else(|| error("native metadata child exceeds its parent"))?;
        self.bytes = &self.bytes[end..];
        Ok(value)
    }
    fn key_bytes(&mut self) -> Result<&'a [u8], Error> {
        let length = self
            .bytes
            .get(..4)
            .ok_or_else(|| error("truncated native metadata key"))?;
        let length = u32::from_le_bytes(length.try_into().unwrap()) as usize;
        let end = 4usize
            .checked_add(length)
            .ok_or_else(|| error("native metadata key length overflows"))?;
        let text = self
            .bytes
            .get(4..end)
            .ok_or_else(|| error("native metadata key exceeds its parent"))?;
        self.bytes = &self.bytes[end..];
        Ok(text)
    }
    fn key(&mut self) -> Result<&'a str, Error> {
        std::str::from_utf8(self.key_bytes()?).map_err(|_| error("native metadata key is not UTF8"))
    }
    fn decode<T: DeserializeSeed<'a>>(&mut self, seed: T) -> Result<T::Value, Error> {
        Node {
            bytes: self.node()?,
            depth: self.depth,
            canonical: self.canonical,
        }
        .decode(seed)
    }
}
impl<'de> SeqAccess<'de> for &mut Fields<'de> {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        if self.remaining == 0 {
            return Ok(None);
        }
        self.remaining -= 1;
        self.decode(seed).map(Some)
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}
impl<'de> MapAccess<'de> for &mut Fields<'de> {
    type Error = Error;
    fn next_key_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        if self.pending_key {
            return Err(error("native metadata key has no value"));
        }
        if self.remaining == 0 {
            return Ok(None);
        }
        let key = if self.known_keys.is_some() {
            let bytes = self.key_bytes()?;
            match self.known_keys.as_mut().unwrap() {
                StructKeys::Ordered(names) => {
                    let expected = names[names.len() - self.remaining];
                    if bytes != expected.as_bytes() {
                        return Err(error("native metadata field order differs"));
                    }
                    expected
                }
                StructKeys::Declared(names) => {
                    let ordered = names
                        .len()
                        .checked_sub(self.remaining)
                        .and_then(|at| names.get(at))
                        .copied()
                        .filter(|name| bytes == name.as_bytes());
                    if let Some(name) = ordered
                        .or_else(|| names.iter().copied().find(|name| bytes == name.as_bytes()))
                    {
                        name
                    } else {
                        std::str::from_utf8(bytes)
                            .map_err(|_| error("native metadata key is not UTF8"))?
                    }
                }
                StructKeys::Canonical { names, seen } => {
                    let ordered = names.len() - self.remaining;
                    let position = if bytes == names[ordered].as_bytes() {
                        ordered
                    } else {
                        names
                            .iter()
                            .position(|name| bytes == name.as_bytes())
                            .ok_or_else(|| error("canonical native metadata fields differ"))?
                    };
                    let bit = 1u64 << position;
                    if *seen & bit != 0 {
                        return Err(error("canonical native metadata fields differ"));
                    }
                    *seen |= bit;
                    names[position]
                }
            }
            // Known names prove UTF8 by exact static-name equality. Unknown
            // noncanonical names receive ordinary UTF8 validation above.
        } else {
            self.key()?
        };
        self.pending_key = true;
        seed.deserialize(key::Key(key)).map(Some)
    }
    fn next_value_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<T::Value, Error> {
        if !self.pending_key {
            return Err(error("native metadata value has no key"));
        }
        self.pending_key = false;
        self.remaining -= 1;
        self.decode(seed)
    }
    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining)
    }
}

macro_rules! fixed {
    ($payload:expr, $ty:ty) => {
        <$ty>::from_le_bytes(
            $payload
                .try_into()
                .map_err(|_| error("native metadata scalar width differs"))?,
        )
    };
}
impl<'de> Deserializer<'de> for Node<'de> {
    type Error = Error;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let (tag, bytes) = self.parts()?;
        match tag {
            NULL | FALSE | TRUE if !bytes.is_empty() => {
                Err(error("native metadata unit payload differs"))
            }
            NULL => visitor.visit_unit(),
            FALSE => visitor.visit_bool(false),
            TRUE => visitor.visit_bool(true),
            U64 => {
                if !matches!(bytes.len(), 0 | 1 | 2 | 4 | 8) {
                    return Err(error("native metadata unsigned width differs"));
                }
                let mut value = [0u8; 8];
                value[..bytes.len()].copy_from_slice(bytes);
                visitor.visit_u64(u64::from_le_bytes(value))
            }
            I64 => {
                if !matches!(bytes.len(), 1 | 2 | 4 | 8) {
                    return Err(error("native metadata signed width differs"));
                }
                let mut value = [if bytes.last().unwrap() & 128 != 0 {
                    255
                } else {
                    0
                }; 8];
                value[..bytes.len()].copy_from_slice(bytes);
                visitor.visit_i64(i64::from_le_bytes(value))
            }
            U128 => visitor.visit_u128(fixed!(bytes, u128)),
            I128 => visitor.visit_i128(fixed!(bytes, i128)),
            F64 => {
                let value = fixed!(bytes, f64);
                if !value.is_finite() {
                    return Err(error("native metadata float is not finite"));
                }
                visitor.visit_f64(value)
            }
            TEXT => visitor.visit_borrowed_str(
                std::str::from_utf8(bytes)
                    .map_err(|_| error("native metadata string is not UTF8"))?,
            ),
            ARRAY | BYTE_ARRAY => self.array(visitor, None),
            OBJECT => self.object(visitor),
            _ => Err(error("unknown native metadata kind")),
        }
    }
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let (tag, payload) = self.parts()?;
        if tag == NULL {
            if !payload.is_empty() {
                return Err(error("native metadata option payload differs"));
            }
            visitor.visit_none()
        } else {
            let mut reader = NativeReader(Some(Node {
                depth: self.depth + 1,
                ..self
            }));
            let value = visitor.visit_some(&mut reader)?;
            reader.end()?;
            Ok(value)
        }
    }
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        let (tag, bytes) = self.parts()?;
        if tag != NULL || !bytes.is_empty() {
            return Err(error("native metadata unit differs"));
        }
        visitor.visit_unit()
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
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        if name == RAW_RECORD {
            self.parts()?;
            return visitor.visit_borrowed_bytes(self.bytes);
        }
        let mut reader = NativeReader(Some(Node {
            depth: self.depth + 1,
            ..self
        }));
        let value = visitor.visit_newtype_struct(&mut reader)?;
        reader.end()?;
        Ok(value)
    }
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.array(visitor, None)
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        length: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.array(visitor, Some(length))
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        length: usize,
        visitor: V,
    ) -> Result<V::Value, Error> {
        self.array(visitor, Some(length))
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        self.object(visitor)
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        expected: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let mut fields = self.fields(OBJECT)?;
        if self.canonical {
            if fields.remaining != expected.len() {
                return Err(error("canonical native metadata field count differs"));
            }
            if expected.len() <= u64::BITS as usize {
                fields.known_keys = Some(StructKeys::Canonical {
                    names: expected,
                    seen: 0,
                });
                let value = visitor.visit_map(&mut fields)?;
                fields.end()?;
                return Ok(value);
            }
        } else {
            fields.known_keys = Some(StructKeys::Declared(expected));
            let value = visitor.visit_map(&mut fields)?;
            fields.end()?;
            return Ok(value);
        }
        if fields.remaining == expected.len() {
            let mut probe = self.fields(OBJECT)?;
            let mut ordered = true;
            for name in expected {
                if probe.key_bytes()? != name.as_bytes() {
                    ordered = false;
                    break;
                }
                probe.node()?;
                probe.remaining -= 1;
            }
            if ordered {
                probe.end()?;
                fields.known_keys = Some(StructKeys::Ordered(expected));
                let value = visitor.visit_map(&mut fields)?;
                fields.end()?;
                return Ok(value);
            }
        }
        if self.canonical {
            if fields.remaining != expected.len() {
                return Err(error("canonical native metadata field count differs"));
            }
            for expected in expected {
                let mut fields = self.fields(OBJECT)?;
                let mut occurrences = 0;
                while fields.remaining != 0 {
                    if fields.key_bytes()? == expected.as_bytes() {
                        occurrences += 1;
                    }
                    fields.node()?;
                    fields.remaining -= 1;
                }
                fields.end()?;
                if occurrences != 1 {
                    return Err(error("canonical native metadata fields differ"));
                }
            }
        }
        self.object(visitor)
    }
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        let (tag, bytes) = self.parts()?;
        let (name, payload) = if tag == TEXT {
            (
                std::str::from_utf8(bytes)
                    .map_err(|_| error("native metadata variant is not UTF8"))?,
                None,
            )
        } else {
            let mut fields = self.fields(OBJECT)?;
            if fields.remaining != 1 {
                return Err(error("native metadata enum has more than one tag"));
            }
            let name = fields.key()?;
            let payload = fields.node()?;
            fields.remaining = 0;
            fields.end()?;
            (name, Some(payload))
        };
        visitor.visit_enum(Enumeration {
            name,
            payload,
            depth: self.depth + 1,
            canonical: self.canonical,
        })
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        if self.canonical {
            return Err(error("canonical native metadata has an ignored field"));
        }
        self.parts()?;
        visitor.visit_unit()
    }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf identifier }
}

#[cfg(test)]
#[path = "build_image/tests.rs"]
mod tests;
struct Enumeration<'a> {
    name: &'a str,
    payload: Option<&'a [u8]>,
    depth: usize,
    canonical: bool,
}
impl<'de> EnumAccess<'de> for Enumeration<'de> {
    type Error = Error;
    type Variant = Self;
    fn variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<(T::Value, Self), Error> {
        Ok((
            seed.deserialize(de::value::BorrowedStrDeserializer::<Error>::new(self.name))?,
            self,
        ))
    }
}
impl<'de> VariantAccess<'de> for Enumeration<'de> {
    type Error = Error;
    fn unit_variant(self) -> Result<(), Error> {
        if self.payload.is_none() {
            Ok(())
        } else {
            Err(error("native metadata unit variant has a payload"))
        }
    }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        Node {
            bytes: self
                .payload
                .ok_or_else(|| error("native metadata variant has no payload"))?,
            depth: self.depth,
            canonical: self.canonical,
        }
        .decode(seed)
    }
    fn tuple_variant<V: Visitor<'de>>(self, length: usize, visitor: V) -> Result<V::Value, Error> {
        Node {
            bytes: self
                .payload
                .ok_or_else(|| error("native metadata variant has no tuple"))?,
            depth: self.depth,
            canonical: self.canonical,
        }
        .array(visitor, Some(length))
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        Node {
            bytes: self
                .payload
                .ok_or_else(|| error("native metadata variant has no fields"))?,
            depth: self.depth,
            canonical: self.canonical,
        }
        .deserialize_struct("variant", fields, visitor)
    }
}

macro_rules! native_forward {
    ($($method:ident($($arg:ident: $ty:ty),*);)*) => {$ (
        fn $method<V: Visitor<'de>>(self, $($arg: $ty,)* visitor: V) -> Result<V::Value, Error> {
            self.0.take().ok_or_else(|| error("native metadata record already consumed"))?
                .$method($($arg,)* visitor)
        }
    )*};
}
impl<'de> Deserializer<'de> for &mut NativeReader<'de> {
    type Error = Error;
    native_forward! {
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

macro_rules! forward {
    ($($method:ident($($arg:ident: $ty:ty),*);)*) => {$ (
        fn $method<V: Visitor<'de>>(self, $($arg: $ty,)* visitor: V) -> Result<V::Value, Error> {
            match &mut self.source {
                Source::Json(reader) => reader.$method($($arg,)* visitor),
                Source::Native(bytes) => Node { bytes: bytes.take().ok_or_else(|| error("native metadata record already consumed"))?, depth: self.depth, canonical: self.canonical }.$method($($arg,)* visitor),
                Source::UnsignedByte(byte) => packed::Byte { byte: byte.take().ok_or_else(|| error("native metadata record already consumed"))?, depth: self.depth, canonical: self.canonical }.$method($($arg,)* visitor),
            }
        }
    )*};
}
impl<'de> Deserializer<'de> for &mut BuildOwnedImageReader<'de> {
    type Error = Error;
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        if name == RAW_RECORD {
            match &mut self.source {
                Source::Json(reader) => {
                    visitor.visit_borrowed_str(<&RawValue>::deserialize(reader)?.get())
                }
                Source::Native(bytes) => {
                    let bytes = bytes
                        .take()
                        .ok_or_else(|| error("native metadata record already consumed"))?;
                    Node {
                        bytes,
                        depth: self.depth,
                        canonical: self.canonical,
                    }
                    .parts()?;
                    visitor.visit_borrowed_bytes(bytes)
                }
                Source::UnsignedByte(byte) => packed::Byte {
                    byte: byte
                        .take()
                        .ok_or_else(|| error("native metadata record already consumed"))?,
                    depth: self.depth,
                    canonical: self.canonical,
                }
                .deserialize_newtype_struct(name, visitor),
            }
        } else {
            match &mut self.source {
                Source::Json(reader) => reader.deserialize_newtype_struct(name, visitor),
                Source::Native(bytes) => Node {
                    bytes: bytes
                        .take()
                        .ok_or_else(|| error("native metadata record already consumed"))?,
                    depth: self.depth,
                    canonical: self.canonical,
                }
                .deserialize_newtype_struct(name, visitor),
                Source::UnsignedByte(byte) => packed::Byte {
                    byte: byte
                        .take()
                        .ok_or_else(|| error("native metadata record already consumed"))?,
                    depth: self.depth,
                    canonical: self.canonical,
                }
                .deserialize_newtype_struct(name, visitor),
            }
        }
    }
    forward! {
        deserialize_any(); deserialize_bool(); deserialize_i8(); deserialize_i16(); deserialize_i32(); deserialize_i64(); deserialize_i128();
        deserialize_u8(); deserialize_u16(); deserialize_u32(); deserialize_u64(); deserialize_u128(); deserialize_f32(); deserialize_f64();
        deserialize_char(); deserialize_str(); deserialize_string(); deserialize_bytes(); deserialize_byte_buf(); deserialize_option(); deserialize_unit();
        deserialize_unit_struct(name: &'static str); deserialize_seq(); deserialize_tuple(length: usize);
        deserialize_tuple_struct(name: &'static str, length: usize); deserialize_map();
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]); deserialize_identifier(); deserialize_ignored_any();
    }
}
