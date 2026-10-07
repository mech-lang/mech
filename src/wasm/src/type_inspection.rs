//! Thin inspection adapter over canonical compilation and resident publication.
//! Numeric payloads are exported as Rust-formatted strings to preserve u128.
use std::collections::BTreeMap;
use std::sync::Arc;

use mech_core::snapshot::SnapshotValidationContext;
use mech_core::{
    IntegerInterval, IntegerWidth, ReactiveInstanceId, SchemaBody, SchemaDraft, SchemaTableBuilder,
    Value, ValueDataDraft, ValueDraft,
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, ReactiveInstance, activate};
use mech_engine::{CanonicalSourceFrontend, CanonicalSourceProgram, ProgramArtifact};
use mech_runtime::{
    CanonicalDocumentRenderer, CanonicalRenderScope, CanonicalScopeResults, RuntimeValueSnapshot,
    SourceDocument,
};
use mech_syntax::document::{
    AstNode, CodeBlockSyntax, CodeFenceScope, ParseConfig, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot, render_source_excerpt,
};
use serde_json::{Value as Json, json};
use wasm_bindgen::prelude::*;

const INTERVAL_SOURCE: &str = "~state⟨u8:1..10⟩ := 2\nstate = signal\nstate\n";

fn semantic_error(error: mech_engine::SourceSemanticError, source: &TextSnapshot) -> Json {
    let excerpt = render_source_excerpt(source, error.anchor.range, &error.message);
    let severity = if error.code == "source-semantics/empty-document" {
        "info"
    } else {
        "error"
    };
    let report = format!(
        "{}[{}]: {}\n{}",
        if severity == "info" { "Info" } else { "Error" },
        error.code,
        error.message,
        excerpt
    );
    json!({"phase":"semantic_checking", "code":error.code, "message":error.message,
        "document":error.anchor.document.0.to_string(), "revision":error.anchor.revision.0.to_string(),
        "range":[error.anchor.range.start.0,error.anchor.range.end.0], "severity":severity, "presentation":{"report":report,"excerpt":excerpt}})
}
fn scalar_text(value: &Value) -> Option<String> {
    use mech_core::ValueData;
    match value.data() {
        ValueData::U8(value) => Some(value.to_string()),
        ValueData::U64(value) => Some(value.to_string()),
        ValueData::U128(value) => Some(value.to_string()),
        ValueData::F64(value) => Some(value.to_f64().to_string()),
        _ => None,
    }
}
fn describe(value: &Value) -> Json {
    let mut result = json!({"schema":value.schemas().and_then(|schemas|schemas.get(value.schema()).map(|schema|format!("{:?}",schema.body()))),"shape":format!("{:?}",value.shape()),
        "value":format!("{:?}",value.data()),"scalar_text":scalar_text(value),"transport":"exact Rust value text; no JavaScript Number conversion"});
    match mech_runtime::RuntimeValueSnapshot::try_from(value) {
        Ok(snapshot) => {
            let limit = mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT;
            result["html"] = json!(snapshot.format_repl_html(limit));
            result["text"] = json!(snapshot.format_repl_inline(limit));
            result["kind"] = json!(snapshot.format_repl_kind());
        }
        Err(error) => result["format_error"] = json!(error.display_message()),
    }
    result
}

