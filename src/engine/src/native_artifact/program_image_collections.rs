//! Private collection reconstruction. No producer, consumer, or kernel runs here.
#![allow(dead_code)]
use super::{WireOrigin, library};
use crate::native_artifact::*;
use mech_core::{FunctionCatalog, SchemaTable};
use serde::Deserialize;
use serde_json::Value as Json;

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
enum Kind {
    Matrix,
    MatrixPreserveShape,
    Set,
    MatrixHorizontalBlocks { rows: Rows },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Span {
    range_site: u32,
    selection_site: u32,
    range_template: Json,
    selection_template: Json,
    operands: [u32; 2],
    source: u32,
    selector: u32,
    output: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapProjection {
    iteration: u32,
    element_site: u32,
    selection_site: u32,
    source: u32,
    map: u32,
    element: u32,
    output: u32,
    selection_template: Json,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Collection {
    origin: WireOrigin,
    parent: Option<u32>,
    parent_collection: Option<u32>,
    kind: Kind,
    output: u32,
    result: Json,
    string_span_projection: Option<Span>,
    string_element_map_projection: Option<MapProjection>,
    generators: Box<[u32]>,
    begin: u32,
    push: u32,
    finish: u32,
}

/// # Safety
/// Keep all reconstructed collection proofs confined until the enclosing image
/// has unchanged compiler/Object provenance. Parsing is not escape analysis.
pub(super) unsafe fn collection(
    image: &Json,
    catalog: &FunctionCatalog,
    schemas: &SchemaTable,
) -> Result<NativeProgramCollection, String> {
    let wire = Collection::deserialize(image).map_err(|e| e.to_string())?;
    let kind = match wire.kind {
        Kind::Matrix => crate::ComprehensionKind::Matrix,
        Kind::MatrixPreserveShape => crate::ComprehensionKind::MatrixPreserveShape,
        Kind::Set => crate::ComprehensionKind::Set,
        Kind::MatrixHorizontalBlocks { rows } => crate::ComprehensionKind::MatrixHorizontalBlocks {
            rows: match rows {
                Rows::InferOrZero => crate::MatrixBlockRows::InferOrZero,
                Rows::Fixed(n) => crate::MatrixBlockRows::Fixed(n),
                Rows::InputMatrixElements { input } => {
                    crate::MatrixBlockRows::InputMatrixElements { input }
                }
            },
        },
    };
    let span = wire
        .string_span_projection
        .map(|s| -> Result<_, String> {
            Ok(NativeStringSpanProjection {
                range_site: s.range_site,
                selection_site: s.selection_site,
                // SAFETY: propagate the enclosing unchanged build-image obligation.
                range_template: unsafe { library::template(&s.range_template, catalog, schemas)? },
                // SAFETY: same confined image and selected catalog.
                selection_template: unsafe {
                    library::template(&s.selection_template, catalog, schemas)?
                },
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
                selection_template: unsafe {
                    library::template(&s.selection_template, catalog, schemas)?
                },
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
        result: unsafe { sinks::result(&wire.result, catalog, schemas)? },
        string_span_projection: span,
        string_element_map_projection: map,
        generators: wire.generators,
        begin: wire.begin,
        push: wire.push,
        finish: wire.finish,
    })
}
