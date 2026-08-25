use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use colored::*;
use mech_core::*;
use mech_runtime::{
    DefaultIdGenerator, FS_READ, HostFilesystemAuthority, MECH_TOOL_SUBJECT, SharedCapabilityKernel,
};
use mech_syntax::formatter::*;
use mech_syntax::parser;

mod document_bundle;
mod publication;

use document_bundle::resolve_document_source_bundle;
use publication::{PlannedOutput, publish_outputs_recoverably};

use crate::cli::outcome::{CliOutcome, RootFlags};
use crate::cli::resources::{
    LoadedStylesheets, ResourceEvent, ResourceFallback, Utf8ConversionError, WebResourceDefaults,
    html_style_sheets, load_resource, load_stylesheets,
};
use crate::fs_paths::{
    absolute_path, extension_allowed, paths_equivalent, source_extension,
    unsupported_source_path_error,
};
use crate::source_discovery::{
    DiscoveryOptions, MissingPathPolicy, SkipReason, SourceDiscoveryEvent,
    collect_sources_with_events,
};
use crate::{GenericError, MechError, save_to_file};

pub(crate) fn command() -> Command {
    Command::new("format")
        .about("Format Mech source code into standard format.")
        .arg(
            Arg::new("mech_format_file_paths")
                .help("Source .mec/.mdoc files, HTML files, or directories")
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("output_path")
                .short('o')
                .long("out")
                .help("Destination folder.")
                .required(false),
        )
        .arg(
            Arg::new("stylesheet")
                .short('s')
                .long("stylesheet")
                .value_name("STYLESHEET")
                .num_args(1..)
                .action(ArgAction::Append)
                .help("Sets the stylesheet for the HTML output"),
        )
        .arg(
            Arg::new("shim")
                .short('m')
                .long("shim")
                .value_name("SHIM")
                .help("Sets the shim for the HTML output"),
        )
        .arg(
            Arg::new("html")
                .short('t')
                .long("html")
                .required(false)
                .help("Output as HTML")
                .action(ArgAction::SetTrue),
        )
}

