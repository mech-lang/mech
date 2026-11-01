//! Size-only envelope for immutable compiler metadata, never program authority.
//! JSON remains a pinned build product for inspection. Only this packed copy is
//! embedded in the executable; unpacking restores exactly the same descriptor.
use miniz_oxide::inflate::{
    TINFLStatus,
    core::{DecompressorOxide, decompress, inflate_flags},
};

pub(super) const MAGIC: &[u8; 8] = b"MECHNDI\x01";
const HEADER_BYTES: usize = 24;
// Bound allocation before inspecting compressed contents, independently of any
// claimed compression ratio. The compiler rejects images beyond the same limit.
const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;

#[cfg(feature = "canonical-native-program")]
pub(super) fn pack(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return Err("native descriptor image exceeds the packing limit".into());
    }
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(bytes, 9);
    let mut image = Vec::with_capacity(HEADER_BYTES + compressed.len());
    image.extend_from_slice(MAGIC);
    image.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    image.extend_from_slice(&(compressed.len() as u64).to_le_bytes());
    image.extend_from_slice(&compressed);
    Ok(image)
}

pub(super) fn unpack(image: &[u8]) -> Result<Vec<u8>, String> {
    if image.len() < HEADER_BYTES || !image.starts_with(MAGIC) {
        return Err("invalid packed native descriptor header".into());
    }
    let length = |start| {
        usize::try_from(u64::from_le_bytes(
            image[start..start + 8].try_into().unwrap(),
        ))
        .map_err(|_| "packed native descriptor length overflows".to_owned())
    };
    let expanded = length(8)?;
    let compressed = length(16)?;
    if expanded == 0 || expanded > MAX_IMAGE_BYTES || compressed != image.len() - HEADER_BYTES {
        return Err("packed native descriptor length differs or exceeds its limit".into());
    }
    // One exact allocation, not geometric buffer growth. The decoder checks the
    // zlib checksum, consumes the complete payload, and must fill this exact size.
    let mut bytes = vec![0; expanded];
    let mut decoder = Box::<DecompressorOxide>::default();
    let flags = inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER
        | inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
    let (status, consumed, written) =
        decompress(&mut decoder, &image[HEADER_BYTES..], &mut bytes, 0, flags);
    if status != TINFLStatus::Done || consumed != compressed || written != expanded {
        return Err("packed native descriptor payload is corrupt or has a different size".into());
    }
    Ok(bytes)
}
