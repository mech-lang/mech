//! Generic encoded-payload transport, independent of source syntax trees.

use crate::*;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

pub fn compress_and_encode<T: serde::Serialize>(
    tree: &T,
) -> Result<String, Box<dyn std::error::Error>> {
    let serialized_code = bincode::serde::encode_to_vec(tree, bincode::config::standard())?;
    let mut compressed = Vec::new();
    brotli::CompressorWriter::new(&mut compressed, 9, 4096, 22).write(&serialized_code)?;
    Ok(BASE64_STANDARD.encode(compressed))
}

pub fn decode_and_decompress<T: serde::de::DeserializeOwned>(
    encoded: &str,
) -> Result<T, Box<dyn std::error::Error>> {
    let decoded = BASE64_STANDARD.decode(encoded)?;

    let mut decompressed = Vec::new();
    brotli::Decompressor::new(Cursor::new(decoded), 4096).read_to_end(&mut decompressed)?;

    let (decoded, _) =
        bincode::serde::decode_from_slice(&decompressed, bincode::config::standard())?;

    Ok(decoded)
}
