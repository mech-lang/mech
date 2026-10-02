#![cfg(feature = "formatter")]

#[cfg(has_file_wasm)]
#[path = "support/shim_contract.rs"]
mod shim_contract;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "mech-format-shim-{label}-{}-{sequence}-{nanos}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&path).expect("format shim temporary directory must be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        drop(std::fs::remove_dir_all(&self.path));
    }
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("shims")
        .join(name)
}

fn format_fixture_output(shim: Option<&Path>, stylesheet: Option<&Path>, output: &Path) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
    command
        .arg("format")
        .arg(fixture_path("all-slots.mec"))
        .arg("--html")
        .arg("--out")
        .arg(output);
    if let Some(shim) = shim {
        command.arg("--shim").arg(shim);
    }
    if let Some(stylesheet) = stylesheet {
        command.arg("--stylesheet").arg(stylesheet);
    }
    command
        .output()
        .expect("Cargo-built mech formatter must start")
}

fn assert_retained_execution_payload(html: &str) {
    let Some(tail) = html.split("data-mech-document-code>").nth(1) else {
        return;
    };
    let encoded = tail.split("</script>").next().unwrap().trim();
    let payload = mech_runtime::BrowserDocumentPayload::decode(encoded)
        .expect("formatter execution payload must retain source, never a detached tree");
    let document = mech_runtime::SourceDocument::parse_resolved(
        payload.root_specifier(),
        mech_syntax::document::Revision(0),
        payload.source(),
        mech_syntax::document::ParseConfig::default(),
    )
    .unwrap();
    document.index().unwrap();
    assert_eq!(
        payload.presentation_output_ids(),
        mech_runtime::canonical_document_presentation_output_ids(&document.document()).unwrap()
    );
    if let Some(bundle) = html.split("data-mech-document-sources>").nth(1) {
        use base64::Engine as _;
        let encoded = bundle.split("</script>").next().unwrap().trim();
        let bundle: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            payload.root_specifier(),
            bundle["rootSpecifier"].as_str().unwrap()
        );
        let root = bundle["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["specifier"] == payload.root_specifier())
            .unwrap();
        assert_eq!(payload.source(), root["source"].as_str().unwrap());
    }
}

fn format_fixture(shim: Option<&Path>, stylesheet: Option<&Path>, output: &Path) -> String {
    let output_result = format_fixture_output(shim, stylesheet, output);
    assert!(
        output_result.status.success(),
        "mech format failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output_result.stdout),
        String::from_utf8_lossy(&output_result.stderr),
    );
    let html =
        std::fs::read_to_string(output).expect("formatter must write the requested HTML file");
    assert_retained_execution_payload(&html);
    html
}

#[test]
fn mech_format_static_custom_shim_emits_no_runtime_assets() {
    let directory = TestDirectory::new("static-custom");
    let output = directory.path().join("static.html");
    let html = format_fixture(
        Some(&fixture_path("static-no-controller.html")),
        None,
        &output,
    );
    assert!(html.contains("static-document"));
    assert!(!directory.path().join("_mech/pkg/mech_wasm.js").exists());
    assert!(
        !directory
            .path()
            .join("_mech/pkg/mech_wasm_bg.wasm")
            .exists()
    );
}

#[test]
fn mech_format_static_custom_shim_is_ready_and_repeated_sections_are_navigable() {
    let directory = TestDirectory::new("static-status-and-anchors");
    let input = directory.path().join("document.mec");
    let shim = directory.path().join("custom.html");
    let output = directory.path().join("formatted.html");
    std::fs::write(
        &input,
        "1. First\n---------\nFirst body with [BOOK] and [^note].\n\n1. Second\n----------\nSecond body.\n\n[^note]: A footnote.\n\n[BOOK]: A reference.\n",
    )
    .unwrap();
    for (attribute, controller) in [
        (
            "data-mech-document-status='loading'",
            "data-mech-document-controller",
        ),
        (
            "DATA-MECH-DOCUMENT-STATUS = loading",
            "data-mech-document-controller='document'",
        ),
        (
            "data-mech-document-status\n=\t\"loading\"",
            "DATA-MECH-DOCUMENT-CONTROLLER\n= \"document\"",
        ),
    ] {
        std::fs::write(
            &shim,
            format!("<aside>{{{{SECTION1}}}}</aside><nav>{{{{TOC}}}}</nav><main {controller} data-mech-document-controller-extra='keep' {attribute}>{{{{CONTENT}}}}{{{{FOOTNOTES}}}}{{{{CITED}}}}</main><aside>{{{{CONTENT}}}}{{{{FOOTNOTES}}}}{{{{CITED}}}}</aside>"),
        )
        .unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_mech"))
            .arg("format")
            .arg(&input)
            .args(["--html", "--shim"])
            .arg(&shim)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let html = std::fs::read_to_string(&output).unwrap();
        assert!(
            html.contains("data-mech-document-status=\"ready\""),
            "{html}"
        );
        assert!(
            html.contains("data-mech-document-controller-extra='keep'"),
            "{html}"
        );
        assert!(!html.contains(&format!("{controller} ")), "{html}");
        let anchors = html
            .split("href='#")
            .skip(1)
            .map(|link| link.split_once('\'').unwrap().0)
            .collect::<Vec<_>>();
        let anchors = anchors
            .into_iter()
            .filter(|anchor| !anchor.starts_with("footnote-") && !anchor.starts_with("reference-"))
            .collect::<Vec<_>>();
        assert_eq!(anchors.len(), 2, "{html}");
        assert_ne!(anchors[0], anchors[1], "{html}");
        for anchor in anchors {
            assert_eq!(html.matches(&format!("id='{anchor}'")).count(), 1, "{html}");
        }
        assert!(!directory.path().join("_mech").exists());
    }
}

