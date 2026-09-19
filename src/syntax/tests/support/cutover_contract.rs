//! Structural acceptance of linked preparation and final removal evidence.
use super::{consumers, repository_root};
use std::{collections::BTreeMap, fs};

const READINESS_HEADER: &str = "consumer-id\tintegration-owner\tfinal-entry-point\towning-data-type\tinput-finality-policy\tdiagnostic-policy\tfeature-distribution-profiles\tpositive-witness\tnegative-witness\tfinal-c-change\tremoval-dependencies\tblocker-owner\tstatus\tevidence-sha\tevidence-result";
const REMOVAL_HEADER: &str = "removal-id\tkind\tsource-path\tsurface\tpreparation-owner\tc-action\ttest-disposition\treplacement-contract\tblocker\tstatus\tevidence-sha\tevidence-result\tcompletion-action";

fn evidence(sha: &str, result: &str) -> Result<(), String> {
    if sha.len() != 40
        || !sha.bytes().all(|byte| byte.is_ascii_hexdigit())
        || sha.bytes().all(|byte| byte == b'0')
        || result != "pass"
    {
        return Err("completed evidence requires a commit SHA and pass result".into());
    }
    Ok(())
}

fn removal_state(
    action: &str,
    exists: bool,
    status: &str,
    sha: &str,
    result: &str,
) -> Result<bool, String> {
    match (action, status) {
        ("delete-path" | "replace-surface" | "retain-verified", "inventoried") if exists => {
            Ok(false)
        }
        // Routing may move during preparation; it does not close deletion evidence.
        ("replace-surface", "routed") if exists => Ok(false),
        ("delete-path", "removed") if !exists => {
            evidence(sha, result)?;
            Ok(true)
        }
        // An API/route replacement retains its containing file; product and
        // zero-route checks prove the removed surface on the cited candidate.
        ("replace-surface", "replaced") | ("retain-verified", "retained") if exists => {
            evidence(sha, result)?;
            Ok(true)
        }
        _ => Err(format!(
            "invalid removal action/status/path state: {action}/{status}, exists={exists}"
        )),
    }
}

pub(super) fn routed_parser_consumers() -> std::collections::BTreeSet<String> {
    rows(
        &fs::read_to_string(
            repository_root().join("docs/design/grammar-audit/s8-removal-manifest.tsv"),
        )
        .unwrap(),
        REMOVAL_HEADER,
    )
    .into_iter()
    .filter_map(|row| {
        if row[1] == "route" && matches!(row[9].as_str(), "routed" | "replaced") {
            assert_eq!(row[12], "replace-surface");
            Some(row[0].strip_prefix("route:").unwrap().to_owned())
        } else {
            None
        }
    })
    .collect()
}

#[test]
fn routed_consumers_do_not_claim_deletion_qualification() {
    assert_eq!(
        removal_state(
            "replace-surface",
            true,
            "routed",
            "pending",
            "not yet qualified"
        ),
        Ok(false)
    );
    assert!(
        removal_state(
            "delete-path",
            true,
            "routed",
            "pending",
            "not yet qualified"
        )
        .is_err()
    );
    let manifest = BTreeMap::from([("route:example".to_owned(), false)]);
    assert!(dependencies("demonstrated", "route:example", &manifest).is_err());
}

fn dependencies(
    status: &str,
    value: &str,
    manifest: &BTreeMap<String, bool>,
) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for id in value.split(';').map(str::trim) {
        if !seen.insert(id) {
            return Err(format!("duplicate dependency: {id}"));
        }
        let closed = manifest
            .get(id)
            .ok_or_else(|| format!("unknown removal dependency: {id}"))?;
        if status == "demonstrated" && !closed {
            return Err(format!("demonstrated consumer has open dependency: {id}"));
        }
    }
    Ok(())
}

fn rows(source: &str, header: &str) -> Vec<Vec<String>> {
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(header));
    let width = header.split('\t').count();
    lines
        .map(|line| {
            let row = line.split('\t').map(str::to_owned).collect::<Vec<_>>();
            assert_eq!(row.len(), width);
            assert!(row.iter().all(|value| !value.is_empty()));
            row
        })
        .collect()
}

