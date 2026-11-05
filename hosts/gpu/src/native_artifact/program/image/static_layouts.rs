//! Immutable compiler results restored only inside the confined image loader.
//! Actual linked factories, owner admission and current visits remain checked.
use crate::native_artifact::program::prepare::{
    ProgramMetadata, call_cache, call_routes::LibraryCallRoute, cleanup_effects, composition,
    frame_retirement,
};
use mech_engine::native_artifact::NativeUnsignedBufferCall;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize)]
pub(super) struct Borrowed<'a> {
    unsigned_buffer_calls: &'a [Option<NativeUnsignedBufferCall>],
    call_routes: &'a [LibraryCallRoute],
    call_cache: &'a call_cache::Layout,
    frame_retirements: &'a frame_retirement::FrameRetirements,
    cleanup_effects: &'a cleanup_effects::Layout,
    compositions: composition::image::Borrowed<'a>,
}
impl<'a> From<&'a ProgramMetadata> for Borrowed<'a> {
    fn from(metadata: &'a ProgramMetadata) -> Self {
        Self {
            unsigned_buffer_calls: &metadata.unsigned_buffer_calls,
            call_routes: &metadata.call_routes,
            call_cache: &metadata.call_cache,
            frame_retirements: &metadata.frame_retirements,
            cleanup_effects: &metadata.cleanup_effects,
            compositions: metadata.compositions.image(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owned {
    #[serde(deserialize_with = "buffers")]
    pub unsigned_buffer_calls: Box<[Option<NativeUnsignedBufferCall>]>,
    pub call_routes: Box<[LibraryCallRoute]>,
    pub call_cache: call_cache::Layout,
    pub frame_retirements: frame_retirement::FrameRetirements,
    pub cleanup_effects: cleanup_effects::Layout,
    pub compositions: composition::image::Owned,
}
fn buffers<'de, D: Deserializer<'de>>(
    image: D,
) -> Result<Box<[Option<NativeUnsignedBufferCall>]>, D::Error> {
    struct Call(NativeUnsignedBufferCall);
    impl<'de> Deserialize<'de> for Call {
        fn deserialize<D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
            // SAFETY: only the confined descriptor loader reaches this reader.
            // A value from an independent image is compared to fresh source
            // derivation; bound images have genuine checked-Object provenance.
            unsafe { NativeUnsignedBufferCall::from_build_owned_image_reader(image) }.map(Self)
        }
    }
    Box::<[Option<Call>]>::deserialize(image).map(|calls| {
        calls
            .into_vec()
            .into_iter()
            .map(|call| call.map(|call| call.0))
            .collect()
    })
}