/// Parse, check, construct, activate and execute through real public interfaces.
/// Each successful stage is reported separately; source diagnostics retain anchors.
#[wasm_bindgen(js_name = inspectMechTypes)]
pub fn inspect_mech_types(source: &str) -> String {
    inspect(source).to_string()
}
fn inspect(source: &str) -> Json {
    let mut result = json!({"source":source,"target":if cfg!(target_arch="wasm32"){"WASM-hosted resident CPU"}else{"native resident CPU"},"stages":{},"diagnostics":[]});
    let document = match SourceDocument::parse_resolved(
        "audit:types",
        Revision(0),
        source,
        ParseConfig::default(),
    ) {
        Ok(value) => value,
        Err(error) => {
            result["diagnostics"] = json!([{"phase":"parsing","message":format!("{error:?}")}]);
            return result;
        }
    };
    result["stages"]["parsing"] = json!("completed");
    if !document.is_strictly_clean() {
        result["stages"]["semantic_checking"] = json!("blocked by strict syntax validation");
        result["diagnostics"] =
            serde_json::to_value(document.snapshot().diagnostics.as_slice()).unwrap_or(json!([]));
        return result;
    }
    let catalog = mech_stdlib::source_catalog();
    let program = match CanonicalSourceFrontend
        .compile_document_with_catalog(&document.document(), Arc::clone(&catalog))
    {
        Ok(value) => value,
        Err(error) => {
            result["stages"]["semantic_checking"] = json!("rejected");
            result["diagnostics"] = json!([semantic_error(error, &document.snapshot().source)]);
            // Exercise the product handoff as well as direct frontend inspection.
            result["product_diagnostic"] = match mech_runtime::RuntimeBuilder::new()
                .function_catalog(Arc::clone(&catalog))
                .build_compiler()
                .and_then(|mut compiler| compiler.compile_document(&document))
            {
                Err(error) => json!({
                    "semantic":error.kind_as::<mech_engine::SourceSemanticError>().cloned().map(|error| semantic_error(error, &document.snapshot().source)),
                    "presentation_range":error.primary_range().map(|range|format!("{range:?}")),
                    "message":error.display_message(),
                }),
                Ok(_) => {
                    json!({"unexpected":"product compiler accepted a rejected semantic candidate"})
                }
            };
            return result;
        }
    };
    inspect_program(&program, &catalog, &mut result);
    result
}

fn inspect_program(
    program: &CanonicalSourceProgram,
    catalog: &Arc<mech_core::FunctionCatalog>,
    result: &mut Json,
) -> Option<Vec<Value>> {
    result["stages"]["semantic_checking"] = json!("completed");
    result["inputs"]=json!(program.program().inputs.iter().map(|input|json!({"name":input.name,"schema":format!("{:?}",program.schemas().get(input.schema))})).collect::<Vec<_>>());
    result["outputs"] = json!(
        program
            .program()
            .outputs
            .iter()
            .map(|output| format!("{:?}", program.schemas().get(output.schema)))
            .collect::<Vec<_>>()
    );
    let artifact = match program.compile_artifact() {
        Ok(value) => value,
        Err(error) => {
            result["stages"]["artifact_construction"] = json!("rejected");
            result["diagnostics"] =
                json!([{"phase":"artifact_construction","message":format!("{error:?}")}]);
            return None;
        }
    };
    result["stages"]["artifact_construction"] = json!("completed");
    let encoded = match mech_engine::encode_program_artifact_bytecode_v1(&artifact) {
        Ok(value) => value,
        Err(error) => {
            result["diagnostics"] =
                json!([{"phase":"artifact_transport","message":format!("{error:?}")}]);
            return None;
        }
    };
    result["artifact_bytes"] = json!(encoded.len());
    let decoded = match mech_engine::decode_program_artifact_bytecode_v1(&encoded) {
        Ok(value) => value,
        Err(error) => {
            result["diagnostics"] =
                json!([{"phase":"artifact_transport","message":format!("{error:?}")}]);
            return None;
        }
    };
    result["artifact_transport"] = json!("bytecode-v1 encoded and decoded before activation");
    let mut instance = match activate(
        ReactiveInstanceId::new(0xA04, 0),
        &decoded,
        &catalog,
        &ActivationFacts::default(),
    ) {
        Ok(value) => value,
        Err(error) => {
            result["stages"]["activation"] = json!("rejected");
            result["diagnostics"] = json!([{"phase":"activation","message":format!("{error:?}")}]);
            return None;
        }
    };
    result["stages"]["activation"] = json!("completed");
    if !instance.plan.inputs.is_empty() {
        result["stages"]["execution"] = json!("awaiting external inputs");
        return None;
    }
    match instance.turn(&[]) {
        Ok(_) => {
            result["stages"]["execution"] = json!("completed");
            result["stages"]["correct_publication"] =
                json!("published; independent expected-value comparison required");
            let values = match (0..decoded.outputs().len())
                .map(|index| instance.copied_output(index))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(values) => values,
                Err(error) => {
                    result["diagnostics"] =
                        json!([{"phase":"publication","message":format!("{error:?}")}]);
                    return None;
                }
            };
            result["values"] = json!(values.iter().map(describe).collect::<Vec<_>>());
            result["epoch"] = json!(format!("{:?}", instance.published_epoch()));
            return Some(values);
        }
        Err(error) => {
            result["stages"]["execution"] = json!("rejected");
            result["diagnostics"] = json!([{"phase":"execution","message":format!("{error:?}")}]);
        }
    }
    None
}

