#[path = "bundle_planning.rs"]
mod planning;

use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use mech_core::*;
#[cfg(test)]
use mech_runtime::CanonicalProgramBundle;

use crate::canonical_presentation::{HtmlShimExtraSlots, HtmlStyleSheets, render_canonical_html};
use crate::fs_paths::validate_safe_relative_path;
use crate::{HostAuthorityInjection, LoadedMechConfig, resolve_config_path};

const STATIC_PROJECT_BOOTSTRAP: &str = include_str!("../include/static-project.js");
const STATIC_PROJECT_ADMISSION: &str = include_str!("../include/static-project-admission.mjs");
const STATIC_PROJECT_SCRIPT: &str =
    r#"<script type="module" src="./_mech/project.js" data-mech-project="."></script>"#;

fn validation_error(msg: impl Into<String>) -> MechError {
    MechError::new(GenericError { msg: msg.into() }, None).with_compiler_loc()
}

#[derive(Clone, Debug)]
pub struct BundleWebOptions {
    pub project_dir: PathBuf,
    pub output_dir: PathBuf,
    pub source_paths: Vec<PathBuf>,
    pub shim_path: PathBuf,
    pub stylesheet_paths: Vec<PathBuf>,
    pub wasm_pkg: PathBuf,
    pub loaded_config: LoadedMechConfig,
    pub host_config_injection: Option<HostAuthorityInjection>,
}

#[derive(Debug)]
pub struct BundleWebResult {
    pub output_dir: PathBuf,
    pub index_html: PathBuf,
    pub source_count: usize,
}

#[derive(Debug)]
struct BundledSource {
    canonical_path: PathBuf,
    specifier: String,
    url: String,
    document_url: Option<String>,
    nominal_origin: Option<CanonicalNominalPath>,
    nominal_package_id: Option<String>,
}