#[test]
fn mech_format_custom_controller_literal_module_owns_runtime_assets() {
    let directory = TestDirectory::new("literal-module");
    let output = directory.path().join("literal.html");
    let html = format_fixture(
        Some(&fixture_path("controller-literal-module.html")),
        None,
        &output,
    );

    assert!(html.contains("data-mech-wasm-module=\"https://cdn.example.test/mech_wasm.js\"",));
    assert!(html.contains("data-mech-document-sources"));
    assert!(!directory.path().join("_mech/pkg").exists());
}

#[test]
fn mech_format_controller_without_module_location_fails() {
    let directory = TestDirectory::new("missing-module-location");
    let output = directory.path().join("missing.html");
    let result = format_fixture_output(
        Some(&fixture_path("controller-missing-module.html")),
        None,
        &output,
    );

    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr)
            .contains("does not provide {{WASM_MODULE_URL}} or an explicit data-mech-wasm-module",),
    );
    assert!(!output.exists());
    assert!(!directory.path().join("_mech").exists());
}

#[cfg(all(has_file_wasm, has_file_js))]
#[test]
fn mech_format_placeholder_module_emits_both_runtime_assets() {
    let directory = TestDirectory::new("placeholder-module");
    let output = directory.path().join("placeholder.html");
    let html = format_fixture(Some(&fixture_path("all-slots.html")), None, &output);

    assert!(html.contains("data-mech-wasm-module=\"./_mech/pkg/mech_wasm.js\"",));
    assert!(directory.path().join("_mech/pkg/mech_wasm.js").is_file());
    assert!(
        directory
            .path()
            .join("_mech/pkg/mech_wasm_bg.wasm")
            .is_file(),
    );
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_custom_shim_renders_all_supported_slots() {
    let directory = TestDirectory::new("all-slots");
    let output = directory.path().join("all-slots.html");
    let html = format_fixture(
        Some(&fixture_path("all-slots.html")),
        Some(&fixture_path("all-slots.css")),
        &output,
    );

    shim_contract::assert_complete_slot_contract(&html, "");
    assert!(
        html.contains("41"),
        "encoded document program is unexpectedly empty"
    );
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_default_shim_restores_rich_shell() {
    let directory = TestDirectory::new("default");
    let html = format_fixture(None, None, &directory.path().join("default.html"));
    shim_contract::assert_rich_shell(
        &html,
        &[
            "contentShell",
            "articleIntro",
            "articleLayout",
            "main-content",
            "data-mech-console-resizer",
            "console-pane",
        ],
    );
    for layer in ["palette", "source", "mechdown", "page", "repl"] {
        assert!(
            html.contains(&format!("data-mech-style-layer=\"{layer}\"")),
            "default output did not expose the {layer} style layer",
        );
    }
    assert!(
        html.contains("data-mech-source"),
        "formatted Mech source lost its presentation boundary",
    );
    assert!(
        html.contains("mechdown-paragraph"),
        "formatted prose lost its Mechdown presentation hook",
    );
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_blog_shim_restores_rich_shell() {
    let directory = TestDirectory::new("blog");
    let html = format_fixture(
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("include/blog.html")),
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("include/blog.css")),
        &directory.path().join("blog.html"),
    );
    shim_contract::assert_rich_shell(
        &html,
        &[
            "contentShell",
            "articleIntro",
            "articleLayout",
            "console-pane",
        ],
    );
    for retained_style in [
        ".site-header {",
        ".toc a {",
        ".footer {",
        "html[data-mech-shim=\"blog\"] .hero-summary .mech-summary",
        ".mech-hyperlink,",
        "text-decoration-style: dotted",
    ] {
        assert!(
            html.contains(retained_style),
            "formatted blog lost shared or variant style {retained_style}"
        );
    }
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_docs_shim_restores_rich_shell() {
    let directory = TestDirectory::new("docs");
    let html = format_fixture(
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("include/docs.html")),
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("include/docs.css")),
        &directory.path().join("docs.html"),
    );
    shim_contract::assert_rich_shell(
        &html,
        &[
            "contentShell",
            "articleIntro",
            "articleLayout",
            "console-pane",
        ],
    );
    for retained_style in [
        ".toc a {",
        ".main-content {",
        "html[data-mech-shim=\"docs\"] .docs-layout",
        ".mech-hyperlink,",
        "text-decoration-style: dotted",
    ] {
        assert!(
            html.contains(retained_style),
            "formatted docs lost shared or variant style {retained_style}"
        );
    }
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_bundles_relative_import_sources() {
    use base64::Engine as _;

    let directory = TestDirectory::new("relative-import-bundle");
    let main = directory.path().join("main.mec");
    let support = directory.path().join("support.mec");
    let output = directory.path().join("formatted/main.html");
    std::fs::write(
        &main,
        "+> ./support.mec\nanswer := support/value + 1\nanswer\n",
    )
    .expect("main fixture must be written");
    std::fs::write(&support, "value := 41\n<+ value\n").expect("support fixture must be written");

    let output_result = Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("format")
        .arg(&main)
        .arg("--html")
        .arg("--out")
        .arg(&output)
        .output()
        .expect("Cargo-built mech formatter must start");
    assert!(
        output_result.status.success(),
        "mech format failed:\n{}",
        String::from_utf8_lossy(&output_result.stderr),
    );

    let html = std::fs::read_to_string(&output).expect("formatted page must exist");
    assert_retained_execution_payload(&html);
    let mount = html
        .split("data-mech-document-sources>")
        .nth(1)
        .and_then(|tail| tail.split("</script>").next())
        .map(str::trim)
        .expect("formatted page must contain the source bundle mount");
    let bundle: serde_json::Value = serde_json::from_slice(
        &base64::engine::general_purpose::STANDARD
            .decode(mount)
            .expect("source bundle must be base64"),
    )
    .expect("source bundle must be JSON");
    assert_eq!(bundle["version"], 2);
    assert_eq!(bundle["rootSpecifier"], "bundle/000000.mec");
    assert_eq!(
        bundle["sources"]
            .as_array()
            .expect("source list")
            .iter()
            .map(|source| source["specifier"].as_str().expect("specifier"))
            .collect::<Vec<_>>(),
        vec!["bundle/000000.mec", "bundle/000001.mec"],
    );
    assert_eq!(
        bundle["resolutions"],
        serde_json::json!([{
            "referrer": "bundle/000000.mec",
            "specifier": "./support.mec",
            "target": "bundle/000001.mec",
        }]),
    );
    assert!(html.contains("data-mech-wasm-module=\"./_mech/pkg/mech_wasm.js\""));
    assert!(!html.contains(&directory.path().display().to_string()));
    assert!(
        directory
            .path()
            .join("formatted/_mech/pkg/mech_wasm.js")
            .is_file()
    );
    assert!(
        directory
            .path()
            .join("formatted/_mech/pkg/mech_wasm_bg.wasm")
            .is_file()
    );
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_missing_dependency_writes_no_partial_bundle() {
    let directory = TestDirectory::new("missing-import-bundle");
    let main = directory.path().join("main.mec");
    let output = directory.path().join("formatted/main.html");
    std::fs::write(&main, "+> ./missing.mec\nanswer := 1\n").expect("main fixture must be written");
    let output_result = Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("format")
        .arg(&main)
        .arg("--html")
        .arg("--out")
        .arg(&output)
        .output()
        .expect("Cargo-built mech formatter must start");
    assert!(!output_result.status.success());
    let error = String::from_utf8_lossy(&output_result.stderr);
    assert!(
        error.contains("standalone HTML cannot bundle dependency `./missing.mec`",),
        "got {error}",
    );
    assert!(error.contains("requested by `file://"), "got {error}");
    assert!(!output.exists());
    assert!(!directory.path().join("formatted/_mech").exists());
}

#[test]
fn mech_format_unresolvable_standalone_dependencies_publish_nothing() {
    for (label, specifier) in [
        ("https", "https://example.com/dep.mec"),
        ("mech", "mech://stdlib/dep.mec"),
    ] {
        let directory = TestDirectory::new(label);
        let main = directory.path().join("main.mec");
        let output = directory.path().join("formatted/main.html");
        std::fs::write(&main, format!("+> {specifier}\nanswer := 1\n"))
            .expect("main fixture must be written");

        let result = Command::new(env!("CARGO_BIN_EXE_mech"))
            .arg("format")
            .arg(&main)
            .arg("--html")
            .arg("--shim")
            .arg(fixture_path("controller-literal-module.html"))
            .arg("--out")
            .arg(&output)
            .output()
            .expect("Cargo-built mech formatter must start");
        let error = String::from_utf8_lossy(&result.stderr);

        assert!(!result.status.success());
        assert!(
            error.contains(&format!(
                "standalone HTML cannot bundle dependency `{specifier}`",
            )),
            "got {error}",
        );
        assert!(error.contains("requested by `file://"), "got {error}");
        assert!(!output.exists());
        assert!(!directory.path().join("formatted/_mech").exists());
    }
}

#[cfg(all(windows, has_file_wasm, has_file_js))]
#[test]
fn mech_format_replaces_existing_runtime_assets_on_windows() {
    let directory = TestDirectory::new("windows-runtime-replacement");
    let output = directory.path().join("formatted/main.html");

    format_fixture(None, None, &output);
    let package = directory.path().join("formatted/_mech/pkg");
    let js = package.join("mech_wasm.js");
    let wasm = package.join("mech_wasm_bg.wasm");
    let expected_js = std::fs::read(&js).expect("first JavaScript asset");
    let expected_wasm = std::fs::read(&wasm).expect("first WASM asset");
    std::fs::write(&js, b"stale-js").expect("stale JavaScript fixture");
    std::fs::write(&wasm, b"stale-wasm").expect("stale WASM fixture");

    format_fixture(None, None, &output);

    assert_eq!(std::fs::read(&js).unwrap(), expected_js);
    assert_eq!(std::fs::read(&wasm).unwrap(), expected_wasm);
    let artifacts = std::fs::read_dir(&package)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.ends_with(".tmp") || name.ends_with(".stage") || name.ends_with(".backup")
        })
        .collect::<Vec<_>>();
    assert!(
        artifacts.is_empty(),
        "left runtime asset artifacts: {artifacts:?}"
    );
}