fn render_discovery_events(badge: &str, events: &[SourceDiscoveryEvent]) {
    for event in events {
        match event {
            SourceDiscoveryEvent::SkippedBrokenSymlink { path } => {
                println!("{badge} Skipped broken symlink: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedSymlinkedDirectory { path } => {
                println!("{badge} Skipped symlinked directory: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedFileSymlink { path } => {
                println!("{badge} Skipped file symlink: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedUnsupportedExtension { path } => {
                println!("{badge} Skipped unsupported source: {}", path.display())
            }
            SourceDiscoveryEvent::SkippedDirectory { path, reason } => match reason {
                SkipReason::SkippedByName => {
                    println!("{badge} Skipped directory: {}", path.display())
                }
                SkipReason::AlreadyVisited => println!(
                    "{badge} Skipped already visited directory: {}",
                    path.display()
                ),
            },
        }
    }
}

fn render_resource_events(badge: &str, name: &str, events: &[ResourceEvent]) {
    for event in events {
        match event {
            ResourceEvent::LoadedLocal { path } => {
                println!("{badge} Loaded {name}: {}", path.display())
            }
            ResourceEvent::MissingLocalUsedFallback { path, fallback } => match fallback {
                ResourceFallback::EmbeddedDefault => println!(
                    "{badge} {name} not found: {}; using embedded default",
                    path.display()
                ),
                ResourceFallback::RemoteUrl(url) => println!(
                    "{badge} {name} not found: {}; using fallback {url}",
                    path.display()
                ),
            },
            ResourceEvent::LoadedEmbeddedDefault => {
                println!("{badge} Using embedded default {name}")
            }
            ResourceEvent::LoadedRemoteFallback { url } => {
                println!("{badge} Downloaded fallback {name}: {url}")
            }
        }
    }
}

fn document_controller_slots(
    shim: &str,
    document_js: Option<&str>,
    source_url_key: &str,
    wasm_module_url: &str,
    document_sources: &str,
) -> MResult<HtmlShimExtraSlots> {
    let mut slots = HtmlShimExtraSlots::default();
    slots.insert("SOURCE_URL_KEY", source_url_key);
    if !shim.contains("{{DOCUMENT_SCRIPT}}") {
        return Ok(slots);
    }

    let document_js = document_js.ok_or_else(|| format_error(
        "selected HTML shim requests {{DOCUMENT_SCRIPT}}, but the embedded document controller is unavailable",
    ))?;
    if shim.contains("{{WASM_MODULE_URL}}") {
        slots.insert("WASM_MODULE_URL", wasm_module_url);
    } else if !shim.contains("data-mech-wasm-module") {
        return Err(format_error(
            "selected HTML shim requests {{DOCUMENT_SCRIPT}}, but does not provide {{WASM_MODULE_URL}} or an explicit data-mech-wasm-module",
        ));
    }
    if shim.contains("{{DOCUMENT_SOURCES}}") {
        slots.insert("DOCUMENT_SOURCES", document_sources);
    } else {
        return Err(format_error(
            "selected HTML shim requests {{DOCUMENT_SCRIPT}}, but does not provide {{DOCUMENT_SOURCES}} for the standalone source bundle",
        ));
    }
    slots.insert("DOCUMENT_SCRIPT", document_js);
    Ok(slots)
}

fn format_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn common_parent_directory(paths: &[PathBuf]) -> MResult<PathBuf> {
    let first = paths.first().ok_or_else(|| {
        format_error("cannot determine a source bundle directory without source paths")
    })?;
    let first = absolute_path(first)?;
    let mut ancestor = first
        .parent()
        .ok_or_else(|| {
            format_error(format!(
                "source path `{}` has no parent directory",
                first.display()
            ))
        })?
        .to_path_buf();

    for path in paths.iter().skip(1) {
        let path = absolute_path(path)?;
        let parent = path.parent().ok_or_else(|| {
            format_error(format!(
                "source path `{}` has no parent directory",
                path.display()
            ))
        })?;
        while !parent.starts_with(&ancestor) {
            ancestor = ancestor
                .parent()
                .ok_or_else(|| {
                    format_error("formatted document sources have no common filesystem ancestor")
                })?
                .to_path_buf();
        }
    }
    Ok(ancestor)
}

fn relative_asset_url(output_file: &Path, asset_file: &Path) -> MResult<String> {
    let output_parent = absolute_path(output_file)?
        .parent()
        .ok_or_else(|| {
            format_error(format!(
                "formatted output `{}` has no parent directory",
                output_file.display(),
            ))
        })?
        .to_path_buf();
    let asset_file = absolute_path(asset_file)?;
    let output_parts = output_parent.components().collect::<Vec<_>>();
    let asset_parts = asset_file.components().collect::<Vec<_>>();
    let shared = output_parts
        .iter()
        .zip(&asset_parts)
        .take_while(|(left, right)| left == right)
        .count();
    let mut url = String::new();
    for _ in shared..output_parts.len() {
        url.push_str("../");
    }
    for (index, component) in asset_parts[shared..].iter().enumerate() {
        if !url.is_empty() && !url.ends_with('/') {
            url.push('/');
        }
        let text = component
            .as_os_str()
            .to_str()
            .ok_or_else(|| format_error("runtime asset path must be valid UTF-8"))?;
        url.push_str(text);
        if index + 1 < asset_parts[shared..].len() {
            url.push('/');
        }
    }
    Ok(if url.starts_with("../") {
        url
    } else {
        format!("./{url}")
    })
}

fn formatter_asset_package_directory(
    output_path: &Path,
    is_output_file: bool,
    writes_in_place: bool,
    outputs: &[PathBuf],
) -> MResult<PathBuf> {
    let root = if is_output_file {
        absolute_path(output_path)?
            .parent()
            .ok_or_else(|| {
                format_error(format!(
                    "formatted output `{}` has no parent directory",
                    output_path.display(),
                ))
            })?
            .to_path_buf()
    } else if writes_in_place {
        common_parent_directory(outputs)?
    } else {
        absolute_path(output_path)?
    };
    Ok(root.join("_mech").join("pkg"))
}

fn shipped_shim_name(source: &crate::cli::resources::ResourceSource) -> Option<&'static str> {
    match source {
        crate::cli::resources::ResourceSource::EmbeddedDefault
        | crate::cli::resources::ResourceSource::EmptyPathFallback => {
            return Some("include/index.html");
        }
        crate::cli::resources::ResourceSource::LocalPath(path) => {
            let include = Path::new(env!("CARGO_MANIFEST_DIR")).join("include");
            for (file, label) in [
                ("index.html", "include/index.html"),
                ("blog.html", "include/blog.html"),
                ("docs.html", "include/docs.html"),
            ] {
                if include.join(file).canonicalize().ok().as_ref() == Some(path) {
                    return Some(label);
                }
            }
            None
        }
        crate::cli::resources::ResourceSource::RemoteUrl(_) => None,
    }
}

const FORMAT_EXTENSIONS: &[&str] = &["mec", "🤖", "html", "htm", "mdoc"];
const SKIP_SOURCE_DIRS: &[&str] = &["target", ".git", "dist", "out"];

#[derive(Clone, Debug)]
struct CollectedSourceTarget {
    #[cfg(test)]
    input_root: PathBuf,
    path: PathBuf,
    relative_path: PathBuf,
    default_output_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
struct CollectedFormatTargets {
    targets: Vec<CollectedSourceTarget>,
    events: Vec<SourceDiscoveryEvent>,
}

impl Deref for CollectedFormatTargets {
    type Target = [CollectedSourceTarget];

    fn deref(&self) -> &Self::Target {
        &self.targets
    }
}

impl DerefMut for CollectedFormatTargets {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.targets
    }
}

impl IntoIterator for CollectedFormatTargets {
    type Item = CollectedSourceTarget;
    type IntoIter = std::vec::IntoIter<CollectedSourceTarget>;

    fn into_iter(self) -> Self::IntoIter {
        self.targets.into_iter()
    }
}

fn normalize_output_exclusion(
    output_path: &Path,
    is_output_file: bool,
) -> MResult<Option<PathBuf>> {
    if is_output_file {
        return Ok(None);
    }
    let absolute = absolute_path(output_path)?;
    Ok(Some(if absolute.exists() {
        absolute.canonicalize()?
    } else {
        absolute
    }))
}

fn format_output_exclusion(
    output_arg: Option<&str>,
    output_path: &Path,
    is_output_file: bool,
) -> MResult<Option<PathBuf>> {
    match output_arg {
        None => Ok(None),
        Some(".") => Ok(None),
        Some(_) if is_output_file => Ok(None),
        Some(_) => {
            let exclusion = normalize_output_exclusion(output_path, false)?;
            match exclusion {
                Some(path) if path == std::env::current_dir()?.canonicalize()? => Ok(None),
                other => Ok(other),
            }
        }
    }
}

fn format_writes_in_place(
    output_arg: Option<&str>,
    output_path: &Path,
    is_output_file: bool,
) -> MResult<bool> {
    if is_output_file {
        return Ok(false);
    }
    match output_arg {
        None => Ok(true),
        Some(_) => {
            let cwd = std::env::current_dir()?.canonicalize()?;
            let absolute = absolute_path(output_path)?;
            let normalized = if absolute.exists() {
                absolute.canonicalize()?
            } else {
                absolute
            };
            Ok(normalized == cwd)
        }
    }
}

fn is_excluded_output_path(path: &Path, output_exclusion: Option<&Path>) -> MResult<bool> {
    let Some(excluded) = output_exclusion else {
        return Ok(false);
    };
    let absolute = absolute_path(path)?;
    let normalized = if absolute.exists() {
        absolute.canonicalize()?
    } else {
        absolute
    };
    Ok(normalized == excluded || normalized.starts_with(excluded))
}

fn safe_output_relative_path(path: &Path) -> MResult<PathBuf> {
    let cwd = std::env::current_dir()?;
    let candidate = if path.is_absolute() {
        match path.strip_prefix(&cwd) {
            Ok(stripped) => stripped.to_path_buf(),
            Err(_) => {
                return Ok(path
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("output.mec")));
            }
        }
    } else {
        path.to_path_buf()
    };

    let mut safe = PathBuf::new();
    for component in candidate.components() {
        match component {
            std::path::Component::Normal(part) => safe.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Ok(path
                    .file_name()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("output.mec")));
            }
        }
    }

    if safe.as_os_str().is_empty() {
        Ok(path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("output.mec")))
    } else {
        Ok(safe)
    }
}