pub fn bundle_web_project(options: BundleWebOptions) -> MResult<BundleWebResult> {
    if options.source_paths.is_empty() {
        return Err(validation_error(
            "bundle-web requires serve.paths in the project config",
        ));
    }
    validate_static_bundle_wasm_package(&options.wasm_pkg)?;

    let project_dir = options.project_dir.canonicalize()?;
    let base_dir = options.loaded_config.base_dir.canonicalize()?;
    let wasm_pkg = options.wasm_pkg.canonicalize()?;
    let config_source = fs::read_to_string(&options.loaded_config.path)?;
    if options.output_dir.exists() {
        validate_bundle_output_directory(
            &options.output_dir.canonicalize()?,
            &project_dir,
            &base_dir,
        )?;
    }
    if options
        .loaded_config
        .document
        .run
        .as_ref()
        .map(|run| run.paths.is_empty())
        .unwrap_or(true)
    {
        return Err(validation_error(
            "bundle-web requires run.paths in the project config",
        ));
    }
    if options
        .loaded_config
        .document
        .run
        .as_ref()
        .unwrap()
        .paths
        .len()
        != 1
    {
        return Err(validation_error(
            "bundle-web requires exactly one run root; the browser project activates one root artifact",
        ));
    }
    let stylesheet_string = read_stylesheets(&options.stylesheet_paths)?;
    let shim_string = read_shim(&options.shim_path)?;
    validate_static_web_shim(&shim_string)?;
    let shim_string = ensure_static_project_bootstrap(&shim_string);

    let runtime_config = crate::apply_runtime_config_patch(
        mech_runtime::RuntimeConfig::default(),
        &options.loaded_config.document.runtime,
    )?;
    let host_config = crate::web_runtime_injection_config_from_document(
        &options.loaded_config.document,
        &runtime_config,
    )?;
    let injection = options
        .host_config_injection
        .unwrap_or_else(|| HostAuthorityInjection::BrowserUnsigned(host_config));
    let root_shim_with_config =
        crate::inject_host_authority_injection_script(&shim_string, &injection)?;

    let root_paths = options
        .loaded_config
        .document
        .run
        .as_ref()
        .unwrap()
        .paths
        .iter()
        .map(|path| resolve_config_path(&base_dir, path).canonicalize())
        .collect::<std::io::Result<BTreeSet<_>>>()?;
    let mut bundled_sources = Vec::with_capacity(options.source_paths.len());
    let mut prepared_files = Vec::new();
    let (resolver, documents, resolutions) =
        planning::retained_sources(&options.source_paths, &base_dir, &project_dir)?;
    let mut compiler = crate::configured_browser_compiler_builder(
        &options.loaded_config.document.hosts,
        runtime_config,
    )?
    .source_resolver(resolver)
    .build_compiler()?;
    for source_path in &options.source_paths {
        let logical_source_path = source_path;
        let read_source_path = source_path.canonicalize()?;
        let relative = relative_source_path(logical_source_path, &base_dir, &project_dir)?;
        let specifier = bundle_source_specifier(&relative)?;
        let canonical_uri = format!("bundle:///{specifier}");
        let document = &documents[&canonical_uri];
        let source_text = document.source().to_contiguous_string();
        let url = format!("source/{}", percent_encode_url_path(&specifier));
        bundled_sources.push(BundledSource {
            canonical_path: read_source_path.clone(),
            specifier: specifier.clone(),
            url,
            document_url: root_paths
                .contains(&read_source_path)
                .then(|| format!("code/{}", percent_encode_url_path(&specifier))),
            nominal_origin: document.nominal_origin().cloned(),
            nominal_package_id: document.nominal_package_id().map(str::to_owned),
        });

        prepared_files.push(("source", relative.clone(), source_text.into_bytes()));

        if root_paths.contains(&read_source_path) {
            let encoded = crate::browser_planning::compile_browser_document_payload(
                &mut compiler,
                &canonical_uri,
                &specifier,
                document,
            )?
            .encode()?;
            prepared_files.push(("code", relative.clone(), encoded.into_bytes()));
        }

        let html_relative = relative.with_extension("html");
        let depth = html_relative.components().count();
        let rebased_shim = rebase_bundle_shim_for_depth(&shim_string, depth);
        let source_shim = crate::inject_host_authority_injection_script(&rebased_shim, &injection)?;
        let html = render_canonical_html(
            &document.document(),
            HtmlStyleSheets::legacy(stylesheet_string.clone()),
            source_shim,
            &HtmlShimExtraSlots::default(),
        )?
        .html;
        prepared_files.push(("html", html_relative, html.into_bytes()));
    }
    let mut roots = Vec::with_capacity(
        options
            .loaded_config
            .document
            .run
            .as_ref()
            .unwrap()
            .paths
            .len(),
    );
    for run_path in &options.loaded_config.document.run.as_ref().unwrap().paths {
        let resolved = resolve_config_path(&base_dir, run_path).canonicalize()?;
        let source = bundled_sources
            .iter()
            .find(|source| source.canonical_path == resolved)
            .ok_or_else(|| {
                validation_error(format!(
                    "bundle-web run path is not included by serve.paths: {}",
                    run_path.display(),
                ))
            })?;
        roots.push(source.specifier.clone());
    }
    let sources = bundled_sources
        .iter()
        .map(|source| {
            let document = &documents[&format!("bundle:///{}", source.specifier)];
            (
                source.specifier.clone(),
                document.source().to_contiguous_string(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let provenance = bundled_sources
        .iter()
        .filter_map(|source| {
            source.nominal_origin.as_ref().map(|origin| {
                (
                    source.specifier.clone(),
                    serde_json::json!({
                        "nominalOrigin": origin,
                        "nominalPackageId": source.nominal_package_id,
                    }),
                )
            })
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    // Freeze the supplied package before asking it to admit sources, then
    // publish those same bytes. A concurrent rebuild cannot replace the
    // validated compiler with a different package between check and copy.
    let admitted_package = tempfile::tempdir()?;
    copy_wasm_package(&wasm_pkg, admitted_package.path())?;
    validate_static_source_closure(
        admitted_package.path(),
        &serde_json::json!({
            "config": config_source,
            "sources": sources,
            "roots": roots,
            "resolutions": resolutions,
            "provenance": provenance,
        }),
    )?;

    // Both native planning and the exact browser package have admitted the
    // retained closure before publishing any project or source files.
    fs::create_dir_all(&options.output_dir)?;
    let output_dir = options.output_dir.canonicalize()?;
    validate_bundle_output_directory(&output_dir, &project_dir, &base_dir)?;
    copy_project_static_assets(
        &project_dir,
        &output_dir,
        &[output_dir.clone(), wasm_pkg.clone()],
    )?;
    fs::write(output_dir.join("style.css"), &stylesheet_string)?;
    copy_wasm_package(admitted_package.path(), &output_dir.join("pkg"))?;
    fs::write(output_dir.join("mech.mcfg"), &config_source)?;
    fs::create_dir_all(output_dir.join("_mech"))?;
    fs::write(
        output_dir.join("_mech/project.js"),
        STATIC_PROJECT_BOOTSTRAP,
    )?;
    let index_html = output_dir.join("index.html");
    fs::write(&index_html, &root_shim_with_config)?;
    for (directory, relative, bytes) in prepared_files {
        write_bundle_file(&output_dir, directory, &relative, &bytes)?;
    }
    let source_entries = bundled_sources
        .iter()
        .map(|source| {
            let mut entry = serde_json::json!({
              "specifier": source.specifier,
              "url": source.url,
            });
            if let Some(url) = &source.document_url {
                entry["documentUrl"] = serde_json::json!(url);
            }
            if let Some(origin) = &source.nominal_origin {
                entry["nominalOrigin"] = serde_json::json!(origin);
            }
            if let Some(package) = &source.nominal_package_id {
                entry["nominalPackageId"] = serde_json::json!(package);
            }
            entry
        })
        .collect::<Vec<_>>();
    let manifest = serde_json::to_vec(&serde_json::json!({
      "version": 4,
      "roots": roots,
      "sources": source_entries,
      "resolutions": resolutions,
    }))
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(output_dir.join("_mech/project-sources.json"), manifest)?;
    Ok(BundleWebResult {
        output_dir,
        index_html,
        source_count: options.source_paths.len(),
    })
}

fn validate_bundle_output_directory(
    output_dir: &Path,
    project_dir: &Path,
    base_dir: &Path,
) -> MResult<()> {
    if output_dir == project_dir {
        return Err(validation_error(format!(
            "bundle-web output directory must not be the project root: {}. Use a subdirectory such as dist/<name>.",
            output_dir.display(),
        )));
    }
    if output_dir == base_dir {
        return Err(validation_error(format!(
            "bundle-web output directory must not be the config base directory: {}. Use a subdirectory such as dist/<name>.",
            output_dir.display(),
        )));
    }
    Ok(())
}

pub(crate) fn validate_static_bundle_wasm_package(path: &Path) -> MResult<()> {
    if !path.is_dir() {
        return Err(validation_error(format!(
            "configuration error: serve.wasm must be an existing directory: {}",
            path.display(),
        )));
    }

    let js_path = path.join("mech_wasm.js");
    let wasm_path = path.join("mech_wasm_bg.wasm");
    for required in [&js_path, &wasm_path] {
        if !required.is_file() {
            return Err(validation_error(format!(
                "configuration error: serve.wasm is missing required file: {}",
                required.display(),
            )));
        }
    }

    let wrapper = fs::read_to_string(&js_path).map_err(|_| static_wasm_profile_error())?;
    if !wrapper.contains("WasmProject")
        || !wrapper.contains("fromServedDocuments")
        || !wrapper.contains("supportsServedDocumentResolutions")
        || !wrapper.contains("supportsServedDocumentProvenance")
        || !wrapper.contains("validateStaticSources")
    {
        return Err(static_wasm_profile_error());
    }

    Ok(())
}

fn validate_static_source_closure(package: &Path, request: &serde_json::Value) -> MResult<()> {
    let mut child = Command::new("node")
        .args(["--input-type=module", "--eval", STATIC_PROJECT_ADMISSION])
        .arg(package)
        // The admitted package is owned and stable for this entire probe.
        // Do not inherit a caller's unrelated or concurrently removed cwd.
        .current_dir(package)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            validation_error(format!(
                "bundle-web requires Node.js to validate the supplied browser package: {error}"
            ))
        })?;
    // Read stderr concurrently so a compiler diagnostic cannot fill the pipe.
    let mut stderr = child.stderr.take().expect("piped child stderr");
    let diagnostics = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        let _ = stderr.by_ref().take(64 * 1024).read_to_end(&mut bytes);
        // Continue draining after the diagnostic limit without retaining it.
        let _ = std::io::copy(&mut stderr, &mut std::io::sink());
        String::from_utf8_lossy(&bytes).into_owned()
    });
    let input = serde_json::to_vec(request).map_err(|error| validation_error(error.to_string()))?;
    let write_result = child
        .stdin
        .take()
        .expect("piped child stdin")
        .write_all(&input);
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(60) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = diagnostics.join();
            return Err(validation_error(
                "browser package source admission exceeded 60 seconds",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let diagnostic = diagnostics.join().unwrap_or_default();
    if !status.success() || write_result.is_err() {
        return Err(validation_error(format!(
            "serve.wasm cannot compile this project's retained source closure: {}",
            diagnostic.trim()
        )));
    }
    Ok(())
}

fn static_wasm_profile_error() -> MechError {
    validation_error(
        "configuration error: serve.wasm was built without static served-project support; rebuild it with `python scripts/build-wasm.py --profile browser` or the `browser_project` feature",
    )
}

fn rebase_bundle_shim_for_depth(shim: &str, depth: usize) -> String {
    if depth == 0 {
        return shim.to_string();
    }
    let prefix = "../".repeat(depth);
    let mut rebased = shim.to_string();
    if let Some(bootstrap) = static_project_bootstrap_tag(&rebased) {
        let project_base = "../".repeat(depth);
        if let Some(attribute) = bootstrap.attribute("data-mech-project") {
            if attribute.value == "." {
                rebased.replace_range(attribute.value_start..attribute.value_end, &project_base);
            }
        } else {
            rebased.insert_str(
                bootstrap.end - 1,
                &format!(" data-mech-project=\"{project_base}\""),
            );
        }
    }
    for asset in ["pkg/", "style.css", "code/", "source/", "_mech/"] {
        let from = format!("./{asset}");
        let to = format!("{prefix}{asset}");
        rebased = rebased.replace(&from, &to);
    }
    rebased
}

fn ensure_static_project_bootstrap(shim: &str) -> String {
    if static_project_bootstrap_tag(shim).is_some() {
        return shim.to_string();
    }
    if let Some(index) = shim.find("</body>") {
        let mut out = shim.to_string();
        out.insert_str(index, STATIC_PROJECT_SCRIPT);
        out
    } else {
        format!("{shim}\n{STATIC_PROJECT_SCRIPT}")
    }
}

#[derive(Debug)]
struct HtmlAttribute {
    name: String,
    value: String,
    value_start: usize,
    value_end: usize,
}

#[derive(Debug)]
struct ScriptStartTag {
    end: usize,
    attributes: Vec<HtmlAttribute>,
}

impl ScriptStartTag {
    fn attribute(&self, name: &str) -> Option<&HtmlAttribute> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name.eq_ignore_ascii_case(name))
    }
}

fn static_project_bootstrap_tag(shim: &str) -> Option<ScriptStartTag> {
    script_start_tags(shim).into_iter().find(|tag| {
        tag.attribute("src")
            .is_some_and(|attribute| is_static_project_bootstrap_url(&attribute.value))
    })
}

fn is_static_project_bootstrap_url(url: &str) -> bool {
    let path_end = url.find(['?', '#']).unwrap_or(url.len());
    &url[..path_end] == "./_mech/project.js"
}

fn script_start_tags(html: &str) -> Vec<ScriptStartTag> {
    let mut tags = Vec::new();
    let mut offset = 0;
    while let Some(relative_start) = html[offset..].find("<script") {
        let start = offset + relative_start;
        let after_name = start + "<script".len();
        let Some(first_after_name) = html.as_bytes().get(after_name) else {
            break;
        };
        if !first_after_name.is_ascii_whitespace() && *first_after_name != b'>' {
            offset = after_name;
            continue;
        }
        let Some(end) = script_start_tag_end(html, after_name) else {
            break;
        };
        tags.push(ScriptStartTag {
            end,
            attributes: quoted_attributes(&html[after_name..end - 1], after_name),
        });
        offset = end;
    }
    tags
}

fn script_start_tag_end(html: &str, start: usize) -> Option<usize> {
    let mut quote = None;
    for (relative, byte) in html.as_bytes()[start..].iter().copied().enumerate() {
        match quote {
            Some(expected) if byte == expected => quote = None,
            Some(_) => {}
            None if matches!(byte, b'\'' | b'\"') => quote = Some(byte),
            None if byte == b'>' => return Some(start + relative + 1),
            None => {}
        }
    }
    None
}

fn quoted_attributes(source: &str, offset: usize) -> Vec<HtmlAttribute> {
    let bytes = source.as_bytes();
    let mut attributes = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
            || bytes.get(index) == Some(&b'/')
        {
            index += 1;
        }
        let name_start = index;
        while let Some(byte) = bytes.get(index) {
            if byte.is_ascii_whitespace() || matches!(*byte, b'=' | b'/') {
                break;
            }
            index += 1;
        }
        if name_start == index {
            index += 1;
            continue;
        }
        let name = &source[name_start..index];
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        if bytes.get(index) != Some(&b'=') {
            continue;
        }
        index += 1;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        let Some(quote) = bytes
            .get(index)
            .copied()
            .filter(|byte| matches!(*byte, b'\'' | b'\"'))
        else {
            continue;
        };
        index += 1;
        let value_start = index;
        while bytes.get(index) != Some(&quote) {
            if index == bytes.len() {
                return attributes;
            }
            index += 1;
        }
        attributes.push(HtmlAttribute {
            name: name.to_string(),
            value: source[value_start..index].to_string(),
            value_start: offset + value_start,
            value_end: offset + index,
        });
        index += 1;
    }
    attributes
}

fn read_stylesheets(paths: &[PathBuf]) -> MResult<String> {
    let mut combined = String::new();
    for path in paths {
        let stylesheet = fs::read_to_string(path)?;
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&stylesheet);
    }
    Ok(combined)
}

fn read_shim(path: &Path) -> MResult<String> {
    Ok(fs::read_to_string(path)?)
}