/// Compile each canonical document-local execution owner and render its completed
/// fence/inline outputs against the same retained source revision.
#[wasm_bindgen(js_name = inspectMechDocument)]
pub fn inspect_mech_document(source: &str) -> String {
    inspect_document(source).to_string()
}

fn named_scopes(root: &SyntaxNode) -> Vec<String> {
    let mut names = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(node) = pending.pop() {
        if matches!(
            node.kind(),
            SyntaxKind::MikaSection | SyntaxKind::InlineMechCode
        ) {
            continue;
        }
        if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
            if let Some(CodeFenceScope::Named(name)) = fence.info().map(|info| info.scope)
                && !names.contains(&name)
            {
                names.push(name);
            }
            continue;
        }
        let children: Vec<_> = node.children().collect();
        pending.extend(children.into_iter().rev());
    }
    names
}

fn inspect_document(source: &str) -> Json {
    let mut result = json!({"source":source,"target":if cfg!(target_arch="wasm32"){"WASM-hosted resident CPU"}else{"native resident CPU"},"stages":{},"diagnostics":[],"scopes":[],"values":[],"inputs":[],"outputs":[]});
    let document = match SourceDocument::parse_resolved(
        "audit:document",
        Revision(0),
        source,
        ParseConfig::default(),
    ) {
        Ok(document) => document,
        Err(error) => {
            result["diagnostics"] = json!([{"phase":"parsing","message":format!("{error:?}")}]);
            return result;
        }
    };
    result["stages"]["parsing"] = json!("completed");
    if !document.is_strictly_clean() {
        result["stages"]["semantic_checking"] = json!("blocked by strict syntax validation");
        result["diagnostics"] =
            serde_json::to_value(document.snapshot().diagnostics.as_slice()).unwrap_or(json!([]));
        return result;
    }
    let syntax = document.document();
    let catalog = mech_stdlib::source_catalog();
    let frontend = CanonicalSourceFrontend;
    let mut candidates = vec![(
        syntax.scope_id(),
        "document".to_owned(),
        CanonicalRenderScope::Root,
        frontend.compile_document_with_catalog(&syntax, Arc::clone(&catalog)),
    )];
    for name in named_scopes(syntax.syntax()) {
        let program = frontend.compile_named_document_scope_with_catalog(
            &syntax,
            &name,
            Arc::clone(&catalog),
        );
        candidates.push((
            syntax.scope_id(),
            "document".to_owned(),
            CanonicalRenderScope::Named(name),
            program,
        ));
    }
    for child in syntax.mika_scopes() {
        let section = child.section;
        let owner_label = format!("Mika at byte {}", section.syntax().range().start.0);
        candidates.push((
            section.scope_id(),
            owner_label.clone(),
            CanonicalRenderScope::Root,
            frontend.compile_mika_section_with_catalog(&section, Arc::clone(&catalog)),
        ));
        if let Some(body) = section.body() {
            for name in named_scopes(body.syntax()) {
                let program = frontend.compile_named_mika_scope_with_catalog(
                    &section,
                    &name,
                    Arc::clone(&catalog),
                );
                candidates.push((
                    section.scope_id(),
                    owner_label.clone(),
                    CanonicalRenderScope::Named(name),
                    program,
                ));
            }
        }
    }
    let mut rendered_results = Vec::new();
    let mut scopes = Vec::new();
    let mut diagnostics = Vec::new();
    let mut visible_values = Vec::new();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut empty_document = None;
    for (owner, owner_label, scope, candidate) in candidates {
        let name = match &scope {
            CanonicalRenderScope::Root => "root",
            CanonicalRenderScope::Named(name) => name,
        };
        let mut record = json!({"owner":owner_label,"name":name,"stages":{"parsing":"completed"},"diagnostics":[]});
        let program = match candidate {
            Ok(program) => program,
            Err(error) if error.code == "source-semantics/empty-document" => {
                empty_document
                    .get_or_insert_with(|| semantic_error(error, &document.snapshot().source));
                continue;
            }
            Err(error) => {
                record["stages"]["semantic_checking"] = json!("rejected");
                record["diagnostics"] = json!([semantic_error(error, &document.snapshot().source)]);
                diagnostics.extend(record["diagnostics"].as_array().unwrap().iter().cloned());
                scopes.push(record);
                continue;
            }
        };
        if let Some(values) = inspect_program(&program, &catalog, &mut record) {
            let snapshots = values
                .iter()
                .map(RuntimeValueSnapshot::try_from)
                .collect::<Result<Vec<_>, _>>();
            let capture =
                snapshots
                    .map_err(|error| error.display_message())
                    .and_then(|snapshots| {
                        CanonicalScopeResults::from_values(
                            owner,
                            scope.clone(),
                            &program,
                            &snapshots,
                        )
                        .map_err(|error| error.to_string())
                    });
            match capture {
                Ok(capture) => rendered_results.push(capture),
                Err(error) => {
                    record["diagnostics"] = json!([{"phase":"document_rendering","message":error}])
                }
            }
            for binding in program
                .document_outputs()
                .iter()
                .filter(|binding| binding.visible)
            {
                let mut value = describe(&values[binding.output as usize]);
                value["scope"] = json!(name);
                value["owner"] = json!(owner_label);
                let anchor = program.source_map().outputs[binding.output as usize];
                value["range"] = json!([anchor.range.start.0, anchor.range.end.0]);
                value["presentation"] = json!(format!("{:?}", binding.kind));
                visible_values.push(value);
            }
        }
        diagnostics.extend(record["diagnostics"].as_array().unwrap().iter().cloned());
        if let Some(scope_inputs) = record["inputs"].as_array() {
            inputs.extend(scope_inputs.iter().map(|input| {
                let mut input = input.clone();
                input["scope"] = json!(name);
                input
            }));
        }
        if let Some(scope_outputs) = record["outputs"].as_array() {
            outputs.extend(scope_outputs.iter().cloned());
        }
        scopes.push(record);
    }
    if scopes.is_empty()
        && let Some(information) = empty_document
    {
        diagnostics.push(information);
    }
    for stage in [
        "semantic_checking",
        "artifact_construction",
        "activation",
        "execution",
    ] {
        let state = if scopes.is_empty() {
            "executable source required"
        } else if scopes
            .iter()
            .all(|scope| scope["stages"][stage] == "completed")
        {
            "completed"
        } else if scopes
            .iter()
            .any(|scope| scope["stages"][stage] == "awaiting external inputs")
        {
            "awaiting external inputs"
        } else {
            "incomplete"
        };
        result["stages"][stage] = json!(state);
    }
    if diagnostics.is_empty() && result["stages"]["execution"] == "completed" {
        match CanonicalDocumentRenderer.render_editor_html(&syntax, &rendered_results) {
            Ok(html) => result["document_html"] = json!(html),
            Err(error) => {
                diagnostics.push(json!({"phase":"document_rendering","message":error.to_string()}))
            }
        }
    }
    visible_values.sort_by_key(|value| value["range"][0].as_u64().unwrap_or(0));
    result["scopes"] = json!(scopes);
    result["diagnostics"] = json!(diagnostics);
    result["values"] = json!(visible_values);
    result["inputs"] = json!(inputs);
    result["outputs"] = json!(outputs);
    result
}