#[test]
fn cutover_readiness_and_removal_routes_extend_the_frozen_census() {
    let root = repository_root();
    let census = consumers();
    let removals = rows(
        &fs::read_to_string(root.join("docs/design/grammar-audit/s8-removal-manifest.tsv"))
            .unwrap(),
        REMOVAL_HEADER,
    );
    let readiness = rows(
        &fs::read_to_string(root.join("docs/design/grammar-audit/s8-consumer-readiness.tsv"))
            .unwrap(),
        READINESS_HEADER,
    );
    let mut manifest = BTreeMap::new();
    let mut routes = BTreeMap::new();
    for row in removals {
        assert!(matches!(row[4].as_str(), "A" | "B"));
        assert!(matches!(
            row[6].as_str(),
            "retain-unchanged" | "retarget" | "remove-retired-detail"
        ));
        let closed = removal_state(
            &row[12],
            root.join(&row[2]).exists(),
            &row[9],
            &row[10],
            &row[11],
        )
        .unwrap_or_else(|error| panic!("{}: {error}", row[0]));
        assert!(manifest.insert(row[0].clone(), closed).is_none());
        if let Some(id) = row[0].strip_prefix("route:") {
            assert_eq!(row[1], "route");
            assert_eq!(row[2], census[id].source_path);
            assert_eq!(row[3], census[id].caller);
            routes.insert(id.to_owned(), row[4].clone());
        }
    }
    // These transitive public routes/cache owners are outside the Nom call count.
    for id in [
        "prototype-fragment",
        "prototype-fragment-exports",
        "module-store-cache",
        "module-store-transfer",
        "test:src/syntax/tests/document_fragments.rs",
        "test:src/syntax/tests/canonical_grammar_fragments.rs",
        "test:src/runtime/src/store.rs",
    ] {
        assert!(
            manifest.contains_key(id),
            "missing transitive removal owner: {id}"
        );
    }
    let mut owners = BTreeMap::new();
    let mut counts = BTreeMap::<String, (usize, usize)>::new();
    for row in readiness {
        let consumer = census
            .get(&row[0])
            .expect("readiness row absent from census");
        assert!(matches!(row[1].as_str(), "A" | "B"));
        assert!(owners.insert(row[0].clone(), row[1].clone()).is_none());
        let count = counts.entry(row[1].clone()).or_default();
        count.0 += 1;
        count.1 += consumer.calls;
        assert!(matches!(
            row[12].as_str(),
            "planned" | "prepared" | "demonstrated"
        ));
        if row[12] != "planned" {
            evidence(&row[13], &row[14]).unwrap();
        }
        dependencies(&row[12], &row[10], &manifest)
            .unwrap_or_else(|error| panic!("{}: {error}", row[0]));
        assert!(
            row[10]
                .split(';')
                .map(str::trim)
                .any(|id| id == format!("route:{}", row[0])),
            "missing own route dependency"
        );
    }
    assert_eq!(
        owners.keys().collect::<Vec<_>>(),
        census.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        counts,
        BTreeMap::from([("A".into(), (10, 11)), ("B".into(), (17, 18))])
    );
    assert_eq!(routes, owners);
}

#[test]
fn historical_deletions_require_absence_and_pass_evidence() {
    let sha = "1234567890abcdef1234567890abcdef12345678";
    assert_eq!(
        removal_state("delete-path", false, "removed", sha, "pass"),
        Ok(true)
    );
    assert!(removal_state("delete-path", true, "removed", sha, "pass").is_err());
    assert!(removal_state("delete-path", false, "inventoried", "pending", "pending").is_err());
    assert!(removal_state("delete-path", false, "removed", "pending", "pass").is_err());
    assert!(removal_state("delete-path", false, "removed", sha, "failed").is_err());
    assert!(removal_state("delete-path", true, "unknown", sha, "pass").is_err());
}

#[test]
fn replacement_evidence_keeps_the_container_but_does_not_allow_pending_results() {
    let sha = "1234567890abcdef1234567890abcdef12345678";
    assert_eq!(
        removal_state("replace-surface", true, "replaced", sha, "pass"),
        Ok(true)
    );
    assert!(removal_state("replace-surface", false, "replaced", sha, "pass").is_err());
    assert!(removal_state("replace-surface", true, "replaced", sha, "pending").is_err());
}

