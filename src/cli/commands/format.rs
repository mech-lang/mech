use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use colored::*;
use mech_core::*;
use mech_syntax::formatter::*;
use mech_syntax::parser;
use mech_runtime::{DefaultIdGenerator, FS_READ, HostFilesystemAuthority, MECH_TOOL_SUBJECT, SharedCapabilityKernel};

use crate::cli::outcome::{CliOutcome, RootFlags};
use crate::cli::resources::{
    LoadedStylesheets, ResourceEvent, ResourceFallback,
    Utf8ConversionError, WebResourceDefaults, load_resource, load_stylesheets,
};
use crate::fs_paths::{
    absolute_path, extension_allowed, paths_equivalent, source_extension,
    unsupported_source_path_error,
};
use crate::source_discovery::{
    DedupePolicy, DiscoveryOptions, MissingPathPolicy, SkipReason, SourceDiscoveryEvent,
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


const FORMAT_EXTENSIONS: &[&str] = &["mec", "🤖", "html", "htm", "mdoc"];
const SKIP_SOURCE_DIRS: &[&str] = &["target", ".git", "dist", "out"];

#[derive(Clone, Debug)]
struct CollectedSourceTarget {
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

impl CollectedFormatTargets {
    fn extend(&mut self, other: CollectedFormatTargets) {
        self.targets.extend(other.targets);
        self.events.extend(other.events);
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
            dedupe_policy: DedupePolicy::LogicalPath,
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
    Ok(CollectedFormatTargets { targets: out, events })
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
    let mut authority = HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, SharedCapabilityKernel::new());
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
    } = load_stylesheets(&resource_authority, &stylesheet_paths, &options.resources.stylesheet_backup_url).await?;
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

    // Only create directory if output_path is not a file
    if !is_output_file && output_path != PathBuf::from(".") {
        fs::create_dir_all(&output_path)?;
        println!(
            "{} Directory created: {}",
            "[Created]".truecolor(153, 221, 85),
            output_path.display()
        );
    }

    // HTML mode
    if html_flag {
        let mut html_items: Vec<(CollectedSourceTarget, String)> = Vec::new();
        for (target, src) in &loaded_sources {
            let html = match src {
                MechSourceCode::Html(content) => content.clone(),
                MechSourceCode::String(source) => {
                    let tree = parser::parse(source.trim())?;
                    let mut formatter = Formatter::new();
                    formatter.format_html(&tree, stylesheet_str.clone(), shim_str.clone())
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
            html_items.push((target.clone(), html));
        }
        if is_output_file && html_items.len() == 1 {
            let (_, content) = html_items.remove(0);
            save_to_file(output_path, &content)?;
        } else {
            for (target, content) in html_items {
                let output_file = format_output_file_for_target(
                    &target,
                    &output_path,
                    is_output_file,
                    writes_in_place,
                    true,
                );
                save_to_file(output_file, &content)?;
            }
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
