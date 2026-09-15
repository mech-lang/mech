#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]
//! Observational audit of frozen replacement contracts. No production implementation.
//! MECH_AUDIT_CASE selects one witness. MECH_AUDIT_REQUIRE_PASS=1 makes a
//! mismatch with the selected oracle fail the test. MECH_AUDIT_REQUIRE_CAPABILITY=1
//! selects the positive milestone oracle instead of current target rejection.
//! A matching rejection does not prove capability; an observational run is not a seal.
use mech_core::ReactiveInstanceId;
use mech_engine::resident::{ActivationFacts, activate};
use mech_runtime::{RuntimeBuilder, RuntimeValueSnapshot, SourceDocument};
use mech_syntax::document::{ParseConfig, Revision};
use serde_json::{Value, json};

fn audit_input_identity() {
    // Execution-time identity of the compiled audit inputs, independent of the
    // later recorder's filesystem. FNV-1a-64 is a deterministic stale-input
    // check; the recorder also records SHA-256 identities of the matched bytes.
    fn fingerprint(bytes: &[u8]) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }
    let harness = include_bytes!("s8_replacement_gap_audit.rs");
    let fixture =
        include_bytes!("../../../tests/fixtures/s8-replacement-audit/semantic-cases.json");
    println!(
        "AUDIT_IDENTITY {}",
        json!({"version":1,"algorithm":"fnv1a64",
            "harness_bytes":harness.len(),"harness_fingerprint":fingerprint(harness),
            "fixture_bytes":fixture.len(),"fixture_fingerprint":fingerprint(fixture)})
    );
}

#[test]
fn semantic_replacement_witnesses() {
    audit_input_identity();
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/s8-replacement-audit/semantic-cases.json"
    ))
    .unwrap();
    let filter = std::env::var("MECH_AUDIT_CASE").ok();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in cases {
        let id = case["id"].as_str().unwrap();
        if filter.as_deref().is_some_and(|filter| filter != id) {
            continue;
        }
        count += 1;
        let result = std::panic::catch_unwind(|| probe(&case));
        let (stage, detail) = match result {
            Ok(Ok(values)) => ("pass".to_owned(), json!(values)),
            Ok(Err((stage, message))) => (stage, json!(message)),
            Err(_) => ("panic".to_owned(), json!("unwound during probe")),
        };
        let require_capability = std::env::var_os("MECH_AUDIT_REQUIRE_CAPABILITY").is_some()
            && case["milestone_capability_group"].is_string();
        let expected_stage = if require_capability {
            "pass"
        } else {
            case["expected_rejection_stage"].as_str().unwrap_or("pass")
        };
        let contract_pass = stage == expected_stage
            && (require_capability
                || case["expected_rejection_detail"]
                    .as_str()
                    .is_none_or(|expected| detail.as_str() == Some(expected)));
        println!(
            "AUDIT {}",
            json!({"id":id,"stage":stage,"detail":detail,
            "expected_stage":expected_stage,"contract_status":if contract_pass {"pass"} else {"fail"},
            "milestone_capability_group":case["milestone_capability_group"],
            "positive_capability_executed":case["milestone_capability_group"].is_string() && stage == "pass"})
        );
        if !contract_pass {
            failures.push(id.to_owned());
        }
    }
    assert!(count > 0, "audit selector must execute a witness");
    println!(
        "AUDIT_SUMMARY {}",
        json!({"count":count,"failures":failures})
    );
    if std::env::var_os("MECH_AUDIT_REQUIRE_PASS").is_some() {
        assert!(failures.is_empty(), "replacement gaps: {failures:?}");
    }
}

