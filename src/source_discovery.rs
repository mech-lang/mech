use crate::fs_paths::{
    canonicalize_for_read, extension_allowed, is_directory_symlink, relative_to_base,
    unsupported_source_path_error,
};
use mech_core::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct SourceEntry {
    pub logical_path: PathBuf,
    pub relative_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceDiscoveryResult {
    pub entries: Vec<SourceEntry>,
    pub events: Vec<SourceDiscoveryEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SourceDiscoveryEvent {
    SkippedBrokenSymlink { path: PathBuf },
    SkippedSymlinkedDirectory { path: PathBuf },
    SkippedFileSymlink { path: PathBuf },
    SkippedUnsupportedExtension { path: PathBuf },
    SkippedDirectory { path: PathBuf, reason: SkipReason },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SkipReason {
    SkippedByName,
    AlreadyVisited,
}

#[derive(Clone, Debug)]
pub(crate) struct DiscoveryOptions {
    pub allowed_file_extensions: &'static [&'static str],
    pub recursive_file_extensions: &'static [&'static str],
    pub skip_dir_names: &'static [&'static str],
    pub follow_file_symlinks: bool,
    pub follow_dir_symlinks: bool,
    pub missing_path_policy: MissingPathPolicy,
    pub dedupe_policy: DedupePolicy,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum MissingPathPolicy {
    SkipBrokenSymlink,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum DedupePolicy {
    LogicalPath,
    CanonicalPath,
}

pub(crate) fn collect_sources_with_events(
    roots: &[PathBuf],
    base_dir: &Path,
    options: DiscoveryOptions,
) -> MResult<SourceDiscoveryResult> {
    let project_dir = base_dir;
    let mut entries = Vec::new();
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    let mut visited_dirs = BTreeSet::new();
    for root in roots {
        collect_one(
            root,
            root,
            base_dir,
            project_dir,
            &options,
            &mut entries,
            &mut events,
            &mut seen,
            &mut visited_dirs,
            true,
        )?;
    }
    Ok(SourceDiscoveryResult { entries, events })
}

fn collect_one(
    logical_path: &Path,
    read_path: &Path,
    base_dir: &Path,
    project_dir: &Path,
    options: &DiscoveryOptions,
    entries: &mut Vec<SourceEntry>,
    events: &mut Vec<SourceDiscoveryEvent>,
    seen: &mut BTreeSet<PathBuf>,
    visited_dirs: &mut BTreeSet<PathBuf>,
    explicit: bool,
) -> MResult<()> {
    let metadata = match fs::symlink_metadata(read_path) {
        Ok(metadata) => metadata,
        Err(error)
            if matches!(
                options.missing_path_policy,
                MissingPathPolicy::SkipBrokenSymlink
            ) =>
        {
            if explicit {
                return Err(error.into());
            }
            events.push(SourceDiscoveryEvent::SkippedBrokenSymlink {
                path: read_path.to_path_buf(),
            });
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        let canonical = match canonicalize_for_read(read_path) {
            Ok(path) => path,
            Err(error)
                if matches!(
                    options.missing_path_policy,
                    MissingPathPolicy::SkipBrokenSymlink
                ) =>
            {
                if explicit {
                    return Err(error);
                }
                events.push(SourceDiscoveryEvent::SkippedBrokenSymlink {
                    path: read_path.to_path_buf(),
                });
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        if canonical.is_dir() {
            if !options.follow_dir_symlinks {
                events.push(SourceDiscoveryEvent::SkippedSymlinkedDirectory {
                    path: logical_path.to_path_buf(),
                });
                return Ok(());
            }
            return collect_dir(
                logical_path,
                &canonical,
                base_dir,
                project_dir,
                options,
                entries,
                events,
                seen,
                visited_dirs,
            );
        }
        if !options.follow_file_symlinks {
            events.push(SourceDiscoveryEvent::SkippedFileSymlink {
                path: logical_path.to_path_buf(),
            });
            return Ok(());
        }
        return collect_file(
            logical_path,
            &canonical,
            &canonical,
            base_dir,
            project_dir,
            options,
            entries,
            events,
            seen,
            explicit,
        );
    }
    if metadata.is_dir() {
        return collect_dir(
            logical_path,
            read_path,
            base_dir,
            project_dir,
            options,
            entries,
            events,
            seen,
            visited_dirs,
        );
    }
    let canonical = canonicalize_for_read(read_path)?;
    collect_file(
        logical_path,
        read_path,
        &canonical,
        base_dir,
        project_dir,
        options,
        entries,
        events,
        seen,
        explicit,
    )
}

fn collect_dir(
    logical_dir: &Path,
    read_dir: &Path,
    base_dir: &Path,
    project_dir: &Path,
    options: &DiscoveryOptions,
    entries: &mut Vec<SourceEntry>,
    events: &mut Vec<SourceDiscoveryEvent>,
    seen: &mut BTreeSet<PathBuf>,
    visited_dirs: &mut BTreeSet<PathBuf>,
) -> MResult<()> {
    let canonical_dir = canonicalize_for_read(read_dir)?;
    if !visited_dirs.insert(canonical_dir.clone()) {
        events.push(SourceDiscoveryEvent::SkippedDirectory {
            path: logical_dir.to_path_buf(),
            reason: SkipReason::AlreadyVisited,
        });
        return Ok(());
    }
    for entry in fs::read_dir(read_dir)? {
        let entry = entry?;
        let logical_path = logical_dir.join(entry.file_name());
        let read_path = entry.path();
        let is_directory_symlink = match is_directory_symlink(&read_path) {
            Ok(value) => value,
            Err(_error) if matches!(options.missing_path_policy, MissingPathPolicy::SkipBrokenSymlink) => {
                events.push(SourceDiscoveryEvent::SkippedBrokenSymlink { path: read_path.clone() });
                continue;
            }
            Err(error) => return Err(error),
        };
        if is_directory_symlink && !options.follow_dir_symlinks {
            events.push(SourceDiscoveryEvent::SkippedSymlinkedDirectory {
                path: logical_path.clone(),
            });
            continue;
        }
        let is_dir = match read_path.canonicalize() {
            Ok(path) => path.is_dir(),
            Err(_error) if matches!(options.missing_path_policy, MissingPathPolicy::SkipBrokenSymlink) => {
                events.push(SourceDiscoveryEvent::SkippedBrokenSymlink { path: read_path.clone() });
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        if read_path.is_dir() || is_dir {
            if options
                .skip_dir_names
                .iter()
                .any(|name| read_path.file_name().and_then(|n| n.to_str()) == Some(*name))
            {
                events.push(SourceDiscoveryEvent::SkippedDirectory {
                    path: logical_path.clone(),
                    reason: SkipReason::SkippedByName,
                });
                continue;
            }
            collect_one(
                &logical_path,
                &read_path,
                base_dir,
                project_dir,
                options,
                entries,
                events,
                seen,
                visited_dirs,
                false,
            )?;
        } else if extension_allowed(&read_path, options.recursive_file_extensions) {
            collect_one(
                &logical_path,
                &read_path,
                base_dir,
                project_dir,
                options,
                entries,
                events,
                seen,
                visited_dirs,
                false,
            )?;
        }
    }
    Ok(())
}

fn collect_file(
    logical_path: &Path,
    _read_path: &Path,
    canonical_path: &Path,
    base_dir: &Path,
    project_dir: &Path,
    options: &DiscoveryOptions,
    entries: &mut Vec<SourceEntry>,
    events: &mut Vec<SourceDiscoveryEvent>,
    seen: &mut BTreeSet<PathBuf>,
    explicit: bool,
) -> MResult<()> {
    if !extension_allowed(
        logical_path,
        if explicit {
            options.allowed_file_extensions
        } else {
            options.recursive_file_extensions
        },
    ) {
        if explicit {
            return Err(unsupported_source_path_error(
                logical_path,
                options.allowed_file_extensions,
            ));
        }
        events.push(SourceDiscoveryEvent::SkippedUnsupportedExtension {
            path: logical_path.to_path_buf(),
        });
        return Ok(());
    }
    let relative_path = relative_to_base(logical_path, base_dir, project_dir)?;
    let key = match options.dedupe_policy {
        DedupePolicy::LogicalPath => logical_path.to_path_buf(),
        DedupePolicy::CanonicalPath => canonical_path.to_path_buf(),
    };
    if !seen.insert(key) {
        return Ok(());
    }
    entries.push(SourceEntry {
        logical_path: logical_path.to_path_buf(),
        relative_path,
    });
    Ok(())
}
