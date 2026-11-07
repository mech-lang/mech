//! Confined image representation of an already-derived composition inventory.
//! No reader is public and no invocation state or owner is persisted.
use super::{Child, Layout, Parent, Segment, frames, numeric};
use crate::native_artifact::program::prepare::readonly_numeric::FrameWords;
use mech_engine::native_artifact::NativeMatchValue;
use serde::{Deserialize, Deserializer, Serialize};
use std::num::NonZeroUsize;

#[derive(Serialize)]
pub(in crate::native_artifact) struct Borrowed<'a> {
    parents: &'a [Parent],
    entries: &'a [Option<NonZeroUsize>],
    regions: &'a [(usize, usize)],
    boundaries: &'a [(usize, usize)],
    native_contexts: &'a [Option<u32>],
    words: usize,
}
impl Layout {
    pub(in crate::native_artifact) fn image(&self) -> Borrowed<'_> {
        Borrowed {
            parents: &self.parents,
            entries: &self.entries,
            regions: &self.regions,
            boundaries: &self.boundaries,
            native_contexts: &self.native_contexts,
            words: self.words,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Owned<Frame = FrameWords> {
    parents: Vec<ParentImage<Frame>>,
    entries: Box<[Option<NonZeroUsize>]>,
    regions: Vec<(usize, usize)>,
    boundaries: Vec<(usize, usize)>,
    native_contexts: Box<[Option<u32>]>,
    words: usize,
}
impl<Frame: Into<FrameWords>> From<Owned<Frame>> for Layout {
    fn from(image: Owned<Frame>) -> Self {
        Self {
            parents: image.parents.into_iter().map(Into::into).collect(),
            entries: image.entries,
            regions: image.regions,
            boundaries: image.boundaries,
            native_contexts: image.native_contexts,
            words: image.words,
        }
    }
}

// Keep the frame reader selected by the enclosing private image loader all the
// way through parent pieces. Only genuine compiled-module bytes select the
// process-lifetime CompiledFrame adapter; independent readers retain FrameWords.
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Parent")]
struct ParentImage<Frame> {
    iteration: u32,
    suspended_child: Option<u32>,
    children: Vec<Child>,
    segments: Vec<SegmentImage<Frame>>,
}
impl<Frame: Into<FrameWords>> From<ParentImage<Frame>> for Parent {
    fn from(image: ParentImage<Frame>) -> Self {
        Self {
            iteration: image.iteration,
            suspended_child: image.suspended_child,
            children: image.children,
            segments: image.segments.into_iter().map(Into::into).collect(),
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Segment")]
struct SegmentImage<Frame> {
    entry: u32,
    sites: Vec<u32>,
    exits: Vec<(u32, u32)>,
    frame_plan: Option<FrameImage<Frame>>,
    numeric: Option<numeric::Layout>,
    prepaid_region: Option<usize>,
    rejection: Option<std::borrow::Cow<'static, str>>,
}
impl<Frame: Into<FrameWords>> From<SegmentImage<Frame>> for Segment {
    fn from(image: SegmentImage<Frame>) -> Self {
        Self {
            entry: image.entry,
            sites: image.sites,
            exits: image.exits,
            frame_plan: image.frame_plan.map(Into::into),
            numeric: image.numeric,
            prepaid_region: image.prepaid_region,
            rejection: image.rejection,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "Layout")]
struct FrameImage<Frame> {
    sources: Vec<frames::Source>,
    aliases: Vec<frames::Alias>,
    frames: Vec<Frame>,
    inputs: Vec<u32>,
    scalar_words: usize,
    scalar_bindings: Vec<super::super::readonly_numeric::ScalarBinding>,
    scalar_entry_steps: usize,
    retirements: Vec<(u32, Vec<u32>)>,
    exits: Vec<frames::Exit>,
}
impl<Frame: Into<FrameWords>> From<FrameImage<Frame>> for frames::Layout {
    fn from(image: FrameImage<Frame>) -> Self {
        Self {
            sources: image.sources,
            aliases: image.aliases,
            frames: image.frames.into_iter().map(Into::into).collect(),
            inputs: image.inputs,
            scalar_words: image.scalar_words,
            scalar_bindings: image.scalar_bindings,
            scalar_entry_steps: image.scalar_entry_steps,
            retirements: image.retirements,
            exits: image.exits,
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
