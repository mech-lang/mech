//! Private sink metadata decoding; invocation accounting stays in the runtime.
use super::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Join {
    consumer: u32,
    template: Json,
    separator: u32,
    output: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Histogram {
    consumer: u32,
    template: Json,
    output: u32,
}
#[derive(Deserialize)]
enum Phase {
    ProducerReady,
    HistogramReady,
    KeysReady,
    SetReady,
    Guarded,
    LookupRead,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum StepKind {
    Histogram,
    Keys,
    Set,
    Membership,
    Lookup,
    Cleanup,
    Alias { phase: Phase },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    site: u32,
    kind: StepKind,
    template: Option<Json>,
    operands: Box<[u32]>,
    output: Option<u32>,
}
#[derive(Deserialize)]
enum SetShape {
    Dynamic,
    FreeExact,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    query: u32,
    steps: Box<[Step]>,
    set_shape: SetShape,
    match_result: u32,
    zero: u32,
    bool_tests: Box<[u32]>,
    true_copy: u32,
    false_copy: u32,
}
#[derive(Deserialize)]
enum Sink {
    Materialized,
    TextJoin(Join),
    Histogram(Histogram),
    GuardedHistogramQuery(Query),
}

/// # Safety
/// The enclosing collection decoder's unchanged build-image confinement applies.
pub(super) unsafe fn result(
    image: &Json,
    catalog: &FunctionCatalog,
    schemas: &SchemaTable,
) -> Result<NativeCollectionResult, String> {
    let sink: Sink = serde_json::from_value(image.clone()).map_err(|e| e.to_string())?;
    Ok(match sink {
        Sink::Materialized => NativeCollectionResult::Materialized,
        Sink::TextJoin(w) => NativeCollectionResult::TextJoin(NativeTextJoinSink {
            consumer: w.consumer,
            separator: NativeMatchValue(w.separator),
            output: NativeMatchValue(w.output),
            // SAFETY: same confined image and selected catalog.
            template: unsafe { library::template(&w.template, catalog, schemas)? },
        }),
        Sink::Histogram(w) => NativeCollectionResult::Histogram(NativeHistogramSink {
            consumer: w.consumer,
            output: NativeMatchValue(w.output),
            // SAFETY: same confined image and selected catalog.
            template: unsafe { library::template(&w.template, catalog, schemas)? },
        }),
        Sink::GuardedHistogramQuery(w) => {
            let steps = w
                .steps
                .into_vec()
                .into_iter()
                .map(|s| -> Result<_, String> {
                    use NativeHistogramQueryStepKind as K;
                    let kind = match s.kind {
                        StepKind::Histogram => K::Histogram,
                        StepKind::Keys => K::Keys,
                        StepKind::Set => K::Set,
                        StepKind::Membership => K::Membership,
                        StepKind::Lookup => K::Lookup,
                        StepKind::Cleanup => K::Cleanup,
                        StepKind::Alias { phase } => K::Alias {
                            phase: match phase {
                                Phase::ProducerReady => NativeHistogramQueryPhase::ProducerReady,
                                Phase::HistogramReady => NativeHistogramQueryPhase::HistogramReady,
                                Phase::KeysReady => NativeHistogramQueryPhase::KeysReady,
                                Phase::SetReady => NativeHistogramQueryPhase::SetReady,
                                Phase::Guarded => NativeHistogramQueryPhase::Guarded,
                                Phase::LookupRead => NativeHistogramQueryPhase::LookupRead,
                            },
                        },
                    };
                    let template = s
                        .template
                        .map(|t| {
                            // SAFETY: same confined image and selected catalog.
                            unsafe { library::template(&t, catalog, schemas) }
                        })
                        .transpose()?;
                    Ok(NativeHistogramQueryStep {
                        site: s.site,
                        kind,
                        template,
                        operands: s
                            .operands
                            .into_vec()
                            .into_iter()
                            .map(NativeMatchValue)
                            .collect(),
                        output: s.output.map(NativeMatchValue),
                    })
                })
                .collect::<Result<Box<[_]>, String>>()?;
            NativeCollectionResult::GuardedHistogramQuery(NativeGuardedHistogramQuery {
                query: NativeMatchValue(w.query),
                steps,
                set_shape: match w.set_shape {
                    SetShape::Dynamic => NativeHistogramQuerySetShape::Dynamic,
                    SetShape::FreeExact => NativeHistogramQuerySetShape::FreeExact,
                },
                match_result: NativeMatchValue(w.match_result),
                zero: NativeMatchValue(w.zero),
                bool_tests: w.bool_tests,
                true_copy: w.true_copy,
                false_copy: w.false_copy,
            })
        }
    })
}
