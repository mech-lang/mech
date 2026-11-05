//! Exact descriptor-byte integrity relative to one trusted checked native Object.
//! A digest supplied by a caller cannot authenticate arbitrary native code.
use super::identity::{NATIVE_PROGRAM_LINK_IDENTITY_BYTES, NativeProgramObjectIdentity};
use sha2::{Digest, Sha256};

pub const NATIVE_PROGRAM_IMAGE_BINDING_BYTES: usize = 64;
const MAGIC: &[u8; 16] = b"MECHIMAGEPIN\0\0\0\0";
const DOMAIN: &[u8] = b"mech.checked-object.descriptor.v1\0";

/// Build output emitted as a read-only symbol beside the exact code identity.
/// Construction is private to the checked compiler; bytes alone grant no entry.
pub struct NativeProgramImageBinding {
    bytes: [u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES],
    symbol: String,
}
pub(in crate::native_artifact) fn symbol(identity_symbol: &str) -> String {
    format!("{identity_symbol}_descriptor_binding_v1")
}
fn digest(image: &[u8], identity: &[u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(DOMAIN);
    hash.update((image.len() as u64).to_le_bytes());
    hash.update(identity);
    hash.update(image);
    hash.finalize().into()
}
impl NativeProgramImageBinding {
    #[cfg(feature = "canonical-native-program")]
    pub(in crate::native_artifact) fn new(
        image: &[u8],
        identity: &NativeProgramObjectIdentity,
    ) -> Self {
        let mut bytes = [0; NATIVE_PROGRAM_IMAGE_BINDING_BYTES];
        bytes[..16].copy_from_slice(MAGIC);
        bytes[16..24].copy_from_slice(&(image.len() as u64).to_le_bytes());
        bytes[24..32].copy_from_slice(&1u64.to_le_bytes());
        bytes[32..].copy_from_slice(&digest(image, identity.bytes()));
        Self {
            bytes,
            symbol: symbol(identity.symbol()),
        }
    }
    pub fn bytes(&self) -> &[u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES] {
        &self.bytes
    }
    pub fn symbol(&self) -> &str {
        &self.symbol
    }
}

/// A synchronous immutable borrow of exactly the bytes checked below. Its
/// private constructor prevents a wire flag from selecting the bound loader.
pub(super) struct VerifiedObjectImage<'a> {
    image: &'a [u8],
}
impl VerifiedObjectImage<'_> {
    pub(super) fn bytes(&self) -> &[u8] {
        self.image
    }
}

/// # Safety
/// `binding` and `identity` must be genuine read-only symbols from the SAME
/// trusted checked compiler Object as the entry to be bound. Arbitrary matching
/// bytes/digests do not establish that provenance or authenticate foreign code.
/// The returned borrow cannot be paired with different image bytes.
pub(super) unsafe fn verify<'a>(
    image: &'a [u8],
    identity: &[u8; NATIVE_PROGRAM_LINK_IDENTITY_BYTES],
    binding: &[u8; NATIVE_PROGRAM_IMAGE_BINDING_BYTES],
) -> Result<VerifiedObjectImage<'a>, String> {
    if &binding[..16] != MAGIC
        || binding[16..24] != (image.len() as u64).to_le_bytes()
        || binding[24..32] != 1u64.to_le_bytes()
        || binding[32..] != digest(image, identity)
    {
        return Err("descriptor bytes differ from the checked Object binding".into());
    }
    Ok(VerifiedObjectImage { image })
}

#[cfg(all(test, feature = "canonical-native-program"))]
#[path = "binding_tests.rs"]
mod tests;
