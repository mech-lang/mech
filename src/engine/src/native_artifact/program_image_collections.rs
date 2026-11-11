//! Private collection reconstruction. No producer, consumer, or kernel runs here.
use super::{WireOrigin, library};
use crate::native_artifact::*;
use serde::{Deserialize, Deserializer};

#[path = "program_image_sinks.rs"]
mod sinks;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Rows {
    InferOrZero,
    Fixed(u64),
    InputMatrixElements { input: u16 },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum CollectionImageKind {
    Matrix,
    MatrixPreserveShape,
    Set,
    MatrixHorizontalBlocks { rows: Rows },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Span")]
struct Span<'a> {
    range_site: u32,
    selection_site: u32,
    #[serde(borrow)]
    range_template: BuildOwnedImageRecord<'a>,
    #[serde(borrow)]
    selection_template: BuildOwnedImageRecord<'a>,
    operands: [u32; 2],
    source: u32,
    selector: u32,
    output: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.MapProjection")]
struct MapProjection<'a> {
    iteration: u32,
    element_site: u32,
    selection_site: u32,
    source: u32,
    map: u32,
    element: u32,
    output: u32,
    #[serde(borrow)]
    selection_template: BuildOwnedImageRecord<'a>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Collection")]
struct Collection<'a> {
    origin: WireOrigin,
    parent: Option<u32>,
    parent_collection: Option<u32>,
    kind: CollectionImageKind,
    output: u32,
    #[serde(borrow)]
    result: BuildOwnedImageRecord<'a>,
    #[serde(borrow)]
    string_span_projection: Option<Span<'a>>,
    #[serde(borrow)]
    string_element_map_projection: Option<MapProjection<'a>>,
    generators: Box<[u32]>,
    begin: u32,
    push: u32,
    finish: u32,
}

/// # Safety
/// Keep all reconstructed collection proofs confined until the enclosing image
/// has unchanged compiler/Object provenance. Parsing is not escape analysis.
pub(super) unsafe fn collection<'de, D: Deserializer<'de>>(
    image: D,
    templates: &library::Templates<'_>,
) -> Result<NativeProgramCollection, String> {
    let wire = Collection::deserialize(image).map_err(|e| e.to_string())?;
    let kind = match wire.kind {
        CollectionImageKind::Matrix => crate::ComprehensionKind::Matrix,
        CollectionImageKind::MatrixPreserveShape => crate::ComprehensionKind::MatrixPreserveShape,
        CollectionImageKind::Set => crate::ComprehensionKind::Set,
        CollectionImageKind::MatrixHorizontalBlocks { rows } => {
            crate::ComprehensionKind::MatrixHorizontalBlocks {
                rows: match rows {
                    Rows::InferOrZero => crate::MatrixBlockRows::InferOrZero,
                    Rows::Fixed(n) => crate::MatrixBlockRows::Fixed(n),
                    Rows::InputMatrixElements { input } => {
                        crate::MatrixBlockRows::InputMatrixElements { input }
                    }
                },
            }
        }
    };
    let span = wire
        .string_span_projection
        .map(|s| -> Result<_, String> {
            Ok(NativeStringSpanProjection {
                range_site: s.range_site,
                selection_site: s.selection_site,
                // SAFETY: propagate the enclosing unchanged build-image obligation.
                range_template: unsafe { templates.read(s.range_template)? },
                // SAFETY: same confined image and selected catalog.
                selection_template: unsafe { templates.read(s.selection_template)? },
                operands: s.operands.map(NativeMatchValue),
                source: NativeMatchValue(s.source),
                selector: NativeMatchValue(s.selector),
                output: NativeMatchValue(s.output),
            })
        })
        .transpose()?;
    let map = wire
        .string_element_map_projection
        .map(|s| -> Result<_, String> {
            Ok(NativeStringElementMapProjection {
                iteration: s.iteration,
                element_site: s.element_site,
                selection_site: s.selection_site,
                source: NativeMatchValue(s.source),
                map: NativeMatchValue(s.map),
                element: NativeMatchValue(s.element),
                output: NativeMatchValue(s.output),
                // SAFETY: propagate the enclosing unchanged build-image obligation.
                selection_template: unsafe { templates.read(s.selection_template)? },
            })
        })
        .transpose()?;
    Ok(NativeProgramCollection {
        origin: NativeMatchOrigin {
            node: mech_core::NodeId::new(wire.origin.node),
            block: wire.origin.block,
            local: wire.origin.local,
        },
        parent: wire.parent,
        parent_collection: wire.parent_collection,
        kind,
        output: NativeMatchValue(wire.output),
        // SAFETY: result metadata belongs to the same confined collection image.
        result: unsafe { sinks::result(&mut wire.result.reader(), templates)? },
        string_span_projection: span,
        string_element_map_projection: map,
        generators: wire.generators,
        begin: wire.begin,
        push: wire.push,
        finish: wire.finish,
    })
}
