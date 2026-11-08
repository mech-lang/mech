//! Compile canonical JSON tokens to typed metadata before application launch.
use super::*;

/// Produce portable typed data from the compiler's inspectable canonical image.
/// Integer tokens are resolved at full U128/I128 width; no JSON Value or float
/// intermediary is used. The result grants no Object or execution authority.
pub fn encode_native_metadata_image(json: &[u8]) -> Result<Vec<u8>, Error> {
    image::<true, true>(json)
}

#[cfg(test)]
pub(super) fn legacy_image(json: &[u8]) -> Result<Vec<u8>, Error> {
    image::<false, false>(json)
}

#[cfg(test)]
pub(super) fn interleaved_image(json: &[u8]) -> Result<Vec<u8>, Error> {
    image::<true, false>(json)
}

fn image<const PACKED: bool, const GROUPED: bool>(json: &[u8]) -> Result<Vec<u8>, Error> {
    if json.len() > MAX_BYTES {
        return Err(error("build JSON exceeds its size limit"));
    }
    let raw: &RawValue = serde_json::from_slice(json)?;
    let mut bytes = if GROUPED {
        NATIVE_METADATA_IMAGE_MAGIC
    } else if PACKED {
        INTERLEAVED_NATIVE_METADATA_IMAGE_MAGIC
    } else {
        LEGACY_NATIVE_METADATA_IMAGE_MAGIC
    }
    .to_vec();
    encode::<PACKED, GROUPED>(raw, &mut bytes, 0)?;
    Ok(bytes)
}

fn length(value: usize) -> Result<[u8; 4], Error> {
    Ok(u32::try_from(value)
        .map_err(|_| error("native metadata length overflows"))?
        .to_le_bytes())
}

struct Members<'a>(Vec<(String, &'a RawValue)>);
impl<'de> Deserialize<'de> for Members<'de> {
    fn deserialize<D: Deserializer<'de>>(reader: D) -> Result<Self, D::Error> {
        struct Read;
        impl<'de> Visitor<'de> for Read {
            type Value = Members<'de>;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("canonical build fields")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut fields: A) -> Result<Self::Value, A::Error> {
                let mut members = Vec::new();
                while let Some(field) = fields.next_entry()? {
                    members.push(field);
                }
                Ok(Members(members))
            }
        }
        reader.deserialize_map(Read)
    }
}

fn encode<const PACKED: bool, const GROUPED: bool>(
    raw: &RawValue,
    out: &mut Vec<u8>,
    depth: usize,
) -> Result<(), Error> {
    if depth > MAX_DEPTH {
        return Err(error("build metadata nesting exceeds its limit"));
    }
    let text = raw.get();
    let start = out.len();
    out.extend_from_slice(&[0; HEADER]);
    let tag = match text.as_bytes().first().copied() {
        Some(b'n') if text == "null" => NULL,
        Some(b'f') if text == "false" => FALSE,
        Some(b't') if text == "true" => TRUE,
        Some(b'"') => {
            let value: String = serde_json::from_str(text)?;
            out.extend_from_slice(value.as_bytes());
            TEXT
        }
        Some(b'[') => {
            let values: Vec<&RawValue> = serde_json::from_str(text)?;
            out.extend_from_slice(&length(values.len())?);
            let start = out.len();
            let mut packed = PACKED && values.len() >= 4;
            if packed {
                for value in &values {
                    let text = value.get();
                    if let Ok(byte) = text.parse::<u8>()
                        && text.as_bytes().first().is_some_and(u8::is_ascii_digit)
                    {
                        out.push(byte);
                    } else {
                        packed = false;
                        break;
                    }
                }
            }
            if packed {
                if depth == MAX_DEPTH {
                    return Err(error("build metadata nesting exceeds its limit"));
                }
                BYTE_ARRAY
            } else {
                out.truncate(start);
                for value in values {
                    encode::<PACKED, GROUPED>(value, out, depth + 1)?;
                }
                ARRAY
            }
        }
        Some(b'{') => {
            // Preserve original field order and duplicates. A malformed input
            // must not lose duplicate keys before the private reader rejects it.
            let members: Members<'_> = serde_json::from_str(text)?;
            out.extend_from_slice(&length(members.0.len())?);
            if GROUPED {
                let size_at = out.len();
                out.extend_from_slice(&[0; 4]);
                let keys_at = out.len();
                for (key, _) in &members.0 {
                    out.extend_from_slice(&length(key.len())?);
                    out.extend_from_slice(key.as_bytes());
                }
                let size = length(out.len() - keys_at)?;
                out[size_at..size_at + 4].copy_from_slice(&size);
                for (_, value) in members.0 {
                    encode::<PACKED, GROUPED>(value, out, depth + 1)?;
                }
                GROUPED_OBJECT
            } else {
                for (key, value) in members.0 {
                    out.extend_from_slice(&length(key.len())?);
                    out.extend_from_slice(key.as_bytes());
                    encode::<PACKED, GROUPED>(value, out, depth + 1)?;
                }
                OBJECT
            }
        }
        Some(b'-' | b'0'..=b'9') => {
            if text.bytes().any(|byte| matches!(byte, b'.' | b'e' | b'E')) {
                let value: f64 = text.parse().map_err(|_| error("build float is invalid"))?;
                if !value.is_finite() {
                    return Err(error("build float is not finite"));
                }
                out.extend_from_slice(&value.to_le_bytes());
                F64
            } else if text.starts_with('-') {
                if text == "-0" {
                    return Err(error(
                        "negative integer zero is not canonical compiler JSON",
                    ));
                }
                let value: i128 = text
                    .parse()
                    .map_err(|_| error("build signed integer overflows"))?;
                if let Ok(value) = i64::try_from(value) {
                    let length = if i8::try_from(value).is_ok() {
                        1
                    } else if i16::try_from(value).is_ok() {
                        2
                    } else if i32::try_from(value).is_ok() {
                        4
                    } else {
                        8
                    };
                    out.extend_from_slice(&value.to_le_bytes()[..length]);
                    I64
                } else {
                    out.extend_from_slice(&value.to_le_bytes());
                    I128
                }
            } else {
                let value: u128 = text
                    .parse()
                    .map_err(|_| error("build unsigned integer overflows"))?;
                if let Ok(value) = u64::try_from(value) {
                    let length = if value == 0 {
                        0
                    } else if value <= u8::MAX as u64 {
                        1
                    } else if value <= u16::MAX as u64 {
                        2
                    } else if value <= u32::MAX as u64 {
                        4
                    } else {
                        8
                    };
                    out.extend_from_slice(&value.to_le_bytes()[..length]);
                    U64
                } else {
                    out.extend_from_slice(&value.to_le_bytes());
                    U128
                }
            }
        }
        _ => return Err(error("unsupported canonical build JSON token")),
    };
    if out.len() > MAX_BYTES {
        return Err(error("native metadata exceeds its size limit"));
    }
    out[start] = tag;
    let size = length(out.len() - start - HEADER)?;
    out[start + 1..start + HEADER].copy_from_slice(&size);
    Ok(())
}
