//! Immutable compiler results restored only inside the confined image loader.
//! Actual linked factories, owner admission and current visits remain checked.
use crate::native_artifact::program::prepare::{
    ProgramMetadata, call_cache, call_routes::LibraryCallRoute, cleanup_effects, composition,
    frame_retirement,
};
use mech_engine::native_artifact::{NativeUniformWordCollection, NativeUnsignedBufferCall};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Serialize)]
pub(super) struct Borrowed<'a> {
    unsigned_buffer_calls: &'a [Option<NativeUnsignedBufferCall>],
    call_routes: &'a [LibraryCallRoute],
    call_cache: &'a call_cache::Layout,
    frame_retirements: &'a frame_retirement::FrameRetirements,
    cleanup_effects: &'a cleanup_effects::Layout,
    compositions: composition::image::Borrowed<'a>,
    uniform_word_collections: &'a [NativeUniformWordCollection],
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
            uniform_word_collections: &metadata.uniform_word_collections,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owned<Frame = super::super::prepare::readonly_numeric::FrameWords> {
    #[serde(deserialize_with = "buffers")]
    pub unsigned_buffer_calls: Box<[Option<NativeUnsignedBufferCall>]>,
    pub call_routes: Box<[LibraryCallRoute]>,
    pub call_cache: call_cache::Layout,
    pub frame_retirements: frame_retirement::FrameRetirements,
    pub cleanup_effects: cleanup_effects::Layout,
    pub compositions: composition::image::Owned<Frame>,
    // Missing only in genuine V4/V5 images. A present field must contain an
    // array, including the empty inventory; null cannot impersonate absence.
    #[serde(default, deserialize_with = "uniform_collections")]
    pub uniform_word_collections: Option<Box<[NativeUniformWordCollection]>>,
}
fn uniform_collections<'de, D: Deserializer<'de>>(
    image: D,
) -> Result<Option<Box<[NativeUniformWordCollection]>>, D::Error> {
    struct Collection(NativeUniformWordCollection);
    impl<'de> Deserialize<'de> for Collection {
        fn deserialize<D: Deserializer<'de>>(image: D) -> Result<Self, D::Error> {
            // SAFETY: confined checked-build loader only. Independent images
            // rederive and compare every field before descriptor publication;
            // bound images inherit the exact immutable same-Object association.
            unsafe { NativeUniformWordCollection::from_build_owned_image_reader(image) }.map(Self)
        }
    }
    Box::<[Collection]>::deserialize(image).map(|regions| {
        Some(
            regions
                .into_vec()
                .into_iter()
                .map(|region| region.0)
                .collect(),
        )
    })
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