/// One activated interval-state instance. Rejected inputs never replace it.
#[wasm_bindgen]
pub struct TypePublicationSession {
    artifact: ProgramArtifact,
    instance: ReactiveInstance,
}
#[wasm_bindgen]
impl TypePublicationSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<TypePublicationSession, JsValue> {
        Self::create().map_err(|error| JsValue::from_str(&error))
    }
    pub fn source(&self) -> String {
        INTERVAL_SOURCE.to_owned()
    }
    pub fn snapshot(&self) -> String {
        self.snapshot_value().to_string()
    }
    pub fn update(&mut self, decimal: &str) -> String {
        self.update_value(decimal).to_string()
    }
}
impl TypePublicationSession {
    fn create() -> Result<Self, String> {
        let document = SourceDocument::parse_resolved(
            "audit:interval-session",
            Revision(0),
            INTERVAL_SOURCE,
            ParseConfig::default(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let catalog = mech_stdlib::source_catalog();
        let body = SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
            width: IntegerWidth::W8,
            lower: 1,
            upper: 10,
            upper_inclusive: false,
        });
        let artifact = CanonicalSourceFrontend
            .compile_document_with_catalog_and_input_schemas(
                &document.document(),
                Arc::clone(&catalog),
                BTreeMap::from([("signal".to_owned(), body)]),
            )
            .map_err(|e| format!("{e:?}"))?
            .compile_artifact()
            .map_err(|e| format!("{e:?}"))?;
        let bytes = mech_engine::encode_program_artifact_bytecode_v1(&artifact)
            .map_err(|e| format!("{e:?}"))?;
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bytes)
            .map_err(|e| format!("{e:?}"))?;
        let instance = activate(
            ReactiveInstanceId::new(0xA04, 1),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let mut session = Self { artifact, instance };
        let initial = session.update_value("2");
        if initial["outcome"] != "accepted" {
            return Err(initial.to_string());
        }
        Ok(session)
    }
    fn snapshot_value(&self) -> Json {
        json!({"source":INTERVAL_SOURCE,"instance":"0xa04:1 (this object; no replacement on update)",
            "epoch":format!("{:?}",self.instance.published_epoch()),"state_hash":self.instance.published_state_hash().to_string(),
            "output":match self.instance.copied_output(0){Ok(value)=>describe(&value),Err(error)=>json!({"error":format!("{error:?}")})}})
    }
    fn update_value(&mut self, decimal: &str) -> Json {
        let before = self.snapshot_value();
        let update = (|| -> Result<(), (String, String)> {
            let byte = decimal
                .parse::<u8>()
                .map_err(|e| ("external_input_admission".into(), e.to_string()))?;
            let mut builder = SchemaTableBuilder::new();
            let schema = SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::UnsignedInteger(IntegerWidth::W8),
            }
            .finalize()
            .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let handle = builder
                .insert(schema)
                .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let built = builder
                .finish()
                .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let schema = built
                .resolve(handle)
                .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let (schemas, _) = built.into_parts();
            let base = ValueDraft {
                schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::U8(byte),
            }
            .finalize(&SnapshotValidationContext::new(&schemas))
            .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            let input = &self.instance.plan.inputs[0];
            let admitted = base
                .rebind(input.schema, &input.shape, self.artifact.schemas())
                .map_err(|e| ("external_input_admission".into(), format!("{e:?}")))?;
            self.instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot: input.slot,
                    value: &admitted,
                }])
                .and_then(|prepared| prepared.publish())
                .map_err(|e| ("execution_or_publication".into(), format!("{e:?}")))?;
            Ok(())
        })();
        let after = self.snapshot_value();
        match update {
            Ok(()) => {
                json!({"input":decimal,"outcome":"accepted","phase":"correct_publication","before":before,"after":after})
            }
            Err((phase, message)) => {
                json!({"input":decimal,"outcome":"rejected","phase":phase,"message":message,"accepted_state_unchanged":before==after,"before":before,"after":after})
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_preview_keeps_unfenced_table_as_source() {
        let result = inspect_document("|x<f64> y<f64>| 1 2 | 3 4 |\n");
        assert_eq!(result["stages"]["execution"], "completed", "{result}");
        assert_eq!(result["values"][0]["kind"], "table");
        let html = result["document_html"].as_str().unwrap();
        assert!(html.contains("<pre class='mech-code'"), "{html}");
        assert_eq!(html.matches("<table").count(), 0, "{html}");
        assert_eq!(html.matches("mech-program-output").count(), 0, "{html}");
    }

    #[test]
    fn document_preview_presents_table_through_fence_channel() {
        for language in ["mech", "mech:example"] {
            let source = format!("```{language}\n|x<f64> y<f64>| 1 2 | 3 4 |\n```\n\n9\n");
            let result = inspect_document(&source);
            assert_eq!(result["diagnostics"], json!([]), "{result}");
            let html = result["document_html"].as_str().unwrap();
            assert_eq!(
                html.matches("<table class='mech-table'>").count(),
                1,
                "{html}"
            );
            assert_eq!(
                html.matches("<figcaption class='mech-output'>").count(),
                1,
                "{html}"
            );
            assert_eq!(html.matches("mech-program-output").count(), 0, "{html}");
            assert!(
                result["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|value| value["text"] == "9"),
                "{result}"
            );
        }
    }

    #[test]
    fn document_preview_presents_table_at_explicit_inline_reference() {
        let result =
            inspect_document("data := |x<f64> y<f64>| 1 2 | 3 4 |\n\nThe data is {data}.\n\n42\n");
        assert_eq!(result["diagnostics"], json!([]), "{result}");
        let html = result["document_html"].as_str().unwrap();
        assert_eq!(
            html.matches("<table class='mech-table'>").count(),
            1,
            "{html}"
        );
        assert_eq!(html.matches("mech-program-output").count(), 0, "{html}");
        assert!(html.contains("The data is "), "{html}");
        assert!(
            result["values"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["text"] == "42"),
            "{result}"
        );
    }

    #[test]
    fn document_named_fences_publish_independent_values_in_place() {
        let source = "Calculation\r\n===========\r\n\r\nThe result is published by the final expression.\r\n\r\n```mech:foo\r\nanswer := 123\r\n```\r\n\r\n```mech:bar\r\nanswer := 456\r\n```\r\n\r\n1. Section One\r\n------------------------------\r\n\r\nThis is the first section.\r\n";
        let result = inspect_document(source);
        assert_eq!(result["stages"]["execution"], "completed", "{result}");
        assert_eq!(result["diagnostics"], json!([]), "{result}");
        assert_eq!(result["scopes"].as_array().unwrap().len(), 2);
        assert_eq!(result["values"][0]["scope"], "foo");
        assert_eq!(result["values"][0]["text"], "123");
        assert_eq!(result["values"][1]["scope"], "bar");
        assert_eq!(result["values"][1]["text"], "456");
        let html = result["document_html"].as_str().unwrap();
        assert_eq!(html.matches("<figcaption class='mech-output'>").count(), 2);
        assert_eq!(
            html.matches("<div class='mech-output-kind'>f64</div>")
                .count(),
            2
        );
        assert!(html.contains("data-mech-scope='foo'"), "{html}");
        assert!(html.contains("This is the first section."), "{html}");
        assert!(html.contains("data-mech-start="), "{html}");
    }

    #[test]
    fn document_scopes_share_repeated_fences_and_reset_each_run() {
        let source = "answer := 9\n\n```mech:foo\n~answer := 10\nanswer += 1\nanswer\n```\n\n```mech:bar\nanswer := 20\n```\n\n```mech:foo\nanswer += 2\nanswer\n```\n";
        for _ in 0..2 {
            let result = inspect_document(source);
            assert_eq!(result["diagnostics"], json!([]), "{result}");
            let values = result["values"].as_array().unwrap();
            assert_eq!(
                values
                    .iter()
                    .map(|value| value["text"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                ["9", "11", "20", "13"]
            );
            assert_eq!(
                result["document_html"]
                    .as_str()
                    .unwrap()
                    .matches("<figcaption class='mech-output'>")
                    .count(),
                3
            );
        }
    }

    #[test]
    fn document_named_scope_uses_host_function_catalog() {
        let result = inspect_document("```mech:foo\nanswer := math/sub(right: 3, left: 10)\n```\n");
        assert_eq!(result["diagnostics"], json!([]), "{result}");
        assert_eq!(result["values"][0]["text"], "7");
    }

    #[test]
    fn document_named_semantic_errors_keep_source_ranges() {
        let source =
            "```mech:foo\nanswer := 10⟨u8:1..10⟩\n```\n\n```mech:bar\nanswer := 456\n```\n";
        let result = inspect_document(source);
        assert_eq!(
            result["diagnostics"][0]["code"], "source-semantics/integer-interval-violation",
            "{result}"
        );
        let range = result["diagnostics"][0]["range"].as_array().unwrap();
        let selected =
            &source[range[0].as_u64().unwrap() as usize..range[1].as_u64().unwrap() as usize];
        assert!(selected.contains("10"), "{selected}");
        assert_eq!(result["values"][0]["scope"], "bar");
        assert_eq!(result["values"][0]["text"], "456");
        assert!(result["document_html"].is_null());
    }

    #[test]
    fn document_reserved_fences_and_output_settings_are_preserved() {
        let source = "```mech:hidden\nanswer := 1\n```\n\n```mech:disabled\nanswer := 999\n```\n\n```rust\nthis is source text\n```\n\n```mech:foo{output: false}\nanswer := 2\n```\n\n```mech:bar\nanswer := {x: 3, y: 4}\n```\n";
        let result = inspect_document(source);
        assert_eq!(result["diagnostics"], json!([]), "{result}");
        assert_eq!(result["scopes"].as_array().unwrap().len(), 3);
        assert_eq!(result["values"].as_array().unwrap().len(), 1);
        let html = result["document_html"].as_str().unwrap();
        assert_eq!(html.matches("<figcaption class='mech-output'>").count(), 1);
        assert!(html.contains("mech-record"), "{html}");
        assert!(html.contains("answer := 999"), "{html}");
        assert!(!html.contains("answer := 1"), "{html}");
    }

    #[test]
    fn document_inline_values_use_root_bindings_and_named_fences_stay_isolated() {
        let result = inspect_document(
            "answer := 42\n\nThe answer is {answer}.\n\n```mech:foo\nanswer := 123\n```\n",
        );
        assert_eq!(result["diagnostics"], json!([]), "{result}");
        let html = result["document_html"].as_str().unwrap();
        assert!(html.contains("The answer is "), "{html}");
        assert!(
            html.contains("The answer is <span class='mech-value'>42</span>."),
            "{html}"
        );
        assert_eq!(
            result["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value["text"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["42", "123"]
        );
    }

    #[cfg(feature = "mika")]
    #[test]
    fn document_mika_named_scopes_keep_local_owners() {
        let result = inspect_document(
            "```mech:foo\nanswer := 123\n```\n\n~∘~⸢```mech:foo\nanswer := 456\n```\n⸥\n",
        );
        assert_eq!(result["diagnostics"], json!([]), "{result}");
        assert_eq!(result["scopes"].as_array().unwrap().len(), 2);
        assert_ne!(result["scopes"][0]["owner"], result["scopes"][1]["owner"]);
        let html = result["document_html"].as_str().unwrap();
        assert_eq!(html.matches("<figcaption class='mech-output'>").count(), 2);
        assert_eq!(result["values"][0]["text"], "123");
        assert_eq!(result["values"][1]["text"], "456");
    }

    #[test]
    fn document_without_active_source_has_information_only() {
        let result = inspect_document(
            "Calculation\n===========\n\nThis is prose.\n\n```mech:disabled\nanswer := 123\n```\n",
        );
        assert_eq!(result["diagnostics"][0]["severity"], "info");
        assert_eq!(result["diagnostics"].as_array().unwrap().len(), 1);
        assert_eq!(result["scopes"], json!([]));
    }

    #[test]
    fn same_instance_rejects_and_recovers() {
        let mut session = TypePublicationSession::create().unwrap();
        assert_eq!(session.update_value("9")["outcome"], "accepted");
        let rejected = session.update_value("10");
        assert_eq!(rejected["outcome"], "rejected");
        assert_eq!(rejected["accepted_state_unchanged"], true);
        assert_eq!(session.update_value("3")["outcome"], "accepted");
    }
    #[cfg(feature = "u128")]
    #[test]
    fn fixtures_compare_independent_scalar_values() {
        let mut records = Vec::new();
        for (source, expected) in [
            ("answer := 1⟨u8:1..10⟩\nanswer\n", "1"),
            ("answer := 9⟨u8:1..10⟩\nanswer\n", "9"),
            ("answer := 10⟨u8:1..=10⟩\nanswer\n", "10"),
            ("answer := math/sub(right: 3, left: 10)\nanswer\n", "7"),
            (
                "answer := 340282366920938463463374607431768211455u128\nanswer\n",
                "340282366920938463463374607431768211455",
            ),
        ] {
            let result = inspect(source);
            assert!(
                result["values"].as_array().is_some_and(|values| values
                    .iter()
                    .any(|value| value["scalar_text"] == expected)),
                "{result}"
            );
            records.push(result);
        }
        println!("NATIVE_TYPE_FIXTURES={}", json!(records));
    }

    #[test]
    fn semantic_interval_rejection_preserves_source_anchor() {
        let result = inspect("10⟨u8:1..10⟩\n");
        assert_eq!(
            result["diagnostics"][0]["code"],
            "source-semantics/integer-interval-violation"
        );
        assert!(result["diagnostics"][0]["range"].is_array());
        assert_eq!(
            result["product_diagnostic"]["semantic"]["range"],
            result["diagnostics"][0]["range"]
        );
        assert!(result["product_diagnostic"]["presentation_range"].is_string());
    }

    #[test]
    fn inspection_exports_canonical_record_html() {
        let result = inspect("point := {x: 1, y: 2}\npoint\n");
        assert_eq!(result["stages"]["execution"], "completed", "{result}");
        let value = &result["values"][0];
        let html = value["html"].as_str().expect("canonical HTML projection");
        assert!(html.contains("<table class='mech-record'>"), "{html}");
        assert!(html.contains("<th scope='row'>x</th>"), "{html}");
        assert!(html.contains("<th scope='row'>y</th>"), "{html}");
        assert!(value["text"].is_string());
        assert_eq!(value["kind"], "record");
        assert!(value["value"].is_string());
    }
}
