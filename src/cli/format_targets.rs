use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};

use mech_core::*;

use crate::cli::format_args::FormatOptions;
use crate::fs_paths::{
    absolute_path, extension_allowed, paths_equivalent, source_extension,
    unsupported_source_path_error,
};
use crate::source_discovery::{
    DedupePolicy, DiscoveryOptions, MissingPathPolicy, SourceDiscoveryEvent,
    collect_sources_with_events,
};

pub(crate) const FORMAT_EXTENSIONS: &[&str] = &["mec", "🤖", "html", "htm", "mdoc"];
const SKIP_SOURCE_DIRS: &[&str] = &["target", ".git", "dist", "out"];

#[derive(Clone, Debug)]
pub(crate) struct CollectedSourceTarget {
    pub(crate) input_root: PathBuf,
    pub(crate) path: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) default_output_path: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CollectedFormatTargets {
    pub(crate) targets: Vec<CollectedSourceTarget>,
    pub(crate) events: Vec<SourceDiscoveryEvent>,
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

pub(crate) fn normalize_output_exclusion(
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

pub(crate) fn format_output_exclusion(
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

pub(crate) fn format_writes_in_place(
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

pub(crate) fn safe_output_relative_path(path: &Path) -> MResult<PathBuf> {
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

pub(crate) fn default_output_relative_path(input_root: &Path, path: &Path) -> MResult<PathBuf> {
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

fn skip_directory_format_source(path: &Path, html: bool, writes_in_place: bool) -> bool {
    html && writes_in_place
        && matches!(
            source_extension(path).as_deref(),
            Some("html") | Some("htm")
        )
}

pub(crate) fn collect_format_targets(
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
    Ok(CollectedFormatTargets {
        targets: out,
        events,
    })
}
pub(crate) fn format_output_matches_input_dir(
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

pub(crate) fn reject_ambiguous_matching_output_dir(
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

pub(crate) fn format_output_file_for_target(
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

pub(crate) fn ensure_unique_format_outputs(
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

pub(crate) fn reject_multi_target_file_output(
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

pub(crate) struct FormatTargetPlan {
    pub(crate) targets: Vec<CollectedSourceTarget>,
    pub(crate) discovery_events: Vec<SourceDiscoveryEvent>,
    pub(crate) output_path: PathBuf,
    pub(crate) is_output_file: bool,
    pub(crate) writes_in_place: bool,
    pub(crate) html: bool,
}

pub(crate) fn plan_format_targets(options: &FormatOptions) -> MResult<FormatTargetPlan> {
    let output_path = options.output_path.clone();
    let is_output_file = output_path.extension().is_some();
    let output_matches_input_dir =
        format_output_matches_input_dir(&options.mech_paths, &output_path, is_output_file)?;
    reject_ambiguous_matching_output_dir(
        output_matches_input_dir,
        options.mech_paths.len(),
        &output_path,
    )?;
    let writes_in_place =
        format_writes_in_place(options.output_arg.as_deref(), &output_path, is_output_file)?
            || output_matches_input_dir;

    if options.mech_paths.len() == 1 {
        let input_path = PathBuf::from(&options.mech_paths[0]);
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

    let output_exclusion = if writes_in_place {
        None
    } else {
        format_output_exclusion(options.output_arg.as_deref(), &output_path, is_output_file)?
    };
    let mut all_targets = Vec::new();
    let mut discovery_events = Vec::new();
    for path in &options.mech_paths {
        let targets = collect_format_targets(
            Path::new(path),
            output_exclusion.as_deref(),
            options.html,
            writes_in_place,
        )?;
        discovery_events.extend(targets.events);
        all_targets.extend(targets.targets);
    }

    reject_multi_target_file_output(all_targets.len(), &output_path, is_output_file)?;
    ensure_unique_format_outputs(
        &all_targets,
        &output_path,
        is_output_file,
        writes_in_place,
        options.html,
    )?;

    Ok(FormatTargetPlan {
        targets: all_targets,
        discovery_events,
        output_path,
        is_output_file,
        writes_in_place,
        html: options.html,
    })
}