#[test]
fn demonstrated_consumers_cannot_hide_missing_or_open_removal_dependencies() {
    let mut manifest = BTreeMap::from([("route:sample".into(), true), ("cache".into(), false)]);
    assert!(dependencies("planned", "unknown", &manifest).is_err());
    assert!(dependencies("demonstrated", "route:sample; cache", &manifest).is_err());
    assert!(dependencies("prepared", "route:sample; cache", &manifest).is_ok());
    manifest.insert("cache".into(), true);
    assert!(dependencies("demonstrated", "route:sample; cache", &manifest).is_ok());
    assert!(dependencies("demonstrated", "route:sample; cache; cache", &manifest).is_err());
    assert!(dependencies("demonstrated", "", &manifest).is_err());
}

// This audits test/example/helper files too; the frozen production census
// deliberately excludes cfg(test), examples and their direct parser helpers.
fn has_retiring_reference(source: &str) -> bool {
    use syn::visit::Visit;
    fn retiring_import(tree: &syn::UseTree, prefix: &[String]) -> bool {
        match tree {
            syn::UseTree::Path(path) => {
                let mut prefix = prefix.to_vec();
                prefix.push(path.ident.to_string());
                retiring_import(&path.tree, &prefix)
            }
            syn::UseTree::Group(group) => {
                group.items.iter().any(|tree| retiring_import(tree, prefix))
            }
            leaf => {
                let mut path = prefix.to_vec();
                match leaf {
                    syn::UseTree::Name(name) => path.push(name.ident.to_string()),
                    syn::UseTree::Rename(name) => path.push(name.ident.to_string()),
                    syn::UseTree::Glob(_) => {}
                    _ => unreachable!(),
                }
                (path.first().is_some_and(|name| name == "mech_syntax")
                    && !path.get(1).is_some_and(|name| name == "document"))
                    || path == ["mech_core", "Program"]
                    || path == ["mech_core", "nodes", "Program"]
            }
        }
    }
    #[derive(Default)]
    struct References(bool);
    impl<'ast> Visit<'ast> for References {
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            self.0 |= retiring_import(&item.tree, &[]);
        }
        fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
            self.0 |= path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "Program");
            syn::visit::visit_type_path(self, path);
        }
    }
    let mut references = References::default();
    if let Ok(file) = syn::parse_file(source) {
        references.visit_file(&file);
    }
    let tokens = super::rust_tokens(source);
    references.0
        || tokens.iter().any(|token| {
            matches!(
                token.text,
                "syntax_tree"
                    | "compile_tree"
                    | "compile_interactive_tree"
                    | "compile_tree_artifact"
                    | "compile_tree_artifact_with_inputs"
                    | "compile_tree_artifact_with_input_initializers"
                    | "evaluate_static_tree_symbols"
                    | "evaluate_static_tree_symbols_with_inputs"
                    | "compile_mixed_tree"
                    | "load_tree_program"
                    | "load_interactive_tree_program"
                    | "from_tree"
                    | "activate_tree"
            )
        })
        || tokens.windows(4).any(|path| {
            path[0].text == "mech_syntax"
                && path[1].text == ":"
                && path[2].text == ":"
                // Root grammar exports (including types and parser functions)
                // retire together; the canonical API lives under document.
                && path[3].text != "document"
        })
}

#[test]
fn direct_retiring_references_in_all_package_targets_have_removal_owners() {
    let root = repository_root();
    let removals = rows(
        &fs::read_to_string(root.join("docs/design/grammar-audit/s8-removal-manifest.tsv"))
            .unwrap(),
        REMOVAL_HEADER,
    );
    let paths = removals
        .iter()
        .map(|row| row[2].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let mut files = std::collections::BTreeSet::new();
    for package in super::local_packages(&root, super::workspace_metadata(&root)) {
        let directory = std::path::Path::new(package["manifest_path"].as_str().unwrap())
            .parent()
            .unwrap();
        for name in ["src", "tests", "examples", "benches"] {
            let path = directory.join(name);
            if path.is_dir() {
                super::visit_rust_files(&path, &mut files);
            }
        }
        // Also cover explicit targets outside the conventional directories.
        for target in package["targets"].as_array().unwrap() {
            files.insert(std::path::PathBuf::from(
                target["src_path"].as_str().unwrap(),
            ));
        }
    }
    let missing = files
        .into_iter()
        .filter_map(|path| {
            let source = fs::read_to_string(&path).unwrap();
            let relative = path.strip_prefix(&root).unwrap().to_str().unwrap();
            (has_retiring_reference(&source) && !paths.contains(relative))
                .then(|| relative.to_owned())
        })
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "retiring references lack removal owners: {missing:#?}"
    );
}