fn probe(case: &Value) -> Result<Vec<Vec<String>>, (String, String)> {
    let error = |stage: &str, message: String| (stage.to_owned(), message);
    let source = case["source"].as_str().unwrap();
    let document = SourceDocument::parse_resolved(
        "audit:semantic",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .map_err(|e| error("syntax", format!("{e:?}")))?;
    document
        .index()
        .map_err(|e| error("index", format!("{e:?}")))?;
    let catalog = mech_stdlib::source_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .build_compiler()
        .map_err(|e| error("compiler-setup", format!("{e:?}")))?;
    let product = compiler
        .compile_document(&document)
        .map_err(|e| error("lowering", format!("{e:?}")))?;
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(product.bytecode())
        .map_err(|e| error("bytecode", format!("{e:?}")))?;
    let mut results = Vec::new();
    let expect_target_rejection = case["expected_rejection_stage"].as_str() == Some("activation")
        && std::env::var_os("MECH_AUDIT_REQUIRE_CAPABILITY").is_none();
    let mut rejected_artifacts = Vec::new();
    for artifact in [product.artifact(), &decoded] {
        let activation = activate(
            ReactiveInstanceId::new(0x58a, 0),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        );
        let mut instance = match activation {
            Err(e) if expect_target_rejection => {
                let detail = format!("{e:?}");
                if Some(detail.as_str()) != case["expected_rejection_detail"].as_str() {
                    return Err(error("wrong-target-rejection", detail));
                }
                rejected_artifacts.push(detail);
                continue;
            }
            Err(e) => return Err(error("activation", format!("{e:?}"))),
            Ok(_) if expect_target_rejection => {
                return Err(error(
                    "unexpected-target-admission",
                    "expected current target rejection".into(),
                ));
            }
            Ok(instance) => instance,
        };
        let mut values = Vec::new();
        for turn in 0..2 {
            instance
                .turn(&[])
                .map_err(|e| error("execution", format!("{e:?}")))?;
            let value = instance
                .copied_output(0)
                .map_err(|e| error("publication", format!("{e:?}")))?;
            let value = RuntimeValueSnapshot::from_value(value)
                .map_err(|e| error("snapshot", format!("{e:?}")))?
                .format_canonical_inline();
            if let Some(expected) = case["positive_milestone_expected"]
                .as_array()
                .or_else(|| case["expected"].as_array())
            {
                if value != expected[turn].as_str().unwrap() {
                    return Err(error(
                        "wrong-value",
                        format!("turn {turn}: expected {}, got {value}", expected[turn]),
                    ));
                }
            }
            values.push(value);
        }
        results.push(values);
    }
    if expect_target_rejection {
        assert_eq!(
            rejected_artifacts.len(),
            2,
            "both artifact representations must reject"
        );
        return Err(error("activation", rejected_artifacts.remove(0)));
    }
    if results[0] != results[1] {
        return Err(error("bytecode-value", "source and bytecode differ".into()));
    }
    Ok(results)
}

#[test]
fn compiler_entry_point_witnesses() {
    audit_input_identity();
    use mech_engine::ProgramArtifactCompilationProduct;
    use mech_runtime::ModuleBuildOptions;
    use mech_runtime::resolver::{InMemorySourceResolver, SourceRequest, SourceResolver};
    use std::collections::{BTreeMap, BTreeSet};
    let source = "answer := 42\nanswer\n";
    let doc = SourceDocument::parse_resolved(
        "memory:main.mec",
        Revision(0),
        source,
        ParseConfig::default(),
    )
    .unwrap();
    let mut failures = Vec::new();
    let options = || ModuleBuildOptions::new("audit", "v0.4", "native", &[], &[]);
    for route in [
        "document",
        "interactive-document",
        "canonical-source",
        "document-artifact",
        "canonical-source-artifact",
        "inputs",
        "initializers",
        "root",
        "resolved",
        "root-options",
        "resolved-options",
        "interactive-root",
        "interactive-resolved",
        "interactive-root-options",
        "interactive-resolved-options",
        "ordered-roots",
        "static",
        "static-inputs",
    ] {
        let mut resolver = InMemorySourceResolver::new();
        resolver.insert_string("main.mec", source).unwrap();
        let resolved = resolver
            .resolve(&SourceRequest::new("main.mec"))
            .unwrap()
            .unwrap();
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .source_resolver(resolver)
            .build_compiler()
            .unwrap();
        let artifact_product =
            |product: ProgramArtifactCompilationProduct| product.artifact().clone();
        let result = match route {
            "document" => compiler
                .compile_document(&doc)
                .map(|p| p.artifact().clone()),
            "interactive-document" => compiler
                .compile_interactive_document(&doc)
                .map(|p| p.artifact().clone()),
            "canonical-source" => compiler
                .compile_canonical_source(source)
                .map(|p| p.artifact().clone()),
            "document-artifact" => compiler
                .compile_document_artifact(&doc)
                .map(artifact_product),
            "canonical-source-artifact" => compiler
                .compile_canonical_source_artifact(source)
                .map(artifact_product),
            "inputs" => compiler
                .compile_document_artifact_with_inputs(&doc, &BTreeMap::new(), &BTreeSet::new())
                .map(artifact_product),
            "initializers" => compiler
                .compile_document_artifact_with_input_initializers(
                    &doc,
                    &BTreeMap::new(),
                    &BTreeSet::new(),
                )
                .map(|(p, _)| artifact_product(p)),
            "root" => compiler
                .compile_canonical_root(SourceRequest::new("main.mec"))
                .map(|p| p.artifact().clone()),
            "resolved" => compiler
                .compile_canonical_resolved_root(resolved)
                .map(|p| p.artifact().clone()),
            "root-options" => compiler
                .compile_canonical_root_with_options(SourceRequest::new("main.mec"), options())
                .map(|p| p.artifact().clone()),
            "resolved-options" => compiler
                .compile_canonical_resolved_root_with_options(resolved, options())
                .map(|p| p.artifact().clone()),
            "interactive-root" => compiler
                .compile_canonical_interactive_root(SourceRequest::new("main.mec"))
                .map(|p| p.artifact().clone()),
            "interactive-resolved" => compiler
                .compile_canonical_interactive_resolved_root(resolved)
                .map(|p| p.artifact().clone()),
            "interactive-root-options" => compiler
                .compile_canonical_interactive_root_with_options(
                    SourceRequest::new("main.mec"),
                    options(),
                )
                .map(|p| p.artifact().clone()),
            "interactive-resolved-options" => compiler
                .compile_canonical_interactive_resolved_root_with_options(resolved, options())
                .map(|p| p.artifact().clone()),
            "ordered-roots" => compiler
                .compile_canonical_roots(&[SourceRequest::new("main.mec")], options())
                .map(|p| p.artifact().clone()),
            "static" | "static-inputs" => {
                let values = if route == "static" {
                    compiler.evaluate_static_document_symbols(&doc, &["answer"])
                } else {
                    compiler.evaluate_static_document_symbols_with_inputs(
                        &doc,
                        &BTreeMap::new(),
                        &["answer"],
                    )
                };
                match values {
                    Ok(values) => {
                        assert!(values.contains_key("answer"));
                        println!("AUDIT_ROUTE {}", json!({"route":route,"stage":"pass"}));
                    }
                    Err(e) => {
                        failures.push(route);
                        println!(
                            "AUDIT_ROUTE {}",
                            json!({"route":route,"stage":"lowering","detail":format!("{e:?}")})
                        );
                    }
                }
                continue;
            }
            _ => unreachable!(),
        };
        match result {
            Ok(artifact) => {
                let catalog = mech_stdlib::source_catalog();
                let encoded = mech_engine::encode_program_artifact_bytecode_v1(&artifact).unwrap();
                let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
                for artifact in [&artifact, &decoded] {
                    let mut instance = activate(
                        ReactiveInstanceId::new(0x58c, 0),
                        artifact,
                        &catalog,
                        &ActivationFacts::default(),
                    )
                    .unwrap();
                    instance.turn(&[]).unwrap();
                    let value =
                        RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap())
                            .unwrap()
                            .format_canonical_inline();
                    assert_eq!(value, "42", "{route}");
                }
                println!("AUDIT_ROUTE {}", json!({"route":route,"stage":"pass"}));
            }
            Err(e) => {
                failures.push(route);
                println!(
                    "AUDIT_ROUTE {}",
                    json!({"route":route,"stage":"lowering","detail":format!("{e:?}")})
                );
            }
        }
    }
    assert!(
        failures.is_empty(),
        "basic compiler routes failed: {failures:?}"
    );
}

