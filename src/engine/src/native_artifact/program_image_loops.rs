//! Private reconstruction of immutable loop metadata, not loop execution.
use super::{WireOrigin, library};
use crate::native_artifact::*;
use mech_core::{FunctionCatalog, SchemaTable};
use serde::Deserialize;
use serde_json::Value as Json;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Counted {
    origin: WireOrigin,
    count: u64,
    parent: Option<u32>,
    seed: u32,
    carrier: u32,
    next: u32,
    result: u32,
    initialize: u32,
    header: u32,
    body: u32,
    latch: u32,
    exit: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tail {
    origin: WireOrigin,
    parent: Option<u32>,
    seed: u32,
    carrier: u32,
    next: u32,
    returned: u32,
    result: u32,
    initialize: u32,
    header: u32,
    body: u32,
    latch: u32,
    finish: u32,
    exit: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    prepare: u32,
    template: Json,
    operands: [u32; 2],
    output: u32,
    before_collection_begin: bool,
}
#[derive(Deserialize)]
enum Source {
    Value(u32),
    U64Range(Range),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Collection {
    origin: WireOrigin,
    parent: Option<u32>,
    collection: u32,
    source: Source,
    element: Option<u32>,
    initialize: u32,
    header: u32,
    body: u32,
    latch: u32,
    exit: u32,
}
#[derive(Deserialize)]
enum Loop {
    Iteration(Counted),
    Collection(Collection),
    Tail(Tail),
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Scope {
    Immutable { iteration: Option<u32> },
    Carrier { iteration: u32 },
}

fn origin(wire: WireOrigin) -> NativeMatchOrigin {
    NativeMatchOrigin {
        node: mech_core::NodeId::new(wire.node),
        block: wire.block,
        local: wire.local,
    }
}
pub(super) fn scope(image: &Json) -> Result<NativeIterationValueScope, String> {
    Ok(
        match Scope::deserialize(image).map_err(|e| e.to_string())? {
            Scope::Immutable { iteration } => NativeIterationValueScope::Immutable { iteration },
            Scope::Carrier { iteration } => NativeIterationValueScope::Carrier { iteration },
        },
    )
}

/// # Safety
/// Keep decoded indices/loop ownership confined until the enclosing loader has
/// established unchanged compiler/Object provenance. Parsing is not CFG proof.
pub(super) unsafe fn control_loop(
    image: &Json,
    catalog: &FunctionCatalog,
    schemas: &SchemaTable,
) -> Result<NativeControlLoop, String> {
    let wire = Loop::deserialize(image).map_err(|e| e.to_string())?;
    Ok(match wire {
        Loop::Iteration(w) => NativeControlLoop::Iteration(NativeIterationLoop {
            origin: origin(w.origin),
            count: w.count,
            parent: w.parent,
            seed: NativeMatchValue(w.seed),
            carrier: NativeMatchValue(w.carrier),
            next: NativeMatchValue(w.next),
            result: NativeMatchValue(w.result),
            initialize: w.initialize,
            header: w.header,
            body: w.body,
            latch: w.latch,
            exit: w.exit,
        }),
        Loop::Tail(w) => NativeControlLoop::Tail(NativeTailLoop {
            origin: origin(w.origin),
            parent: w.parent,
            seed: NativeMatchValue(w.seed),
            carrier: NativeMatchValue(w.carrier),
            next: NativeMatchValue(w.next),
            returned: NativeMatchValue(w.returned),
            result: NativeMatchValue(w.result),
            initialize: w.initialize,
            header: w.header,
            body: w.body,
            latch: w.latch,
            finish: w.finish,
            exit: w.exit,
        }),
        Loop::Collection(w) => NativeControlLoop::Collection(NativeCollectionLoop {
            origin: origin(w.origin),
            parent: w.parent,
            collection: w.collection,
            source: match w.source {
                Source::Value(value) => NativeCollectionSource::Value(NativeMatchValue(value)),
                Source::U64Range(range) => NativeCollectionSource::U64Range(NativeU64RangeCursor {
                    prepare: range.prepare,
                    // SAFETY: delegate the same enclosing build-image obligation.
                    template: unsafe { library::template(&range.template, catalog, schemas)? },
                    operands: range.operands.map(NativeMatchValue),
                    output: NativeMatchValue(range.output),
                    before_collection_begin: range.before_collection_begin,
                }),
            },
            element: w.element.map(NativeMatchValue),
            initialize: w.initialize,
            header: w.header,
            body: w.body,
            latch: w.latch,
            exit: w.exit,
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_loop_forms_and_value_scopes_preserve_wire_fields() {
        let origin = NativeMatchOrigin {
            node: mech_core::NodeId::new(1),
            block: Some(2),
            local: Some(3),
        };
        let loops = vec![
            NativeControlLoop::Iteration(NativeIterationLoop {
                origin,
                count: u64::MAX,
                parent: Some(0),
                seed: NativeMatchValue(1),
                carrier: NativeMatchValue(2),
                next: NativeMatchValue(3),
                result: NativeMatchValue(4),
                initialize: 5,
                header: 6,
                body: 7,
                latch: 8,
                exit: 9,
            }),
            NativeControlLoop::Tail(NativeTailLoop {
                origin,
                parent: None,
                seed: NativeMatchValue(1),
                carrier: NativeMatchValue(2),
                next: NativeMatchValue(3),
                returned: NativeMatchValue(4),
                result: NativeMatchValue(5),
                initialize: 6,
                header: 7,
                body: 8,
                latch: 9,
                finish: 10,
                exit: 11,
            }),
            NativeControlLoop::Collection(NativeCollectionLoop {
                origin,
                parent: Some(0),
                collection: 1,
                source: NativeCollectionSource::Value(NativeMatchValue(2)),
                element: Some(NativeMatchValue(3)),
                initialize: 4,
                header: 5,
                body: 6,
                latch: 7,
                exit: 8,
            }),
        ];
        let catalog = mech_core::FunctionCatalogBuilder::new().build().unwrap();
        let schemas = mech_core::SchemaTableBuilder::new().finish().unwrap().table;
        for original in loops {
            let image = serde_json::to_value(original).unwrap();
            // SAFETY: metadata-only round trip; synthetic graph indices never execute.
            let decoded = unsafe { control_loop(&image, &catalog, &schemas) }.unwrap();
            assert_eq!(serde_json::to_value(decoded).unwrap(), image);
        }
        for original in [
            NativeIterationValueScope::Immutable { iteration: None },
            NativeIterationValueScope::Immutable { iteration: Some(3) },
            NativeIterationValueScope::Carrier { iteration: 4 },
        ] {
            let image = serde_json::to_value(original).unwrap();
            assert_eq!(scope(&image).unwrap(), original);
        }
    }
}