fn default_output_relative_path(input_root: &Path, path: &Path) -> MResult<PathBuf> {
    let cwd = std::env::current_dir()?;
    if path.is_relative() {
        return Ok(path.to_path_buf());
    }
    if let Ok(stripped) = path.strip_prefix(&cwd) {
        return Ok(stripped.to_path_buf());
    }
    if let Ok(stripped) = path.strip_prefix(input_root) {
        return Ok(input_root.join(stripped));
    }
    Ok(path
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| path.to_path_buf()))
}

fn read_format_source(path: &Path) -> MResult<MechSourceCode> {
    let extension = source_extension(path)
        .ok_or_else(|| unsupported_source_path_error(path, FORMAT_EXTENSIONS))?;
    match extension.as_str() {
        "mec" | "🤖" | "mdoc" => Ok(MechSourceCode::String(std::fs::read_to_string(path)?)),
        "html" | "htm" => Ok(MechSourceCode::Html(std::fs::read_to_string(path)?)),
        _ => Err(unsupported_source_path_error(path, FORMAT_EXTENSIONS)),
    }
}

fn skip_directory_format_source(path: &Path, html: bool, writes_in_place: bool) -> bool {
    html && writes_in_place
        && matches!(
            source_extension(path).as_deref(),
            Some("html") | Some("htm")
        )
}