#[test]
fn ordered_transitive_explicit_root_witness() {
    audit_input_identity();
    use mech_runtime::ModuleBuildOptions;
    use mech_runtime::resolver::{InMemorySourceResolver, SourceRequest};
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string("dep.mec", "~counter := 0\ncounter += 1\n<+ counter\n")
        .unwrap();
    resolver
        .insert_string(
            "middle.mec",
            "+> ./dep.mec\nvalue := dep/counter\n<+ value\n",
        )
        .unwrap();
    resolver
        .insert_string(
            "main.mec",
            "+> ./middle.mec\nanswer := middle/value\nanswer\n",
        )
        .unwrap();
    let catalog = mech_stdlib::source_catalog();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(catalog.clone())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let result = compiler.compile_canonical_roots(
        &[
            SourceRequest::new("main.mec"),
            SourceRequest::new("dep.mec"),
        ],
        ModuleBuildOptions::new("audit", "v0.4", "native", &[], &[]),
    );
    let detail = match result {
        Err(e) => json!({"stage":"lowering","detail":format!("{e:?}")}),
        Ok(product) => {
            let mut instance = activate(
                ReactiveInstanceId::new(0x58d, 0),
                product.artifact(),
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            let mut values = Vec::new();
            for _ in 0..2 {
                instance.turn(&[]).unwrap();
                values.push(
                    RuntimeValueSnapshot::from_value(instance.copied_output(0).unwrap())
                        .unwrap()
                        .format_canonical_inline(),
                );
            }
            json!({"stage":if values == ["1","2"] {"pass"} else {"wrong-value"},"detail":values})
        }
    };
    println!("AUDIT_GRAPH {}", detail);
    if std::env::var_os("MECH_AUDIT_REQUIRE_PASS").is_some() {
        assert_eq!(detail["stage"], "pass");
    }
}

#[test]
fn source_catalog_census() {
    audit_input_identity();
    let catalog = mech_stdlib::source_catalog();
    let mut names = std::collections::BTreeSet::new();
    for export in catalog.all_exports() {
        if names.insert(export.canonical_name.clone()) {
            println!(
                "AUDIT_CATALOG {}",
                json!({"name":export.canonical_name,"types":format!("{:?}",catalog.source_type_declaration(&export.canonical_name))})
            );
        }
    }
    assert!(!names.is_empty());
}

#[cfg(feature = "serde")]
#[test]
fn browser_document_payload_witness() {
    audit_input_identity();
    let document = SourceDocument::parse_resolved(
        "audit:browser",
        Revision(0),
        "answer := 42\n",
        ParseConfig::default(),
    )
    .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler.compile_document(&document).unwrap();
    let bundle =
        mech_runtime::CanonicalProgramBundle::from_product("audit:browser", &document, &product)
            .unwrap();
    let encoded = bundle.encode().unwrap();
    assert!(mech_runtime::CanonicalProgramBundle::decode(&encoded, Some("answer := 42\n")).is_ok());
    // Exact decoder and destination type used by the frozen C WasmDocument path.
    // This is a transport frontier witness, not a browser execution claim.
    let retiring_decoder: Result<mech_core::nodes::Program, _> =
        mech_core::nodes::decode_and_decompress(&encoded);
    println!(
        "AUDIT_BROWSER {}",
        json!({"stage":if retiring_decoder.is_ok() {"pass"} else {"transport"},
        "detail":format!("{:?}",retiring_decoder.as_ref().err())})
    );
    if std::env::var_os("MECH_AUDIT_REQUIRE_PASS").is_some() {
        assert!(
            retiring_decoder.is_ok(),
            "canonical producer payload cannot reach the frozen C document loader"
        );
    }
}

#[test]
fn source_visibility_witnesses() {
    audit_input_identity();
    let cases = [
        (
            "internal-call",
            "answer := compare/max(1,2)\nanswer\n",
            false,
        ),
        (
            "module-without-import",
            "answer := math/cos(0f32)\nanswer\n",
            false,
        ),
        (
            "module-import",
            "+> math\nanswer := math/cos(0f32)\nanswer\n",
            true,
        ),
        (
            "aliased-import",
            "+> wave := math/cos\nanswer := wave(0f32)\nanswer\n",
            true,
        ),
        ("prelude-call", "answer := compare/eq(1,1)\nanswer\n", true),
        ("intrinsic", "answer := 1 > 0\nanswer\n", true),
    ];
    let mut failures = Vec::new();
    for (id, source, expected_admission) in cases {
        for route in ["frozen-shipping", "canonical"] {
            let mut compiler = RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .build_compiler()
                .unwrap();
            let result = if route == "canonical" {
                compiler.compile_canonical_source(source)
            } else {
                // A frozen-contract audit only. This is not a production fallback.
                compiler.compile_source(source)
            };
            let admitted = result.is_ok();
            let matches_contract = admitted == expected_admission;
            println!(
                "AUDIT_VISIBILITY {}",
                json!({"id":id,"route":route,
                "expected_admission":expected_admission,"admitted":admitted,
                "stage":if matches_contract {"pass"} else {"visibility"},
                "detail":result.err().map(|e|format!("{e:?}"))})
            );
            if !matches_contract {
                failures.push(format!("{id}/{route}"));
            }
        }
    }
    if std::env::var_os("MECH_AUDIT_REQUIRE_PASS").is_some() {
        assert!(
            failures.is_empty(),
            "visibility contract violations: {failures:?}"
        );
    }
}