fn validate_static_web_shim(shim: &str) -> MResult<()> {
    if shim.contains("{{DOCUMENT_SCRIPT}}") || shim.contains("{{SOURCE_URL_KEY}}") {
        return Err(validation_error(
            "bundle-web does not support source-document controller shims. Use a static project shim with `./_mech/project.js`, or use `mech format --html` / `mech serve` for a standalone document.",
        ));
    }

    for (pattern, url, fix) in [
        ("\"/code/", "/code/", "./code/..."),
        ("'/code/", "/code/", "./code/..."),
        ("`/code/", "/code/", "./code/..."),
        ("\"/source/", "/source/", "./source/..."),
        ("'/source/", "/source/", "./source/..."),
        ("`/source/", "/source/", "./source/..."),
        (
            "\"/pkg/mech_wasm.js",
            "/pkg/mech_wasm.js",
            "./pkg/mech_wasm.js",
        ),
        (
            "'/pkg/mech_wasm.js",
            "/pkg/mech_wasm.js",
            "./pkg/mech_wasm.js",
        ),
        (
            "`/pkg/mech_wasm.js",
            "/pkg/mech_wasm.js",
            "./pkg/mech_wasm.js",
        ),
        ("\"/_mech/", "/_mech/", "./pkg/mech_wasm.js"),
        ("'/_mech/", "/_mech/", "./pkg/mech_wasm.js"),
        ("`/_mech/", "/_mech/", "./pkg/mech_wasm.js"),
    ] {
        if shim.contains(pattern) {
            return Err(validation_error(format!(
                "bundle-web shim contains server-root Mech URL `{url}`.\nUse a relative URL such as `{fix}` or `./pkg/mech_wasm.js`.",
            )));
        }
    }

    Ok(())
}

pub fn copy_project_static_assets(
    project_dir: &Path,
    output_dir: &Path,
    excluded_dirs: &[PathBuf],
) -> MResult<()> {
    let project_dir = project_dir.canonicalize()?;
    let output_dir = output_dir.canonicalize()?;
    let excluded_dirs = excluded_dirs
        .iter()
        .filter_map(|path| path.canonicalize().ok())
        .collect::<Vec<_>>();
    let mut visited = BTreeSet::new();
    copy_project_static_assets_inner(
        &project_dir,
        &project_dir,
        &project_dir,
        &output_dir,
        &excluded_dirs,
        &mut visited,
    )
}

fn copy_project_static_assets_inner(
    project_dir: &Path,
    logical_dir: &Path,
    read_dir: &Path,
    output_dir: &Path,
    excluded_dirs: &[PathBuf],
    visited: &mut BTreeSet<PathBuf>,
) -> MResult<()> {
    let canonical_dir = read_dir.canonicalize()?;
    if !visited.insert(canonical_dir.clone()) {
        return Ok(());
    }
    for entry in fs::read_dir(read_dir)? {
        let entry = entry?;
        let logical_path = logical_dir.join(entry.file_name());
        let read_path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink()
            && read_path
                .canonicalize()
                .map(|target| target.is_dir())
                .unwrap_or(false)
        {
            continue;
        }
        let canonical_path = read_path.canonicalize()?;

        if should_skip_static_asset_path(&canonical_path, output_dir, excluded_dirs) {
            continue;
        }

        if canonical_path.is_dir() {
            if should_skip_static_asset_dir(&canonical_path) {
                continue;
            }
            copy_project_static_assets_inner(
                project_dir,
                &logical_path,
                &canonical_path,
                output_dir,
                excluded_dirs,
                visited,
            )?;
            continue;
        }

        if !is_allowed_static_asset(&canonical_path) {
            continue;
        }

        if !canonical_path.starts_with(project_dir) {
            return Err(validation_error(format!(
                "bundle-web static asset target is outside project root: {}",
                canonical_path.display()
            )));
        }

        let relative = logical_path.strip_prefix(project_dir).map_err(|error| {
            validation_error(format!(
                "bundle-web static asset path is outside project root: {error}"
            ))
        })?;
        validate_safe_relative_path(relative)?;

        let output_path = output_dir.join(relative);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&canonical_path, output_path)?;
    }
    Ok(())
}

fn should_skip_static_asset_path(
    path: &Path,
    output_dir: &Path,
    excluded_dirs: &[PathBuf],
) -> bool {
    path == output_dir
        || path.starts_with(output_dir)
        || excluded_dirs
            .iter()
            .any(|excluded| path == excluded || path.starts_with(excluded))
}

fn should_skip_static_asset_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("target" | "dist" | ".git")
    )
}

fn is_allowed_static_asset(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(
            "html"
                | "htm"
                | "css"
                | "js"
                | "wasm"
                | "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "svg"
                | "webp"
                | "ico"
                | "md"
                | "csv"
                | "json"
        )
    )
}

fn copy_wasm_package(wasm_pkg: &Path, output_pkg: &Path) -> MResult<()> {
    fs::create_dir_all(output_pkg)?;
    fs::copy(
        wasm_pkg.join("mech_wasm.js"),
        output_pkg.join("mech_wasm.js"),
    )?;
    fs::copy(
        wasm_pkg.join("mech_wasm_bg.wasm"),
        output_pkg.join("mech_wasm_bg.wasm"),
    )?;
    let snippets = wasm_pkg.join("snippets");
    if snippets.exists() {
        copy_wasm_snippets(&snippets, &output_pkg.join("snippets"))?;
    }
    Ok(())
}

fn copy_wasm_snippets(source: &Path, destination: &Path) -> MResult<()> {
    if fs::symlink_metadata(source)?.file_type().is_symlink() {
        return Err(validation_error("WASM snippets must not contain symlinks"));
    }
    if source.is_dir() {
        fs::create_dir_all(destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            copy_wasm_snippets(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else {
        fs::copy(source, destination)?;
    }
    Ok(())
}

fn relative_source_path(source: &Path, base_dir: &Path, project_dir: &Path) -> MResult<PathBuf> {
    let relative = if let Ok(relative) = source.strip_prefix(project_dir) {
        relative
    } else if let Ok(relative) = source.strip_prefix(base_dir) {
        relative
    } else {
        return Err(validation_error(format!(
            "bundle-web source is outside project/config root: {}",
            source.display()
        )));
    };

    validate_safe_relative_path(relative)?;
    Ok(relative.to_path_buf())
}

fn bundle_source_specifier(relative: &Path) -> MResult<String> {
    validate_safe_relative_path(relative)?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn percent_encode_url_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(percent_encode_hex_digit(byte >> 4));
            encoded.push(percent_encode_hex_digit(byte & 0x0f));
        }
    }
    encoded
}

fn percent_encode_hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        _ => (b'A' + value - 10) as char,
    }
}

