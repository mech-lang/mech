//! Confined image representation of an already-derived composition inventory.
//! No reader is public and no invocation state or owner is persisted.
use super::{Layout, Parent};
use mech_engine::native_artifact::NativeMatchValue;
use serde::{Deserialize, Deserializer, Serialize};
use std::num::NonZeroUsize;

#[derive(Serialize)]
pub(in crate::native_artifact) struct Borrowed<'a> {
    parents: &'a [Parent],
    entries: &'a [Option<NonZeroUsize>],
    regions: &'a [(usize, usize)],
    boundaries: &'a [(usize, usize)],
    words: usize,
}
impl Layout {
    pub(in crate::native_artifact) fn image(&self) -> Borrowed<'_> {
        Borrowed {
            parents: &self.parents,
            entries: &self.entries,
            regions: &self.regions,
            boundaries: &self.boundaries,
            words: self.words,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Owned {
    parents: Vec<Parent>,
    entries: Box<[Option<NonZeroUsize>]>,
    regions: Vec<(usize, usize)>,
    boundaries: Vec<(usize, usize)>,
    words: usize,
}
impl From<Owned> for Layout {
    fn from(image: Owned) -> Self {
        Self {
            parents: image.parents,
            entries: image.entries,
            regions: image.regions,
            boundaries: image.boundaries,
            words: image.words,
        }
    }
}

pub(super) fn value<'de, D: Deserializer<'de>>(image: D) -> Result<NativeMatchValue, D::Error> {
    // SAFETY: these private adapters are used only within the confined descriptor
    // image loader. Independent loading rederives and compares the complete
    // inventory; bound loading first verifies genuine same-Object bytes.
    unsafe { NativeMatchValue::from_build_owned_image_reader(image) }
}
pub(super) fn values<'de, D: Deserializer<'de>>(
    image: D,
) -> Result<Vec<NativeMatchValue>, D::Error> {
    #[derive(Deserialize)]
    struct Value(#[serde(deserialize_with = "value")] NativeMatchValue);
    Vec::<Value>::deserialize(image).map(|values| values.into_iter().map(|value| value.0).collect())
}