#[cfg(not(has_file_wasm))]
#[test]
fn mech_format_shipped_controller_explains_missing_embedded_runtime_assets() {
    let directory = TestDirectory::new("missing-runtime-assets");
    let output = directory.path().join("default.html");
    let output = format_fixture_output(None, None, &output);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("embedded mech_wasm_bg.wasm is unavailable"),
    );
    assert!(!directory.path().join("_mech").exists());
}

#[test]
fn mech_format_reports_positioned_canonical_errors_without_publication() {
    for ending in ["\n", "\r\n"] {
        for malformed in ["value := 1 + )", "résultat := [1 )"] {
            let directory = TestDirectory::new("positioned-errors");
            let input = directory.path().join("entrée.mec");
            let output = directory.path().join("formatted.mec");
            let shim = directory.path().join("static.html");
            let source = format!("valid := 1{ending}{malformed}{ending}");
            std::fs::write(&input, &source).unwrap();
            std::fs::write(&output, "previous published content").unwrap();
            std::fs::write(&shim, "<article>{{CONTENT}}</article>").unwrap();
            let retained = mech_runtime::SourceDocument::parse_resolved(
                &mech_runtime::SourceRequest::from_filesystem_path(&input)
                    .unwrap()
                    .specifier,
                mech_syntax::document::Revision(0),
                source.as_str(),
                mech_syntax::document::ParseConfig::default(),
            )
            .unwrap();
            let snapshot = retained.snapshot();
            assert!(!snapshot.diagnostics.is_empty());
            for html in [false, true] {
                let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
                command.arg("format").arg(&input).arg("--out").arg(&output);
                if html {
                    command.arg("--html").arg("--shim").arg(&shim);
                }
                let result = command.output().unwrap();
                assert!(
                    !result.status.success(),
                    "malformed {source:?} was published"
                );
                let stderr = String::from_utf8_lossy(&result.stderr);
                assert!(stderr.contains(&input.display().to_string()), "{stderr}");
                assert!(stderr.contains("InvalidFormatSyntax"), "{stderr}");
                for diagnostic in snapshot.diagnostics.iter() {
                    let range = diagnostic
                        .primary
                        .resolve(snapshot.source.revision(), &snapshot.nodes)
                        .unwrap();
                    assert!(range.start.0 >= "valid := 1".len() as u32, "{diagnostic:?}");
                    let (line, column) = snapshot
                        .source
                        .line_index()
                        .line_and_byte_column(range.start);
                    assert!(
                        stderr.contains(&format!("at {}:{}:", line + 1, column.0 + 1)),
                        "{stderr}"
                    );
                    assert!(stderr.contains(&diagnostic.message), "{stderr}");
                    assert!(
                        stderr
                            .contains(&format!("source bytes {}..{}", range.start.0, range.end.0)),
                        "{stderr}"
                    );
                }
                assert_eq!(std::fs::read_to_string(&input).unwrap(), source);
                assert_eq!(
                    std::fs::read_to_string(&output).unwrap(),
                    "previous published content"
                );
            }
        }
    }
}

