//! Private wire reconstruction of native iteration blocks; never a scheduler.
use super::{WireAction, WireOrigin, WireTerminator, action, library, terminator};
use crate::native_artifact::*;
use mech_core::{FunctionCatalog, SchemaTable};
use serde::Deserialize;
use serde_json::Value as Json;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Step {
    EnumPayload { ordinal: u32 },
    TupleItem { ordinal: u32 },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Observation {
    EnumOrdinal { ordinal: u32 },
    U64,
    CanonicalValue,
    StringEqual { peer: u32 },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Action {
    Scalar(WireAction),
    Structural {
        source: u32,
        path: Box<[Step]>,
        observation: Observation,
        output: u32,
    },
    LibraryCall {
        template: Json,
        operands: Box<[u32]>,
        output: u32,
    },
    PrepareU64Range {
        iteration: u32,
    },
    PrepareStringSpanSelector {
        collection: u32,
    },
    ProjectStringSpan {
        collection: u32,
    },
    ProjectStringMapEntry {
        collection: u32,
    },
    CollectionBegin {
        collection: u32,
    },
    CollectionElement {
        iteration: u32,
    },
    CollectionPush {
        collection: u32,
        value: u32,
    },
    CollectionFinish {
        collection: u32,
    },
    TextJoinFinish {
        collection: u32,
    },
    HistogramQueryStep {
        collection: u32,
        step: u32,
    },
    HistogramFinish {
        collection: u32,
    },
    Initialize {
        iteration: u32,
    },
    Latch {
        iteration: u32,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Terminator {
    Scalar(WireTerminator),
    TailHeader {
        iteration: u32,
        body: u32,
    },
    Header {
        iteration: u32,
        body: u32,
        exit: u32,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Block {
    origin: WireOrigin,
    action: Action,
    terminator: Terminator,
    iteration: Option<u32>,
}

/// # Safety
/// Returned metadata must stay confined until the enclosing loader establishes
/// unchanged compiler/Object provenance, including all loop and value indices.
pub(super) unsafe fn block(
    image: &Json,
    catalog: &FunctionCatalog,
    schemas: &SchemaTable,
) -> Result<NativeIterationBlock, String> {
    let wire = Block::deserialize(image).map_err(|e| e.to_string())?;
    use NativeIterationAction as N;
    let action = match wire.action {
        Action::Scalar(a) => N::Scalar(action(a)),
        Action::Structural {
            source,
            path,
            observation,
            output,
        } => N::Structural {
            source: NativeMatchValue(source),
            output: NativeMatchValue(output),
            path: path
                .into_vec()
                .into_iter()
                .map(|step| match step {
                    Step::EnumPayload { ordinal } => NativeStructuralStep::EnumPayload { ordinal },
                    Step::TupleItem { ordinal } => NativeStructuralStep::TupleItem { ordinal },
                })
                .collect(),
            observation: match observation {
                Observation::EnumOrdinal { ordinal } => {
                    NativeStructuralObservation::EnumOrdinal { ordinal }
                }
                Observation::U64 => NativeStructuralObservation::U64,
                Observation::CanonicalValue => NativeStructuralObservation::CanonicalValue,
                Observation::StringEqual { peer } => NativeStructuralObservation::StringEqual {
                    peer: NativeMatchValue(peer),
                },
            },
        },
        Action::LibraryCall {
            template,
            operands,
            output,
        } => N::LibraryCall {
            // SAFETY: propagate the enclosing build-image confinement contract.
            template: unsafe { library::template(&template, catalog, schemas)? },
            operands: operands
                .into_vec()
                .into_iter()
                .map(NativeMatchValue)
                .collect(),
            output: NativeMatchValue(output),
        },
        Action::PrepareU64Range { iteration } => N::PrepareU64Range { iteration },
        Action::PrepareStringSpanSelector { collection } => {
            N::PrepareStringSpanSelector { collection }
        }
        Action::ProjectStringSpan { collection } => N::ProjectStringSpan { collection },
        Action::ProjectStringMapEntry { collection } => N::ProjectStringMapEntry { collection },
        Action::CollectionBegin { collection } => N::CollectionBegin { collection },
        Action::CollectionElement { iteration } => N::CollectionElement { iteration },
        Action::CollectionPush { collection, value } => N::CollectionPush {
            collection,
            value: NativeMatchValue(value),
        },
        Action::CollectionFinish { collection } => N::CollectionFinish { collection },
        Action::TextJoinFinish { collection } => N::TextJoinFinish { collection },
        Action::HistogramQueryStep { collection, step } => {
            N::HistogramQueryStep { collection, step }
        }
        Action::HistogramFinish { collection } => N::HistogramFinish { collection },
        Action::Initialize { iteration } => N::Initialize { iteration },
        Action::Latch { iteration } => N::Latch { iteration },
    };
    let terminator = match wire.terminator {
        Terminator::Scalar(t) => NativeIterationTerminator::Scalar(terminator(t)),
        Terminator::TailHeader { iteration, body } => {
            NativeIterationTerminator::TailHeader { iteration, body }
        }
        Terminator::Header {
            iteration,
            body,
            exit,
        } => NativeIterationTerminator::Header {
            iteration,
            body,
            exit,
        },
    };
    Ok(NativeIterationBlock {
        origin: NativeMatchOrigin {
            node: mech_core::NodeId::new(wire.origin.node),
            block: wire.origin.block,
            local: wire.origin.local,
        },
        action,
        terminator,
        iteration: wire.iteration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn iteration_actions_preserve_exact_wire_fields() {
        use NativeIterationAction as N;
        let actions = vec![
            N::Scalar(NativeMatchAction::None),
            N::Structural {
                source: NativeMatchValue(2),
                path: Box::new([NativeStructuralStep::TupleItem { ordinal: 1 }]),
                observation: NativeStructuralObservation::U64,
                output: NativeMatchValue(3),
            },
            N::PrepareU64Range { iteration: 2 },
            N::PrepareStringSpanSelector { collection: 3 },
            N::ProjectStringSpan { collection: 4 },
            N::ProjectStringMapEntry { collection: 5 },
            N::CollectionBegin { collection: 6 },
            N::CollectionElement { iteration: 7 },
            N::CollectionPush {
                collection: 8,
                value: NativeMatchValue(9),
            },
            N::CollectionFinish { collection: 10 },
            N::TextJoinFinish { collection: 11 },
            N::HistogramQueryStep {
                collection: 12,
                step: 13,
            },
            N::HistogramFinish { collection: 14 },
            N::Initialize { iteration: 15 },
            N::Latch { iteration: 16 },
        ];
        let catalog = mech_core::FunctionCatalogBuilder::new().build().unwrap();
        let schemas = mech_core::SchemaTableBuilder::new().finish().unwrap().table;
        for action in actions {
            let original = NativeIterationBlock {
                origin: NativeMatchOrigin {
                    node: mech_core::NodeId::new(1),
                    block: Some(2),
                    local: Some(3),
                },
                action,
                terminator: NativeIterationTerminator::Header {
                    iteration: 0,
                    body: 1,
                    exit: 2,
                },
                iteration: Some(0),
            };
            let image = serde_json::to_value(original).unwrap();
            // SAFETY: confined metadata round-trip test; synthetic indices never execute.
            let loaded = unsafe { block(&image, &catalog, &schemas) }.unwrap();
            assert_eq!(serde_json::to_value(loaded).unwrap(), image);
        }
    }
}