fn write_bundle_file(
    output_dir: &Path,
    section: &str,
    relative: &Path,
    bytes: &[u8],
) -> MResult<()> {
    validate_safe_relative_path(relative)?;
    let path = output_dir.join(section).join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const STATIC_WASM_WRAPPER: &str = r#"export class WasmProject {
  static fromServedDocuments() {}
  static supportsServedAuthority() { return true; }
  static supportsServedDocumentResolutions() { return true; }
  static supportsServedDocumentProvenance() { return true; }
  static validateStaticSources() { return true; }
}
export default async function init() {}
"#;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-bundle-web-{name}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn write_demo_project(root: &Path) -> LoadedMechConfig {
        fs::write(
            root.join("demo.mcfg"),
            r#"config := {
  runtime: {name: "bundle-test"}
  serve: {
    paths: ["demo.mec"]
    shim: "index.html"
    wasm: "pkg"
  }
  run: {paths: ["demo.mec"]}
}
"#,
        )
        .unwrap();
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec");</script></body></html>"#,
    )
    .unwrap();
        fs::write(root.join("demo.mec"), "x := 1\n").unwrap();
        fs::create_dir_all(root.join("pkg")).unwrap();
        fs::write(root.join("pkg/mech_wasm.js"), STATIC_WASM_WRAPPER).unwrap();
        fs::write(root.join("pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
        crate::load_mech_config_path(root.join("demo.mcfg"), Some(root.to_path_buf())).unwrap()
    }

    fn write_browser_alias_project(root: &Path) -> LoadedMechConfig {
        fs::write(
            root.join("demo.mcfg"),
            r##"config := {
  runtime: {name: "bundle-alias-test"}
  serve: {
    paths: ["demo.mec"]
    shim: "index.html"
    wasm: "pkg"
  }
  hosts: [
    {
      name: "ui"
      provider: "browser"
      settings: {
        dom: [
          {
            path: "body/content/allowed/_value"
            selector: "#allowed"
            property: "value"
            operations: ["write"]
          }
          {
            path: "body/content/denied/_value"
            selector: "#denied"
            property: "value"
            operations: ["write"]
          }
        ]
      }
    }
  ]
  run: {
    paths: ["demo.mec"]
    grants: [
      {
        target: "ui/dom"
        operations: ["write"]
        paths: ["body/content/allowed/_value"]
      }
    ]
  }
}
"##,
        )
        .unwrap();
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec");</script></body></html>"#,
    )
    .unwrap();
        fs::write(root.join("demo.mec"), "x := 1\n").unwrap();
        fs::create_dir_all(root.join("pkg")).unwrap();
        fs::write(root.join("pkg/mech_wasm.js"), STATIC_WASM_WRAPPER).unwrap();
        fs::write(root.join("pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
        crate::load_mech_config_path(root.join("demo.mcfg"), Some(root.to_path_buf())).unwrap()
    }

    fn options(root: &Path, out: &Path, loaded: LoadedMechConfig) -> BundleWebOptions {
        BundleWebOptions {
            project_dir: root.to_path_buf(),
            output_dir: out.to_path_buf(),
            source_paths: vec![root.join("demo.mec")],
            shim_path: root.join("index.html"),
            stylesheet_paths: Vec::new(),
            wasm_pkg: root.join("pkg"),
            loaded_config: loaded,
            host_config_injection: None,
        }
    }

    fn compile_test_bundle(
        root: &Path,
        loaded: &LoadedMechConfig,
        source_paths: &[PathBuf],
    ) -> CanonicalProgramBundle {
        let project_dir = root.canonicalize().unwrap();
        let base_dir = loaded.base_dir.canonicalize().unwrap();
        let (resolver, documents, _) =
            planning::retained_sources(source_paths, &base_dir, &project_dir).unwrap();
        let runtime_config = crate::apply_runtime_config_patch(
            mech_runtime::RuntimeConfig::default(),
            &loaded.document.runtime,
        )
        .unwrap();
        let mut compiler =
            crate::configured_browser_compiler_builder(&loaded.document.hosts, runtime_config)
                .unwrap()
                .source_resolver(resolver)
                .build_compiler()
                .unwrap();
        let uri = "bundle:///demo.mec";
        crate::browser_planning::compile_browser_document_bundle(
            &mut compiler,
            uri,
            &documents[uri],
        )
        .unwrap()
    }

    fn decode_browser_payload(out: &Path) -> mech_runtime::BrowserDocumentPayload {
        mech_runtime::BrowserDocumentPayload::decode(
            &fs::read_to_string(out.join("code/demo.mec")).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn canonical_bundle_root_executes_with_transitive_source_exports() {
        let root = temp_root("canonical-imports");
        let loaded = write_demo_project(&root);
        fs::write(
            root.join("demo.mec"),
            "+> ./dep.mec\nanswer := dep/value + 2\n",
        )
        .unwrap();
        fs::write(
            root.join("dep.mec"),
            "+> ./leaf.mec\nvalue := leaf/value + 1\n<+ value\n",
        )
        .unwrap();
        fs::write(root.join("leaf.mec"), "value := 39\n<+ value\n").unwrap();
        let out = root.join("out");
        let source_paths = vec![
            root.join("demo.mec"),
            root.join("dep.mec"),
            root.join("leaf.mec"),
        ];
        let mut options = options(&root, &out, loaded.clone());
        options.source_paths = source_paths.clone();
        bundle_web_project(options).unwrap();
        let payload = decode_browser_payload(&out);
        assert_eq!(payload.root_specifier(), "demo.mec");
        assert_eq!(
            payload.source(),
            fs::read_to_string(root.join("demo.mec")).unwrap()
        );
        let bundle = compile_test_bundle(&root, &loaded, &source_paths);
        let mut served_sources = std::collections::BTreeMap::from([
            (
                "bundle:///dep.mec".to_owned(),
                fs::read_to_string(out.join("source/dep.mec")).unwrap(),
            ),
            (
                "bundle:///leaf.mec".to_owned(),
                fs::read_to_string(out.join("source/leaf.mec")).unwrap(),
            ),
        ]);
        assert_eq!(
            bundle.source_dependencies,
            served_sources
                .iter()
                .map(|(uri, text)| (uri.clone(), hash_str(text)))
                .collect()
        );
        bundle
            .validate_dependency_sources(|uri| served_sources.get(uri).map(String::as_str))
            .unwrap();
        let old_leaf = served_sources
            .insert(
                "bundle:///leaf.mec".to_owned(),
                "value := 100\n<+ value\n".to_owned(),
            )
            .unwrap();
        let error = bundle
            .validate_dependency_sources(|uri| served_sources.get(uri).map(String::as_str))
            .unwrap_err();
        assert!(error.display_message().contains("leaf.mec"));
        served_sources.insert("bundle:///leaf.mec".to_owned(), old_leaf);
        served_sources.remove("bundle:///dep.mec");
        let error = bundle
            .validate_dependency_sources(|uri| served_sources.get(uri).map(String::as_str))
            .unwrap_err();
        assert!(error.display_message().contains("dep.mec"));
        let mut runtime = mech_runtime::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap();
        let durability = runtime.config().resident_durability;
        let result = runtime
            .load_bytecode_program(&bundle.bytecode, durability)
            .unwrap();
        assert_eq!(result.initial_value.format_canonical_inline(), "42");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_bundle_dependencies_use_configured_resource_planning_values() {
        let root = temp_root("canonical-import-resource");
        let mut loaded = write_demo_project(&root);
        loaded
            .document
            .hosts
            .push(mech_runtime::HostInstanceConfig {
                name: "clock".to_owned(),
                provider: "timer".to_owned(),
                settings: mech_runtime::ConfigValue::Map(std::collections::BTreeMap::new()),
            });
        fs::write(root.join("demo.mec"), "+> ./dep.mec\nanswer := dep/value\n").unwrap();
        fs::write(
            root.join("dep.mec"),
            "@clock := timer://clock/tick{:read(tick)}\nvalue := @clock/tick\n<+ value\n",
        )
        .unwrap();
        let out = root.join("out");
        let source_paths = vec![root.join("demo.mec"), root.join("dep.mec")];
        let mut options = options(&root, &out, loaded.clone());
        options.source_paths = source_paths.clone();
        bundle_web_project(options).unwrap();
        let bundle = compile_test_bundle(&root, &loaded, &source_paths);
        let mut runtime = mech_runtime::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap();
        let durability = runtime.config().resident_durability;
        runtime
            .load_bytecode_program(&bundle.bytecode, durability)
            .unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_bundle_admits_documented_browser_assignments() {
        let root = temp_root("canonical-browser-assignments");
        write_demo_project(&root);
        fs::write(
            root.join("demo.mec"),
            include_str!("../examples/browser-dom-demo/demo.mec"),
        )
        .unwrap();
        fs::write(
            root.join("demo.mcfg"),
            include_str!("../examples/browser-dom-demo/demo.mcfg"),
        )
        .unwrap();
        let loaded =
            crate::load_mech_config_path(root.join("demo.mcfg"), Some(root.clone())).unwrap();
        let out = root.join("out");
        bundle_web_project(options(&root, &out, loaded.clone())).unwrap();
        let bundle = compile_test_bundle(&root, &loaded, &[root.join("demo.mec")]);
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bundle.bytecode).unwrap();
        let assignments = artifact
            .requirements()
            .iter()
            .filter(|(_, requirement)| {
                matches!(
                    requirement, mech_core::ApplicationRequirement::Resource(request)
                        if request.intent == mech_core::ResourceIntent::Assign
                )
            })
            .count();
        assert_eq!(assignments, 4);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_bundle_uses_file_resolver_import_candidates() {
        for (specifier, dependency) in [
            ("./filters", "filters/index.mec"),
            ("./filters.v1", "filters.v1.mec"),
            ("fs://lib/geometry.mec", "lib/geometry.mec"),
            ("fs://lib/space%20dep.mec", "lib/space dep.mec"),
        ] {
            let root = temp_root("canonical-file-candidates");
            let loaded = write_demo_project(&root);
            fs::write(
                root.join("demo.mec"),
                format!("+> {specifier}\nanswer := 42\n"),
            )
            .unwrap();
            let dependency = root.join(dependency);
            fs::create_dir_all(dependency.parent().unwrap()).unwrap();
            fs::write(&dependency, "value := 1\n<+ value\n").unwrap();
            let out = root.join("out");
            let mut options = options(&root, &out, loaded);
            options.source_paths.push(dependency);
            bundle_web_project(options).unwrap();
            assert!(out.join("code/demo.mec").is_file());
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn canonical_bundle_prefers_resolved_source_modules_to_catalog_modules() {
        for (import, value) in [
            ("+> math/custom", "custom"),
            ("+> math/*", "custom"),
            ("+> math/{custom, cos}", "cos"),
            ("+> selected := math/custom", "selected"),
        ] {
            let root = temp_root("canonical-source-precedence");
            let loaded = write_demo_project(&root);
            fs::write(
                root.join("demo.mec"),
                format!("{import}\nanswer := {value}\n"),
            )
            .unwrap();
            fs::write(
                root.join("math.mec"),
                "custom := 42\ncos := 42\n<+ custom\n<+ cos\n",
            )
            .unwrap();
            let out = root.join("out");
            let source_paths = vec![root.join("demo.mec"), root.join("math.mec")];
            let mut options = options(&root, &out, loaded.clone());
            options.source_paths = source_paths.clone();
            bundle_web_project(options).unwrap();
            let bundle = compile_test_bundle(&root, &loaded, &source_paths);
            let mut runtime = mech_runtime::RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .build()
                .unwrap();
            let durability = runtime.config().resident_durability;
            let loaded = runtime
                .load_bytecode_program(&bundle.bytecode, durability)
                .unwrap();
            assert_eq!(
                loaded.initial_value.format_canonical_inline(),
                "42",
                "{import}"
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn canonical_source_module_errors_do_not_fall_back_to_catalog_functions() {
        for source in [
            "+> math\nanswer := math/cos(0.0)\n",
            "+> math/cos\nanswer := cos(0.0)\n",
            "+> ./math.mec\nanswer := math/cos(0.0)\n",
            "+> ./math\nanswer := math/cos(0.0)\n",
        ] {
            let root = temp_root("source-module-no-catalog-fallback");
            let loaded = write_demo_project(&root);
            fs::write(root.join("demo.mec"), source).unwrap();
            fs::write(root.join("math.mec"), "custom := 42\n<+ custom\n").unwrap();
            let out = root.join("out");
            let mut options = options(&root, &out, loaded);
            options.source_paths.push(root.join("math.mec"));
            assert!(bundle_web_project(options).is_err(), "{source}");
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn bundle_web_rejects_multiple_run_roots_before_emitting_assets() {
        let root = temp_root("multiple-run-roots");
        let mut loaded = write_demo_project(&root);
        fs::write(root.join("second.mec"), "answer := 2\n").unwrap();
        loaded
            .document
            .run
            .as_mut()
            .unwrap()
            .paths
            .push("second.mec".into());
        let out = root.join("out");
        let mut options = options(&root, &out, loaded);
        options.source_paths.push(root.join("second.mec"));
        let error = bundle_web_project(options).unwrap_err().display_message();
        assert!(error.contains("exactly one run root"), "{error}");
        assert!(!out.join("style.css").exists());
        assert!(!out.join("_mech/project-sources.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_bundle_serves_prose_without_requiring_an_artifact() {
        let root = temp_root("canonical-presentation-only");
        let loaded = write_demo_project(&root);
        fs::write(
            root.join("notes.mec"),
            "These are presentation-only notes.\n",
        )
        .unwrap();
        let out = root.join("out");
        let mut options = options(&root, &out, loaded);
        options.source_paths.push(root.join("notes.mec"));
        bundle_web_project(options).unwrap();
        assert!(out.join("code/demo.mec").is_file());
        assert!(!out.join("code/notes.mec").exists());
        assert!(out.join("source/notes.mec").is_file());
        assert!(out.join("html/notes.html").is_file());
        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(out.join("_mech/project-sources.json")).unwrap(),
        )
        .unwrap();
        let notes = manifest["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| source["specifier"] == "notes.mec")
            .unwrap();
        assert!(notes.get("documentUrl").is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn canonical_bundle_rejects_missing_and_cyclic_dependencies() {
        for cyclic in [false, true] {
            let root = temp_root("canonical-invalid-imports");
            let loaded = write_demo_project(&root);
            fs::write(root.join("demo.mec"), "+> ./dep.mec\nanswer := 42\n").unwrap();
            let out = root.join("out");
            let mut options = options(&root, &out, loaded);
            if cyclic {
                fs::write(
                    root.join("dep.mec"),
                    "+> ./demo.mec\nvalue := 1\n<+ value\n",
                )
                .unwrap();
                options.source_paths.push(root.join("dep.mec"));
            }
            let error = bundle_web_project(options).unwrap_err().display_message();
            assert!(
                error.contains(if cyclic { "cycle" } else { "absent" }),
                "{error}"
            );
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn canonical_bundle_compiler_installs_configured_timer_contracts() {
        let root = temp_root("canonical-provider");
        let mut loaded = write_demo_project(&root);
        loaded
            .document
            .hosts
            .push(mech_runtime::HostInstanceConfig {
                name: "clock".to_owned(),
                provider: "timer".to_owned(),
                settings: mech_runtime::ConfigValue::Map(std::collections::BTreeMap::new()),
            });
        fs::write(
            root.join("demo.mec"),
            "@clock := timer://clock/tick{:read(tick)}\nvalue := @clock/tick\n",
        )
        .unwrap();
        let out = root.join("out");
        bundle_web_project(options(&root, &out, loaded.clone())).unwrap();
        let bundle = compile_test_bundle(&root, &loaded, &[root.join("demo.mec")]);
        let artifact = mech_engine::decode_program_artifact_bytecode_v1(&bundle.bytecode).unwrap();
        assert!(artifact.inputs().is_empty());
        assert!(artifact.requirements().iter().any(|(_, requirement)| matches!(requirement, mech_core::ApplicationRequirement::Resource(request) if request.base_uri == "timer://clock/tick")));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_requires_source_paths() {
        let root = temp_root("requires-source-paths");
        let loaded = write_demo_project(&root);
        let out = root.join("out");
        let mut options = options(&root, &out, loaded);
        options.source_paths.clear();

        let error = format!("{:?}", bundle_web_project(options).unwrap_err());
        assert!(error.contains("bundle-web requires serve.paths"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_requires_run_paths() {
        let root = temp_root("requires-run-paths");
        let mut loaded = write_demo_project(&root);
        loaded.document.run = None;
        let out = root.join("out");

        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &out, loaded)).unwrap_err()
        );
        assert!(error.contains("bundle-web requires run.paths"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wasm_package_validator_accepts_static_served_project_wrapper() {
        let root = temp_root("wasm-package-valid");
        let package = root.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("mech_wasm.js"), STATIC_WASM_WRAPPER).unwrap();
        fs::write(package.join("mech_wasm_bg.wasm"), b"wasm").unwrap();

        validate_static_bundle_wasm_package(&package).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wasm_package_validator_rejects_missing_javascript() {
        let root = temp_root("wasm-package-missing-js");
        let package = root.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("mech_wasm_bg.wasm"), b"wasm").unwrap();

        let error = format!(
            "{:?}",
            validate_static_bundle_wasm_package(&package).unwrap_err()
        );
        assert!(error.contains("mech_wasm.js"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wasm_package_validator_rejects_missing_wasm() {
        let root = temp_root("wasm-package-missing-wasm");
        let package = root.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(package.join("mech_wasm.js"), STATIC_WASM_WRAPPER).unwrap();

        let error = format!(
            "{:?}",
            validate_static_bundle_wasm_package(&package).unwrap_err()
        );
        assert!(error.contains("mech_wasm_bg.wasm"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wasm_package_validator_rejects_missing_wasm_project_export() {
        let root = temp_root("wasm-package-missing-project");
        let package = root.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("mech_wasm.js"),
            "export default async function init() {}\n",
        )
        .unwrap();
        fs::write(package.join("mech_wasm_bg.wasm"), b"wasm").unwrap();

        let error = format!(
            "{:?}",
            validate_static_bundle_wasm_package(&package).unwrap_err()
        );
        assert!(error.contains("static served-project support"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wasm_package_validator_rejects_missing_served_document_export() {
        let root = temp_root("wasm-package-missing-bundle");
        let package = root.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("mech_wasm.js"),
            "export class WasmProject {}\n",
        )
        .unwrap();
        fs::write(package.join("mech_wasm_bg.wasm"), b"wasm").unwrap();

        let error = format!(
            "{:?}",
            validate_static_bundle_wasm_package(&package).unwrap_err()
        );
        assert!(error.contains("static served-project support"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wasm_package_validator_rejects_missing_resolution_transport_capability() {
        let root = temp_root("wasm-package-missing-resolutions");
        let package = root.join("pkg");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("mech_wasm.js"),
            r#"export class WasmProject {
  static fromServedDocuments() {}
  static supportsServedAuthority() { return true; }
}
"#,
        )
        .unwrap();
        fs::write(package.join("mech_wasm_bg.wasm"), b"wasm").unwrap();

        let error = format!(
            "{:?}",
            validate_static_bundle_wasm_package(&package).unwrap_err()
        );
        assert!(error.contains("static served-project support"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wasm_package_incompatibility_leaves_output_untouched() {
        let root = temp_root("wasm-package-no-output");
        let loaded = write_demo_project(&root);
        fs::write(
            root.join("pkg/mech_wasm.js"),
            "export class WasmProject {}\n",
        )
        .unwrap();
        let out = root.join("out");

        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &out, loaded)).unwrap_err()
        );

        assert!(error.contains("static served-project support"));
        assert!(!out.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn browser_source_admission_decline_does_not_publish_any_files() {
        let root = temp_root("browser-language-no-output");
        let loaded = write_demo_project(&root);
        // A package probe, not a native compiler failure. The ordinary source
        // and native catalog are valid, but the selected target declines them.
        fs::write(root.join("pkg/mech_wasm.js"), STATIC_WASM_WRAPPER.replace(
            "static validateStaticSources() { return true; }",
            "static validateStaticSources() { throw new Error('target language does not support u8'); }",
        )).unwrap();
        let out = root.join("out");
        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &out, loaded.clone())).unwrap_err()
        );
        assert!(
            error.contains("target language does not support u8"),
            "{error}"
        );
        assert!(!out.exists());
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("accepted.txt"), "previous accepted output").unwrap();
        assert!(bundle_web_project(options(&root, &out, loaded)).is_err());
        assert_eq!(
            fs::read_to_string(out.join("accepted.txt")).unwrap(),
            "previous accepted output"
        );
        assert_eq!(fs::read_dir(&out).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publication_uses_the_admitted_package_and_retained_config_bytes() {
        let root = temp_root("frozen-browser-package");
        let loaded = write_demo_project(&root);
        let config = fs::read_to_string(&loaded.path).unwrap();
        let js = root.join("pkg/mech_wasm.js");
        fs::create_dir_all(root.join("pkg/snippets/fixture")).unwrap();
        fs::write(
            root.join("pkg/snippets/fixture/helper.js"),
            "export const answer = 42;\n",
        )
        .unwrap();
        let admission = format!(
            "static validateStaticSources() {{ if (answer !== 42) throw new Error('missing helper'); writeFileSync({}, 'rebuilt package'); writeFileSync({}, 'changed configuration'); return true; }}",
            serde_json::to_string(&js).unwrap(),
            serde_json::to_string(&loaded.path).unwrap(),
        );
        let wrapper = format!(
            "import {{writeFileSync}} from 'node:fs';\nimport {{answer}} from './snippets/fixture/helper.js';\n{}",
            STATIC_WASM_WRAPPER.replace(
                "static validateStaticSources() { return true; }",
                &admission
            )
        );
        fs::write(&js, &wrapper).unwrap();
        let out = root.join("out");
        bundle_web_project(options(&root, &out, loaded)).unwrap();
        assert_eq!(fs::read_to_string(js).unwrap(), "rebuilt package");
        assert_eq!(
            fs::read_to_string(out.join("pkg/mech_wasm.js")).unwrap(),
            wrapper
        );
        assert_eq!(
            fs::read_to_string(out.join("pkg/snippets/fixture/helper.js")).unwrap(),
            "export const answer = 42;\n"
        );
        assert_eq!(fs::read_to_string(out.join("mech.mcfg")).unwrap(), config);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires an actual browser_project WASM package in MECH_STATIC_TEST_WASM_PKG"]
    fn actual_browser_package_admits_compatible_source_and_refuses_missing_language() {
        let root = temp_root("actual-target-language");
        let loaded = write_demo_project(&root);
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = 'published-package'\nversion = '1.0.0'\n",
        )
        .unwrap();
        let package = PathBuf::from(
            std::env::var_os("MECH_STATIC_TEST_WASM_PKG").expect("actual browser package path"),
        );
        let out = root.join("out");
        // Native full-source admission is deliberately broader than this
        // stock package. The actual publisher must expose the target decline.
        fs::write(
            root.join("demo.mec"),
            "+> math\nvalue := math/fmod(5.3, 2.0)\n",
        )
        .unwrap();
        compile_test_bundle(&root, &loaded, &[root.join("demo.mec")]);
        let mut requested = options(&root, &out, loaded.clone());
        requested.wasm_pkg = package.clone();
        let error = format!("{:?}", bundle_web_project(requested).unwrap_err());
        assert!(error.contains("source closure"), "{error}");
        assert!(error.contains("math/fmod"), "{error}");
        assert!(!out.exists());
        // A compatible retry succeeds using this same selected package,
        // including nominal metadata and non-default scalar schemas.
        fs::write(root.join("demo.mec"), "<event> := :idle | :busy\nstate<event> := :idle\nvalue<u8> := 7\nother<i32> := -7\nvalue\n").unwrap();
        let mut requested = options(&root, &out, loaded);
        requested.wasm_pkg = package;
        bundle_web_project(requested).unwrap();
        assert!(out.join("_mech/project-sources.json").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn static_manifest_transports_retained_nominal_provenance() {
        let root = temp_root("static-nominal-origin");
        let loaded = write_demo_project(&root);
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = 'published-package'\nversion = '1.0.0'\n",
        )
        .unwrap();
        let out = root.join("out");
        fs::write(
            root.join("demo.mec"),
            "<event> := :idle | :busy\nstate<event> := :idle\nstate\n",
        )
        .unwrap();
        let (_, retained, _) =
            planning::retained_sources(&[root.join("demo.mec")], &root, &root).unwrap();
        let document = &retained["bundle:///demo.mec"];
        bundle_web_project(options(&root, &out, loaded)).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(out.join("_mech/project-sources.json")).unwrap())
                .unwrap();
        let source = &manifest["sources"][0];
        assert_eq!(
            source["nominalOrigin"],
            serde_json::to_value(document.nominal_origin().unwrap()).unwrap()
        );
        assert_eq!(
            source["nominalPackageId"],
            document.nominal_package_id().unwrap()
        );
        assert!(!manifest.to_string().contains(root.to_str().unwrap()));
        let native = compile_test_bundle(
            &root,
            &crate::load_mech_config_path(root.join("demo.mcfg"), Some(root.clone())).unwrap(),
            &[root.join("demo.mec")],
        );
        let native = mech_engine::decode_program_artifact_bytecode_v1(&native.bytecode).unwrap();
        let origins = source["nominalOrigin"].clone();
        let origin: CanonicalNominalPath = serde_json::from_value(origins).unwrap();
        let mut resolver = mech_runtime::InMemorySourceResolver::new();
        resolver
            .insert_source(
                "demo.mec",
                mech_runtime::ResolvedSource::new(
                    "demo.mec",
                    "memory:demo.mec",
                    MechSourceCode::String(fs::read_to_string(root.join("demo.mec")).unwrap()),
                )
                .with_nominal_origin(origin)
                .with_nominal_package_id(source["nominalPackageId"].as_str().unwrap())
                .retain_source_document(mech_syntax::document::Revision(0), Default::default())
                .unwrap()
                .admit_canonical_document()
                .unwrap(),
            )
            .unwrap();
        let product = mech_runtime::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .source_resolver(resolver)
            .build_compiler()
            .unwrap()
            .compile_canonical_interactive_root(mech_runtime::SourceRequest::new("demo.mec"))
            .unwrap();
        let nominal = |artifact: &mech_engine::ProgramArtifact| {
            artifact
                .schemas()
                .entries()
                .filter_map(|entry| {
                    if let SchemaBody::Enum { key, .. } = entry.schema().body() {
                        Some((*key, entry.key()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        assert!(!nominal(&native).is_empty());
        assert_eq!(nominal(&native), nominal(product.artifact()));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn incomplete_comment_markup_does_not_hide_the_following_program() {
        let root = temp_root("incomplete-comments");
        let loaded = write_demo_project(&root);
        let source = "-- use { here\n// unmatched `code\n-- TODO [ <script>alert(1)</script>\nanswer := 42\nanswer\n";
        fs::write(root.join("demo.mec"), source).unwrap();
        let bundle = compile_test_bundle(&root, &loaded, &[root.join("demo.mec")]);
        let mut runtime = mech_runtime::RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap();
        runtime
            .load_bytecode_program(&bundle.bytecode, runtime.config().resident_durability)
            .unwrap();
        assert_eq!(
            runtime
                .root_symbol_value("answer")
                .unwrap()
                .format_canonical_inline(),
            "42"
        );
        let out = root.join("out");
        bundle_web_project(options(&root, &out, loaded)).unwrap();
        assert_eq!(decode_browser_payload(&out).source(), source);
        let html = fs::read_to_string(out.join("html/demo.html")).unwrap();
        assert!(!html.contains("<script>alert(1)</script>"));
        let document = mech_runtime::SourceDocument::parse_resolved(
            "demo.mec",
            mech_syntax::document::Revision(0),
            source,
            Default::default(),
        )
        .unwrap();
        let highlighted = mech_runtime::CanonicalDocumentRenderer
            .render_repl_source_html(&document.document())
            .unwrap()
            .expect("complete root source must render");
        assert!(highlighted.contains("here"));
        assert!(highlighted.contains("&lt;script&gt;"));
        assert!(!highlighted.contains("<script>alert(1)</script>"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_writes_source_code_and_html() {
        let root = temp_root("writes");
        let loaded = write_demo_project(&root);
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        assert!(out.join("index.html").is_file());
        assert!(out.join("style.css").is_file());
        assert!(out.join("pkg/mech_wasm.js").is_file());
        assert!(out.join("pkg/mech_wasm_bg.wasm").is_file());
        assert!(out.join("mech.mcfg").is_file());
        assert!(out.join("_mech/project.js").is_file());
        assert!(out.join("_mech/project-sources.json").is_file());
        assert!(out.join("source/demo.mec").is_file());
        assert!(out.join("code/demo.mec").is_file());
        assert!(out.join("html/demo.html").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_static_project_bootstrap_uses_served_documents() {
        let root = temp_root("static-bootstrap");
        let loaded = write_demo_project(&root);
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        let index = fs::read_to_string(out.join("index.html")).unwrap();
        let bootstrap = fs::read_to_string(out.join("_mech/project.js")).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(out.join("_mech/project-sources.json")).unwrap())
                .unwrap();
        assert!(index.contains("src=\"./_mech/project.js\""));
        assert!(index.contains("window.__MECH_HOST_CONFIG"));
        assert!(bootstrap.contains("WasmProject.fromServedDocuments"));
        assert!(bootstrap.contains("WasmProject.supportsServedDocumentResolutions"));
        assert!(bootstrap.contains("../pkg/mech_wasm.js"));
        assert_eq!(manifest["version"], 4);
        assert_eq!(manifest["roots"], serde_json::json!(["demo.mec"]));
        assert_eq!(manifest["sources"][0]["specifier"], "demo.mec");
        assert_eq!(manifest["sources"][0]["url"], "source/demo.mec");
        assert_eq!(manifest["sources"][0]["documentUrl"], "code/demo.mec");
        assert_eq!(manifest["resolutions"], serde_json::json!([]));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_rewrites_external_run_paths_to_bundle_local_roots() {
        let root = temp_root("external-run-roots");
        let app = root.join("app");
        let config = root.join("config");
        fs::create_dir_all(app.join("src")).unwrap();
        fs::create_dir_all(app.join("pkg")).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(
            config.join("demo.mcfg"),
            r#"config := {
  runtime: {name: "external-run-roots"}
  serve: {
    paths: ["../app/src"]
    shim: "../app/index.html"
    wasm: "../app/pkg"
  }
  run: {paths: ["../app/src/main.mec"]}
}
"#,
        )
        .unwrap();
        fs::write(app.join("index.html"), "<html><body></body></html>").unwrap();
        fs::write(
            app.join("src/main.mec"),
            "+> ./dep.mec\nanswer := dep/value + 1\n",
        )
        .unwrap();
        fs::write(app.join("src/dep.mec"), "value := 41\n<+ value\n").unwrap();
        fs::write(app.join("pkg/mech_wasm.js"), STATIC_WASM_WRAPPER).unwrap();
        fs::write(app.join("pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
        let loaded =
            crate::load_mech_config_path(config.join("demo.mcfg"), Some(app.clone())).unwrap();
        let options = BundleWebOptions {
            project_dir: app.clone(),
            output_dir: root.join("out"),
            source_paths: vec![app.join("src/main.mec"), app.join("src/dep.mec")],
            shim_path: app.join("index.html"),
            stylesheet_paths: Vec::new(),
            wasm_pkg: app.join("pkg"),
            loaded_config: loaded,
            host_config_injection: None,
        };

        bundle_web_project(options).unwrap();

        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("out/_mech/project-sources.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["version"], 4);
        assert_eq!(manifest["roots"], serde_json::json!(["src/main.mec"]));
        assert!(
            manifest["sources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|source| source["specifier"] == "src/main.mec")
        );
        assert!(
            manifest["sources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|source| source["specifier"] == "src/dep.mec")
        );
        assert_eq!(
            manifest["resolutions"],
            serde_json::json!([{
                "referrer": "src/main.mec",
                "specifier": "./dep.mec",
                "target": "src/dep.mec",
            }])
        );
        assert!(!manifest.to_string().contains("../"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_code_payload_decodes() {
        let root = temp_root("decodes");
        let loaded = write_demo_project(&root);
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        let source = fs::read_to_string(root.join("demo.mec")).unwrap();
        let encoded = fs::read_to_string(out.join("code/demo.mec")).unwrap();
        let decoded = mech_runtime::BrowserDocumentPayload::decode(&encoded).unwrap();
        assert_eq!(decoded.source(), source);
        assert_eq!(decoded.root_specifier(), "demo.mec");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_injects_browser_host_config() {
        let root = temp_root("host-config");
        let loaded = write_demo_project(&root);
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        let index = fs::read_to_string(out.join("index.html")).unwrap();
        let source_html = fs::read_to_string(out.join("html/demo.html")).unwrap();
        assert!(index.contains("window.__MECH_HOST_CONFIG"));
        assert!(source_html.contains("window.__MECH_HOST_CONFIG"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_injection_preserves_browser_alias_and_run_grants() {
        let root = temp_root("host-alias-config");
        let loaded = write_browser_alias_project(&root);
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        let index = fs::read_to_string(out.join("index.html")).unwrap();
        let source_html = fs::read_to_string(out.join("html/demo.html")).unwrap();
        for html in [&index, &source_html] {
            assert!(html.contains("window.__MECH_HOST_CONFIG"));
            assert!(html.contains("\"hosts\""));
            assert!(html.contains("\"name\":\"ui\""));
            assert!(html.contains("\"name\":\"browser\""));
            assert!(html.contains("\"runGrants\""));
            assert!(html.contains("\"target\":\"ui/dom\""));
            assert!(html.contains("body/content/allowed/_value"));
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_copies_static_assets() {
        let root = temp_root("static-assets");
        let loaded = write_demo_project(&root);
        let out = root.join("out");
        fs::write(root.join("app.js"), "console.log('app');\n").unwrap();
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("assets/logo.svg"), "<svg></svg>\n").unwrap();

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        assert!(out.join("app.js").is_file());
        assert!(out.join("assets/logo.svg").is_file());
        assert!(!out.join("demo.mcfg").exists());
        assert!(!out.join("demo.mec").exists());
        assert!(out.join("source/demo.mec").is_file());
        assert!(out.join("code/demo.mec").is_file());
        assert!(out.join("html/demo.html").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn bundle_web_preserves_static_file_symlink_output_identity() {
        use std::os::unix::fs as unix_fs;

        let root = temp_root("static-symlink-file-identity");
        let loaded = write_demo_project(&root);
        let out = root.join("out");
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("assets/favicon.ico"), b"icon").unwrap();
        unix_fs::symlink("assets/favicon.ico", root.join("favicon.ico")).unwrap();
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head><link rel="icon" href="./favicon.ico"></head><body><script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec");</script></body></html>"#,
    )
    .unwrap();

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        assert_eq!(fs::read(out.join("favicon.ico")).unwrap(), b"icon");
        assert!(out.join("assets/favicon.ico").is_file());
        let index = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(index.contains("./favicon.ico"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn bundle_web_rejects_static_file_symlink_target_outside_project() {
        use std::os::unix::fs as unix_fs;

        let root = temp_root("static-symlink-outside-target");
        let project = root.join("project");
        let secrets = root.join("secrets");
        fs::create_dir_all(project.join("pkg")).unwrap();
        fs::create_dir_all(project.join("public")).unwrap();
        fs::create_dir_all(&secrets).unwrap();
        fs::write(
            project.join("demo.mcfg"),
            r#"config := {
  runtime: {name: "bundle-test"}
  serve: {
    paths: ["demo.mec"]
    shim: "index.html"
    wasm: "pkg"
  }
  run: {paths: ["demo.mec"]}
}
"#,
        )
        .unwrap();
        fs::write(
      project.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec");</script></body></html>"#,
    )
    .unwrap();
        fs::write(project.join("demo.mec"), "x := 1\n").unwrap();
        fs::write(project.join("pkg/mech_wasm.js"), STATIC_WASM_WRAPPER).unwrap();
        fs::write(project.join("pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
        fs::write(secrets.join("settings.json"), r#"{"secret":true}"#).unwrap();
        unix_fs::symlink(
            "../../secrets/settings.json",
            project.join("public/settings.json"),
        )
        .unwrap();
        let loaded =
            crate::load_mech_config_path(project.join("demo.mcfg"), Some(project.clone())).unwrap();
        let out = project.join("out");

        let error = format!(
            "{:?}",
            bundle_web_project(options(&project, &out, loaded)).unwrap_err()
        );

        assert!(error.contains("bundle-web static asset target is outside project root"));
        assert!(!out.join("public/settings.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn bundle_web_skips_non_static_symlinks_to_outside_project() {
        use std::os::unix::fs as unix_fs;

        let root = temp_root("non-static-symlinks-outside-project");
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(root.join("README"), "outside readme\n").unwrap();
        fs::write(root.join(".env"), "SECRET=true\n").unwrap();
        unix_fs::symlink("../README", project.join("README")).unwrap();
        unix_fs::symlink("../.env", project.join(".env")).unwrap();
        let loaded = write_demo_project(&project);
        let out = project.join("out");

        bundle_web_project(options(&project, &out, loaded)).unwrap();

        assert!(!out.join("README").exists());
        assert!(!out.join(".env").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn bundle_web_skips_static_symlinked_directories() {
        use std::os::unix::fs as unix_fs;

        let root = temp_root("static-symlink-dir-skip");
        let loaded = write_demo_project(&root);
        let out = root.join("out");
        fs::create_dir_all(root.join("real_assets/nested")).unwrap();
        fs::write(root.join("real_assets/nested/logo.svg"), "<svg></svg>\n").unwrap();
        unix_fs::symlink("real_assets", root.join("linked_assets")).unwrap();

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        assert!(out.join("real_assets/nested/logo.svg").is_file());
        assert!(!out.join("linked_assets/nested/logo.svg").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_output_inside_project_does_not_copy_itself() {
        let root = temp_root("output-inside-project");
        let loaded = write_demo_project(&root);
        let out = root.join("dist/bundle");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("stale.js"), "console.log('stale');\n").unwrap();

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        assert!(out.join("stale.js").is_file());
        assert!(!out.join("dist/bundle/stale.js").exists());
        assert!(!out.join("bundle/stale.js").exists());
        assert!(out.join("source/demo.mec").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_rejects_output_equal_project_root() {
        let root = temp_root("output-project-root");
        let loaded = write_demo_project(&root);
        let original_index = fs::read_to_string(root.join("index.html")).unwrap();

        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &root, loaded)).unwrap_err()
        );

        assert!(error.contains("bundle-web output directory must not be the project root"));
        assert_eq!(
            fs::read_to_string(root.join("index.html")).unwrap(),
            original_index
        );
        assert!(!root.join("style.css").exists());
        assert!(!root.join("source/demo.mec").exists());
        assert!(!root.join("code/demo.mec").exists());
        assert!(!root.join("html/demo.html").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_rejects_output_equal_config_base_dir() {
        let root = temp_root("output-config-base");
        let app = root.join("app");
        let config = root.join("config");
        fs::create_dir_all(app.join("pkg")).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(
      app.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec");</script></body></html>"#,
    )
    .unwrap();
        fs::write(app.join("demo.mec"), "x := 1\n").unwrap();
        fs::write(app.join("pkg/mech_wasm.js"), STATIC_WASM_WRAPPER).unwrap();
        fs::write(app.join("pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
        fs::write(
            config.join("demo.mcfg"),
            r#"config := {
  runtime: {name: "bundle-config-base-test"}
  serve: {
    paths: ["../app/demo.mec"]
    shim: "../app/index.html"
    wasm: "../app/pkg"
  }
}
"#,
        )
        .unwrap();
        let loaded =
            crate::load_mech_config_path(config.join("demo.mcfg"), Some(config.clone())).unwrap();
        let options = BundleWebOptions {
            project_dir: app.clone(),
            output_dir: config.clone(),
            source_paths: vec![app.join("demo.mec")],
            shim_path: app.join("index.html"),
            stylesheet_paths: Vec::new(),
            wasm_pkg: app.join("pkg"),
            loaded_config: loaded,
            host_config_injection: None,
        };

        let error = format!("{:?}", bundle_web_project(options).unwrap_err());

        assert!(
            error.contains("bundle-web output directory must not be the config base directory")
        );
        assert!(config.join("demo.mcfg").is_file());
        assert!(!config.join("style.css").exists());
        assert!(!config.join("source/demo.mec").exists());
        assert!(!config.join("code/demo.mec").exists());
        assert!(!config.join("html/demo.html").exists());
        assert!(!config.join("pkg/mech_wasm.js").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_rejects_root_relative_code_url() {
        let root = temp_root("root-code-url");
        let loaded = write_demo_project(&root);
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script>fetch("/code/demo.mec")</script></body></html>"#,
    )
    .unwrap();
        let out = root.join("out");

        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &out, loaded)).unwrap_err()
        );

        assert!(error.contains("bundle-web shim contains server-root Mech URL"));
        assert!(error.contains("./code/"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_rejects_source_document_controller_shims() {
        let root = temp_root("document-controller-shim");
        let loaded = write_demo_project(&root);
        fs::write(
            root.join("index.html"),
            r#"<!doctype html><html><body>{{DOCUMENT_SCRIPT}}</body></html>"#,
        )
        .unwrap();
        let out = root.join("out");

        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &out, loaded)).unwrap_err()
        );

        assert!(error.contains("bundle-web does not support source-document controller shims"));
        assert!(error.contains("./_mech/project.js"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_rejects_root_relative_pkg_url() {
        let root = temp_root("root-pkg-url");
        let loaded = write_demo_project(&root);
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "/pkg/mech_wasm.js";</script></body></html>"#,
    )
    .unwrap();
        let out = root.join("out");

        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &out, loaded)).unwrap_err()
        );

        assert!(error.contains("bundle-web shim contains server-root Mech URL"));
        assert!(error.contains("./pkg/mech_wasm.js"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_rejects_root_relative_pkg_url_with_query() {
        let root = temp_root("root-pkg-query-url");
        let loaded = write_demo_project(&root);
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "/pkg/mech_wasm.js?v=123";</script></body></html>"#,
    )
    .unwrap();
        let out = root.join("out");

        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &out, loaded)).unwrap_err()
        );

        assert!(error.contains("bundle-web shim contains server-root Mech URL"));
        assert!(error.contains("./pkg/mech_wasm.js"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_rejects_root_relative_pkg_url_with_fragment() {
        let root = temp_root("root-pkg-fragment-url");
        let loaded = write_demo_project(&root);
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from '/pkg/mech_wasm.js#hash';</script></body></html>"#,
    )
    .unwrap();
        let out = root.join("out");

        let error = format!(
            "{:?}",
            bundle_web_project(options(&root, &out, loaded)).unwrap_err()
        );

        assert!(error.contains("bundle-web shim contains server-root Mech URL"));
        assert!(error.contains("./pkg/mech_wasm.js"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_does_not_reject_ordinary_external_urls() {
        let root = temp_root("external-urls");
        let loaded = write_demo_project(&root);
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body>
<a href="https://example.com/code/demo.mec">external</a>
<a href="http://localhost:8081/code/demo.mec">local</a>
<script src="//cdn.example.com/pkg/mech_wasm.js"></script>
<a href="mailto:test@example.com">mail</a>
<img src="data:text/plain,hello" />
<script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec"); const source = await fetch("./source/demo.mec");</script>
</body></html>"#,
    )
    .unwrap();
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        assert!(out.join("index.html").is_file());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundle_web_copies_config_as_mech_mcfg() {
        let root = temp_root("config-copy");
        let loaded = write_demo_project(&root);
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        assert_eq!(
            fs::read_to_string(out.join("mech.mcfg")).unwrap(),
            fs::read_to_string(root.join("demo.mcfg")).unwrap(),
        );
        assert!(!out.join("demo.mcfg").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn source_page_shim_rebases_relative_bundle_assets_by_depth() {
        let shim = r#"<script type="module">import init from "./pkg/mech_wasm.js"; await fetch("./style.css"); await fetch("./code/demo.mec"); await fetch("./source/demo.mec");</script>"#;

        let root = rebase_bundle_shim_for_depth(shim, 0);
        assert!(root.contains("./pkg/mech_wasm.js"));
        assert!(root.contains("./style.css"));

        let one = rebase_bundle_shim_for_depth(shim, 1);
        assert!(one.contains("../pkg/mech_wasm.js"));
        assert!(one.contains("../style.css"));

        let two = rebase_bundle_shim_for_depth(shim, 2);
        assert!(two.contains("../../pkg/mech_wasm.js"));
        assert!(two.contains("../../style.css"));
    }

    #[test]
    fn static_project_bootstrap_recognizes_quoted_and_cache_busted_urls() {
        for shim in [
            r#"<script src="./_mech/project.js"></script>"#,
            r#"<script src='./_mech/project.js'></script>"#,
            r#"<script src = "./_mech/project.js"></script>"#,
            r#"<script src = './_mech/project.js'></script>"#,
            r#"<script src="./_mech/project.js?v=1"></script>"#,
            r#"<script src="./_mech/project.js#release"></script>"#,
            r#"<script src="./_mech/project.js?v=1#release"></script>"#,
        ] {
            assert_eq!(ensure_static_project_bootstrap(shim), shim);
        }
    }

    #[test]
    fn static_project_bootstrap_appends_for_nonmatching_script() {
        let shim = r#"<script src="./assets/project.js"></script>"#;
        let result = ensure_static_project_bootstrap(shim);

        assert!(result.contains("./assets/project.js"));
        assert!(result.contains("src=\"./_mech/project.js\""));
    }

    #[test]
    fn source_page_shim_rebases_single_quoted_project_base() {
        let shim =
            r#"<script type="module" src='./_mech/project.js' data-mech-project='.'></script>"#;
        let rebased = rebase_bundle_shim_for_depth(shim, 2);

        assert!(rebased.contains("src='../../_mech/project.js'"));
        assert!(rebased.contains("data-mech-project='../../'"));
    }

    #[test]
    fn source_page_shim_rebases_double_quoted_project_base() {
        let shim =
            r#"<script type="module" src="./_mech/project.js" data-mech-project="."></script>"#;
        let rebased = rebase_bundle_shim_for_depth(shim, 2);

        assert!(rebased.contains("src=\"../../_mech/project.js\""));
        assert!(rebased.contains("data-mech-project=\"../../\""));
    }

    #[test]
    fn source_page_shim_adds_project_base_to_custom_bootstrap() {
        let shim = r#"<script type="module" src="./_mech/project.js?v=1"></script>"#;
        let rebased = rebase_bundle_shim_for_depth(shim, 2);

        assert!(rebased.contains("src=\"../../_mech/project.js?v=1\""));
        assert!(rebased.contains("data-mech-project=\"../../\""));
    }

    #[test]
    fn bundle_web_rebases_source_shim_before_injecting_host_config() {
        let root = temp_root("rebase-before-inject");
        drop(write_demo_project(&root));
        fs::write(
            root.join("demo.mcfg"),
            r#"config := {
  runtime: {name: "bundle-test"}
  serve: {
    paths: ["demo.mec"]
    shim: "index.html"
    wasm: "pkg"
  }
  run: {
    paths: ["demo.mec"]
    grants: [
      {
        target: "browser/dom"
        operations: ["read"]
        paths: ["./source/foo", "./code/foo"]
      }
    ]
  }
}
"#,
        )
        .unwrap();
        let loaded =
            crate::load_mech_config_path(root.join("demo.mcfg"), Some(root.to_path_buf())).unwrap();
        fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "./pkg/mech_wasm.js"; await fetch("./style.css");</script></body></html>"#,
    )
    .unwrap();
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        let index = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(index.contains("./pkg/mech_wasm.js"));
        assert!(index.contains("./style.css"));
        let source = fs::read_to_string(out.join("html/demo.html")).unwrap();
        assert!(source.contains("../pkg/mech_wasm.js"));
        assert!(source.contains("../style.css"));
        assert!(source.contains("./source/foo"));
        assert!(source.contains("./code/foo"));
        assert!(!source.contains("../source/foo"));
        assert!(!source.contains("../code/foo"));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn bundle_web_uses_symlink_path_as_source_route_identity() {
        use std::os::unix::fs as unix_fs;

        let root = temp_root("symlink-route-identity");
        let mut loaded = write_demo_project(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("shared")).unwrap();
        fs::write(root.join("shared/lib.mec"), "answer := 42\n").unwrap();
        unix_fs::symlink("../shared/lib.mec", root.join("src/link.mec")).unwrap();
        let out = root.join("out");
        loaded.document.run.as_mut().unwrap().paths = vec![PathBuf::from("src/link.mec")];
        let mut options = options(&root, &out, loaded);
        options.source_paths = vec![root.join("src/link.mec")];

        bundle_web_project(options).unwrap();

        assert!(out.join("source/src/link.mec").is_file());
        assert!(out.join("code/src/link.mec").is_file());
        assert!(out.join("html/src/link.html").is_file());
        assert!(!out.join("source/shared/lib.mec").exists());
        assert!(!out.join("html/shared/lib.html").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn bundle_web_copies_static_symlink_using_logical_output_path() {
        use std::os::unix::fs as unix_fs;

        let root = temp_root("static-symlink-logical-path");
        let loaded = write_demo_project(&root);
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(root.join("assets/favicon.ico"), "icon").unwrap();
        unix_fs::symlink("assets/favicon.ico", root.join("favicon.ico")).unwrap();
        let out = root.join("out");

        bundle_web_project(options(&root, &out, loaded)).unwrap();

        assert!(out.join("favicon.ico").is_file());
        let html = fs::read_to_string(out.join("index.html")).unwrap();
        assert!(html.contains("./favicon.ico") || !html.contains("favicon.ico"));
        fs::remove_dir_all(root).unwrap();
    }
}