#[test]
fn mech_format_raw_normalizes_reviewed_syntax_families_idempotently() {
    let directory = TestDirectory::new("canonical-pretty-contract");
    let input = directory.path().join("document.mec");
    let output = directory.path().join("formatted.mec");
    let second = directory.path().join("second.mec");
    let source = "Report\n======\n+> math/{sin,cos}\nauthor: Keep a,b\n======\n+> math/{trig/sin,trig/cos}\n@io:=cli://stdout{:read(*),:write(line)}\nresult:=make(x:1,y:2,note:\"a,b:c\")\n~~~mech{output:false,color:red}\nvalue:=1..3\n~~~\n![Keep  caption](image.png){width:wide,color:red}\n~~~mech:worker\n+> math/{sin,cos}\n~~~\n";
    let expected = "Report\n======\n+> math/{sin, cos}\nauthor: Keep a,b\n======\n+> math/{trig/sin, trig/cos}\n@io := cli://stdout { :read(*), :write(line) }\nresult := make(x: 1, y: 2, note: \"a,b:c\")\n~~~mech{output: false, color: red}\nvalue := 1..3\n~~~\n![Keep  caption](image.png){width: wide, color: red}\n~~~mech:worker\n+> math/{sin, cos}\n~~~\n";
    let source = format!(
        "{source}add(x<u64>,y<u64>) = out<u64> := out := x + y.\nsplit(x<u64>,y<u64>) = (a<u64>,b<u64>) := a := x; b := y.\n~~~mech:signatures\nchoose(x<u64>,y<u64>) => <u64>\n| 0 => y\n| x => x.\n~~~\n"
    );
    let expected = format!(
        "{expected}add(x<u64>, y<u64>) = out<u64> := out := x + y.\nsplit(x<u64>, y<u64>) = (a<u64>, b<u64>) := a := x; b := y.\n~~~mech:signatures\nchoose(x<u64>, y<u64>) => <u64>\n| 0 => y\n| x => x.\n~~~\n"
    );
    std::fs::write(&input, source).unwrap();
    for (source_path, output_path) in [(&input, &output), (&output, &second)] {
        let result = Command::new(env!("CARGO_BIN_EXE_mech"))
            .arg("format")
            .arg(source_path)
            .arg("--out")
            .arg(output_path)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(std::fs::read_to_string(output_path).unwrap(), expected);
    }
}

#[cfg(feature = "run")]
#[test]
fn mech_format_preserves_executable_results_through_public_commands() {
    let directory = TestDirectory::new("format-execution-contract");
    for (index, source) in [
        "pair:=(2,3)\n(left,right):=pair\nleft * 10 + right\n",
        "record:={left:2,right:3}\nrecord.left * 10 + record.right\n",
        "answer:=math/sub(left:30,right:7)\nanswer\n",
        "add(x<u64>,y<u64>) = out<u64> := out := x + y.\nadd(20,3)\n",
    ]
    .iter()
    .enumerate()
    {
        let input = directory.path().join(format!("input-{index}.mec"));
        let formatted = directory.path().join(format!("formatted-{index}.mec"));
        std::fs::write(&input, source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_mech"))
            .args(["--no-config", "format"])
            .arg(&input)
            .arg("--out")
            .arg(&formatted)
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        for path in [&input, &formatted] {
            let output = Command::new(env!("CARGO_BIN_EXE_mech"))
                .args(["--no-config", "run"])
                .arg(path)
                .current_dir(directory.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout).lines().last(),
                Some("23"),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }
}