fn collect_format_targets(
    path: &Path,
    output_exclusion: Option<&Path>,
    html: bool,
    writes_in_place: bool,
) -> MResult<CollectedFormatTargets> {
    if path.is_file() {
        if !extension_allowed(path, FORMAT_EXTENSIONS) {
            return Err(unsupported_source_path_error(path, FORMAT_EXTENSIONS));
        }
        let default_output_path = path.to_path_buf();
        let relative_path = safe_output_relative_path(path)?;
        return Ok(CollectedFormatTargets {
            targets: vec![CollectedSourceTarget {
                #[cfg(test)]
                input_root: path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
                path: path.to_path_buf(),
                relative_path,
                default_output_path,
            }],
            events: Vec::new(),
        });
    }

    if !path.exists() {
        return Err(MechError::new(
            GenericError {
                msg: format!("Source path does not exist: {}", path.display()),
            },
            None,
        )
        .with_compiler_loc());
    }

    if !path.is_dir() {
        return Err(MechError::new(
            GenericError {
                msg: format!(
                    "Source path is neither a file nor directory: {}",
                    path.display()
                ),
            },
            None,
        )
        .with_compiler_loc());
    }

    let discovery = collect_sources_with_events(
        &[path.to_path_buf()],
        path,
        DiscoveryOptions {
            allowed_file_extensions: FORMAT_EXTENSIONS,
            recursive_file_extensions: FORMAT_EXTENSIONS,
            skip_dir_names: SKIP_SOURCE_DIRS,
            follow_file_symlinks: true,
            follow_dir_symlinks: false,
            missing_path_policy: MissingPathPolicy::SkipBrokenSymlink,
        },
    )?;
    let events = discovery.events;
    let mut out = discovery
        .entries
        .into_iter()
        .filter(|entry| !skip_directory_format_source(&entry.logical_path, html, writes_in_place))
        .filter(|entry| {
            is_excluded_output_path(&entry.logical_path, output_exclusion)
                .map(|excluded| !excluded)
                .unwrap_or(false)
        })
        .map(|entry| {
            let default_output_path = default_output_relative_path(path, &entry.logical_path)?;
            Ok(CollectedSourceTarget {
                #[cfg(test)]
                input_root: path.to_path_buf(),
                path: entry.logical_path,
                relative_path: entry.relative_path,
                default_output_path,
            })
        })
        .collect::<MResult<Vec<_>>>()?;
    out.sort_by(|a, b| {
        a.relative_path
            .cmp(&b.relative_path)
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(CollectedFormatTargets {
        targets: out,
        events,
    })
}
fn format_output_matches_input_dir(
    mech_paths: &[String],
    output_path: &Path,
    is_output_file: bool,
) -> MResult<bool> {
    if is_output_file {
        return Ok(false);
    }
    for input in mech_paths {
        let input_path = Path::new(input);
        if input_path.is_dir() && paths_equivalent(input_path, output_path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn reject_ambiguous_matching_output_dir(
    output_matches_input_dir: bool,
    input_count: usize,
    output_path: &Path,
) -> MResult<()> {
    if output_matches_input_dir && input_count > 1 {
        return Err(MechError::new(
      GenericError {
        msg: format!(
          "Output directory `{}` matches one of multiple format inputs. Use in-place formatting without --out, or choose a distinct output directory.",
          output_path.display(),
        ),
      },
      None,
    ).with_compiler_loc());
    }
    Ok(())
}

fn format_output_file_for_target(
    target: &CollectedSourceTarget,
    output_path: &Path,
    is_output_file: bool,
    writes_in_place: bool,
    html: bool,
) -> PathBuf {
    let mut path = if is_output_file {
        output_path.to_path_buf()
    } else if writes_in_place {
        target.default_output_path.clone()
    } else {
        output_path.join(&target.relative_path)
    };
    if html && !is_output_file {
        path = path.with_extension("html");
    }
    path
}

fn ensure_unique_format_outputs(
    targets: &[CollectedSourceTarget],
    output_path: &Path,
    is_output_file: bool,
    writes_in_place: bool,
    html: bool,
) -> MResult<()> {
    let mut seen: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for target in targets {
        let output_file = format_output_file_for_target(
            target,
            output_path,
            is_output_file,
            writes_in_place,
            html,
        );
        if let Some(previous) = seen.insert(output_file.clone(), target.path.clone()) {
            return Err(MechError::new(
                GenericError {
                    msg: format!(
                        "Format output collision for `{}` between `{}` and `{}`",
                        output_file.display(),
                        previous.display(),
                        target.path.display(),
                    ),
                },
                None,
            )
            .with_compiler_loc());
        }
    }
    Ok(())
}

fn reject_multi_target_file_output(
    target_count: usize,
    output_path: &Path,
    is_output_file: bool,
) -> MResult<()> {
    if is_output_file && target_count > 1 {
        return Err(MechError::new(
      GenericError {
        msg: format!(
          "Cannot write {} formatted sources into single output file `{}`. Use an output directory instead.",
          target_count,
          output_path.display(),
        ),
      },
      None,
    ).with_compiler_loc());
    }
    Ok(())
}

pub(crate) struct FormatOptions {
    pub html: bool,
    pub stylesheet_paths: Vec<String>,
    pub shim_path: String,
    pub output_arg: Option<String>,
    pub output_path: PathBuf,
    pub mech_paths: Vec<String>,
    pub resources: WebResourceDefaults,
}

impl FormatOptions {
    pub(crate) fn from_matches(
        _root: RootFlags,
        _root_matches: &ArgMatches,
        matches: &ArgMatches,
        resources: WebResourceDefaults,
    ) -> MResult<Self> {
        let output_arg = matches.get_one::<String>("output_path").cloned();
        Ok(Self {
            html: matches.get_flag("html"),
            stylesheet_paths: matches
                .get_many::<String>("stylesheet")
                .map_or(vec![], |paths| paths.map(|path| path.to_string()).collect()),
            shim_path: matches
                .get_one::<String>("shim")
                .cloned()
                .unwrap_or("".to_string()),
            output_path: PathBuf::from(output_arg.clone().unwrap_or(".".to_string())),
            output_arg,
            mech_paths: matches
                .get_many::<String>("mech_format_file_paths")
                .map_or(vec![], |files| files.map(|file| file.to_string()).collect()),
            resources,
        })
    }
}

fn build_format_resource_authority(
    stylesheet_paths: &[String],
    shim_path: &str,
) -> MResult<HostFilesystemAuthority> {
    let mut ids = DefaultIdGenerator::new();
    let mut authority =
        HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, SharedCapabilityKernel::new());
    let mut paths = BTreeSet::<PathBuf>::new();
    for path in stylesheet_paths {
        if !path.is_empty() {
            paths.insert(PathBuf::from(path));
        }
    }
    if !shim_path.is_empty() {
        paths.insert(PathBuf::from(shim_path));
    }
    for path in paths {
        authority.grant_path(&mut ids, &path, false, [FS_READ])?;
    }
    Ok(authority)
}

pub(crate) async fn run(options: FormatOptions) -> MResult<CliOutcome> {
    let badge = "[Mech Formatter]".truecolor(34, 204, 187);
    let html_flag = options.html;
    let stylesheet_paths = options.stylesheet_paths;
    let shim_path = options.shim_path;
    let output_arg = options.output_arg;
    let output_path = options.output_path;
    let is_output_file = output_path.extension().is_some();
    let mech_paths = options.mech_paths;
    let output_matches_input_dir =
        format_output_matches_input_dir(&mech_paths, &output_path, is_output_file)?;
    reject_ambiguous_matching_output_dir(output_matches_input_dir, mech_paths.len(), &output_path)?;
    let writes_in_place =
        format_writes_in_place(output_arg.as_deref(), &output_path, is_output_file)?
            || output_matches_input_dir;

    // If the user provided exactly one path
    if mech_paths.len() == 1 {
        let input_path = PathBuf::from(&mech_paths[0]);
        if input_path.is_dir() && is_output_file {
            return Err(MechError::new(
      GenericError {
        msg: format!(
          "Cannot write directory `{}` into single output file `{}`. Provide a directory for --out instead.",
          input_path.display(),
          output_path.display(),
        ),
      },
      None,
    ).with_compiler_loc());
        }
    }
    println!("{} Loading resources…", badge);
    let resource_authority = build_format_resource_authority(&stylesheet_paths, &shim_path)?;

    // Load stylesheet
    print!("{} Loading stylesheet…", badge);
    let LoadedStylesheets {
        css: stylesheet_str,
        events,
        ..
    } = load_stylesheets(
        &resource_authority,
        &stylesheet_paths,
        &options.resources.stylesheet_backup_url,
    )
    .await?;
    render_resource_events(&badge.to_string(), "stylesheet", &events);

    // Load shim HTML
    print!("{} Loading HTML shim…", badge);
    let shim = load_resource(
        &resource_authority,
        &shim_path,
        &options.resources.shim_backup_url,
        Some(options.resources.shim_html.as_bytes()),
    )
    .await?;
    render_resource_events(&badge.to_string(), "HTML shim", &shim.events);
    let shim_source = shim.source.clone();
    let shim_str = String::from_utf8(shim.bytes).map_err(|e| {
        MechError::new(
            Utf8ConversionError {
                source_error: e.to_string(),
            },
            None,
        )
        .with_compiler_loc()
    })?;

    let output_exclusion = if writes_in_place {
        None
    } else {
        format_output_exclusion(output_arg.as_deref(), &output_path, is_output_file)?
    };
    let mut loaded_sources: Vec<(CollectedSourceTarget, MechSourceCode)> = Vec::new();
    for path in mech_paths {
        let targets = collect_format_targets(
            Path::new(&path),
            output_exclusion.as_deref(),
            html_flag,
            writes_in_place,
        )?;
        render_discovery_events("[Mech Formatter]", &targets.events);
        for target in targets {
            let code = read_format_source(&target.path)?;
            loaded_sources.push((target, code));
        }
    }
    reject_multi_target_file_output(loaded_sources.len(), &output_path, is_output_file)?;
    let format_targets: Vec<CollectedSourceTarget> = loaded_sources
        .iter()
        .map(|(target, _)| target.clone())
        .collect();
    ensure_unique_format_outputs(
        &format_targets,
        &output_path,
        is_output_file,
        writes_in_place,
        html_flag,
    )?;

    // HTML mode
    if html_flag {
        let shipped_shim = shipped_shim_name(&shim_source);
        let uses_document_controller = shim_str.contains("{{DOCUMENT_SCRIPT}}");
        let needs_bundled_wasm =
            uses_document_controller && shim_str.contains("{{WASM_MODULE_URL}}");
        let controller_outputs = loaded_sources
            .iter()
            .filter_map(|(target, source)| {
                matches!(source, MechSourceCode::String(_)).then(|| {
                    format_output_file_for_target(
                        target,
                        &output_path,
                        is_output_file,
                        writes_in_place,
                        true,
                    )
                })
            })
            .collect::<Vec<_>>();
        let runtime_assets = if needs_bundled_wasm && !controller_outputs.is_empty() {
            let package = formatter_asset_package_directory(
                &output_path,
                is_output_file,
                writes_in_place,
                &controller_outputs,
            )?;
            Some((
                package.join("mech_wasm.js"),
                package.join("mech_wasm_bg.wasm"),
            ))
        } else {
            None
        };

        let mut html_items: Vec<(PathBuf, String)> = Vec::new();
        for (target, src) in &loaded_sources {
            let output_file = format_output_file_for_target(
                target,
                &output_path,
                is_output_file,
                writes_in_place,
                true,
            );
            let html = match src {
                MechSourceCode::Html(content) => content.clone(),
                MechSourceCode::String(source) => {
                    let resolved_document = if uses_document_controller {
                        Some(resolve_document_source_bundle(&target.path)?)
                    } else {
                        None
                    };
                    let document_sources = resolved_document
                        .as_ref()
                        .map(|bundle| bundle.encoded_bundle.as_str())
                        .unwrap_or("");
                    let authoritative_source = resolved_document
                        .as_ref()
                        .map(|bundle| bundle.root_source.as_str())
                        .unwrap_or(source);
                    let wasm_module_url = runtime_assets
                        .as_ref()
                        .map(|(js, _)| relative_asset_url(&output_file, js))
                        .transpose()?
                        .unwrap_or_default();
                    let document_slots = document_controller_slots(
                        &shim_str,
                        options.resources.document_js,
                        "",
                        &wasm_module_url,
                        &document_sources,
                    )?;
                    let tree = parser::parse(authoritative_source.trim())?;
                    let mut formatter = Formatter::new();
                    let render = formatter.format_html_with_style_sheets_and_slots(
                        &tree,
                        html_style_sheets(stylesheet_str.clone()),
                        shim_str.clone(),
                        &document_slots,
                    );
                    if let Some(shim_name) = shipped_shim {
                        validate_shipped_shim_render(shim_name, &render)?;
                    }
                    render.html
                }
                other => {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "Unsupported source kind for HTML formatting `{}`: {:?}",
                                target.path.display(),
                                other
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            };
            html_items.push((output_file, html));
        }
        let mut planned_outputs = Vec::new();
        if let Some((js_path, wasm_path)) = runtime_assets {
            let wasm = options.resources.mech_wasm.ok_or_else(|| format_error(
                "selected HTML shim requests {{DOCUMENT_SCRIPT}}, but embedded mech_wasm_bg.wasm is unavailable; rebuild with bundle_web enabled",
            ))?;
            let js = options.resources.mech_js.ok_or_else(|| format_error(
                "selected HTML shim requests {{DOCUMENT_SCRIPT}}, but embedded mech_wasm.js is unavailable; rebuild with bundle_web enabled",
            ))?;
            if js.is_empty() {
                return Err(format_error("embedded mech_wasm.js is empty"));
            }
            if !wasm.starts_with(b"\0asm") {
                return Err(format_error(
                    "embedded mech_wasm_bg.wasm is not a WebAssembly binary",
                ));
            }
            planned_outputs.push(PlannedOutput {
                path: js_path,
                bytes: js.to_vec(),
            });
            planned_outputs.push(PlannedOutput {
                path: wasm_path,
                bytes: wasm.to_vec(),
            });
        }
        let html_output_paths = html_items
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        planned_outputs.extend(html_items.into_iter().map(|(path, content)| PlannedOutput {
            path,
            bytes: content.into_bytes(),
        }));
        publish_outputs_recoverably(planned_outputs)?;
        for output_file in html_output_paths {
            println!(
                "{} Saving file to {}…Done.",
                "[Save]".truecolor(153, 221, 85),
                output_file.display(),
            );
        }
    } else {
        // Raw source mode
        for (target, mech_src) in loaded_sources {
            let content = match mech_src {
                MechSourceCode::String(source) => {
                    let tree = parser::parse(source.trim())?;
                    let mut formatter = Formatter::new();
                    formatter.format(&tree)
                }
                MechSourceCode::Html(content) => content,
                other => {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "Unsupported source kind for raw formatting `{}`: {:?}",
                                target.path.display(),
                                other
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            };
            let output_file = format_output_file_for_target(
                &target,
                &output_path,
                is_output_file,
                writes_in_place,
                false,
            );
            save_to_file(output_file, &content)?;
        }
    }

    Ok(CliOutcome::success())
}

#[cfg(test)]
mod tests;