#[test]
fn supplemental_scan_includes_test_helpers_but_ignores_fixture_text() {
    assert!(has_retiring_reference(
        "#[cfg(test)] fn helper() { mech_syntax::parse(s); }"
    ));
    assert!(has_retiring_reference(
        "fn benchmark() { mech_syntax :: r#parser :: parse(s); }"
    ));
    assert!(has_retiring_reference("record.syntax_tree.clone()"));
    assert!(!has_retiring_reference(
        "// mech_syntax::parse(s)\nlet fixture = r#\"syntax_tree\"#;"
    ));
    assert!(!has_retiring_reference(
        "mech_syntax::document::parse_canonical_document(s)"
    ));
}

#[test]
fn readiness_includes_cross_boundary_payload_and_cache_handoffs() {
    let root = repository_root();
    let readiness = rows(
        &fs::read_to_string(root.join("docs/design/grammar-audit/s8-consumer-readiness.tsv"))
            .unwrap(),
        READINESS_HEADER,
    );
    for row in readiness {
        let id = row[0].as_str();
        let dependencies = row[10]
            .split(';')
            .map(str::trim)
            .collect::<std::collections::BTreeSet<_>>();
        let mut required = vec!["root-parser-exports"];
        if id == "bundle-web.project" || id.starts_with("wasm.") {
            required.extend(["bundle-payload", "browser-loader", "browser-features"]);
        }
        if id.starts_with("runtime.interactive-") || id.starts_with("wasm.") {
            required.extend(["interactive-cache", "wasm-repl-transfer"]);
        }
        if id.starts_with("runtime.program-") || id.starts_with("runtime.interactive-") {
            required.extend(["compiler-tree-apis", "runtime-tree-loading"]);
        }
        if id.starts_with("runtime.") || id == "serve.workspace-render" || id.starts_with("wasm.") {
            required.extend([
                "module-cache",
                "module-store-cache",
                "module-store-transfer",
                "module-builder-transfer",
                "workspace-cache",
                "workspace-load-transfer",
            ]);
        }
        for dependency in required {
            assert!(
                dependencies.contains(dependency),
                "{id} omits transitive dependency {dependency}"
            );
        }
    }
}

#[test]
fn completion_action_cannot_disguise_a_required_physical_deletion() {
    let sha = "1234567890abcdef1234567890abcdef12345678";
    // parser-nom must remain open while its implementation path exists.
    assert!(removal_state("delete-path", true, "replaced", sha, "pass").is_err());
    assert!(removal_state("delete-path", true, "retained", sha, "pass").is_err());
    assert!(removal_state("replace-surface", true, "replaced", sha, "pass").is_ok());
    assert!(removal_state("retain-verified", true, "retained", sha, "pass").is_ok());
    assert!(removal_state("retain-verified", false, "removed", sha, "pass").is_err());
    assert!(removal_state("unknown", true, "inventoried", "pending", "pending").is_err());
}

#[test]
fn supplemental_scan_covers_grouped_exports_and_typed_program_handoffs() {
    for source in [
        "pub use mech_syntax::{parse, parse_grammar, parser};",
        "use mech_syntax::{parser::{parse as read}};",
        "use mech_syntax::*;",
        "use mech_core::{nodes::{Program as Stored}};",
        "fn load(tree: &mech_core::Program) {}",
        "fn activate(tree: Program) {}",
        "fn load() { compiler.compile_interactive_tree(tree); }",
        "fn activate() { ResidentReplSession::from_tree(tree); }",
    ] {
        assert!(
            has_retiring_reference(source),
            "missed retiring handoff: {source}"
        );
    }
    for source in [
        "use mech_syntax::document::{DocumentStream, DocumentSyntax};",
        "fn lifetime() { let lifetime = MemoryLifetime::Program; }",
        "fn grouping() { let source = MechSourceCode::Program(Vec::new()); }",
    ] {
        assert!(
            !has_retiring_reference(source),
            "misclassified retained owner: {source}"
        );
    }
}

#[test]
fn supplemental_scan_covers_direct_grammar_exports() {
    for source in [
        "fn check() { mech_syntax::empty(input); }",
        "fn check() { mech_syntax::identifier(input); }",
        "fn check() { mech_syntax::module_import(input); }",
        "fn check(input: mech_syntax::ParseString) {}",
        "fn check() { mech_syntax :: r#empty(input); }",
    ] {
        assert!(
            has_retiring_reference(source),
            "missed retiring export: {source}"
        );
    }
}
