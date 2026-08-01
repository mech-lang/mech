use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use mech_core::MResult;

use super::{absolute_path, format_error};

pub(super) struct PlannedOutput {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

struct PreparedOutput {
    destination: PathBuf,
    staging: PathBuf,
    planned_backup: Option<PathBuf>,
    backup: Option<PathBuf>,
    installed: bool,
}

static NEXT_PUBLICATION_ARTIFACT: AtomicU64 = AtomicU64::new(0);

fn normalized_destination(path: &Path) -> MResult<PathBuf> {
    let absolute = absolute_path(path)?;
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

#[cfg(not(windows))]
fn normalized_physical_key(path: PathBuf) -> PathBuf {
    path
}

#[cfg(windows)]
fn normalized_physical_key(path: PathBuf) -> PathBuf {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    // Fold ordinary ASCII Windows output names while preserving every UTF-16
    // code unit. Do not use to_string_lossy: unpaired surrogates must not
    // collapse to replacement characters in the duplicate-detection key.
    let folded = path
        .as_os_str()
        .encode_wide()
        .map(|unit| {
            if unit >= u16::from(b'A') && unit <= u16::from(b'Z') {
                unit + u16::from(b'a' - b'A')
            } else {
                unit
            }
        })
        .collect::<Vec<_>>();
    PathBuf::from(OsString::from_wide(&folded))
}

fn physical_destination_identity(path: &Path) -> MResult<PathBuf> {
    let mut existing_ancestor = path.to_path_buf();
    let mut unresolved_components = Vec::<OsString>::new();

    loop {
        match fs::canonicalize(&existing_ancestor) {
            Ok(mut physical) => {
                for component in unresolved_components.iter().rev() {
                    physical.push(component);
                }
                return Ok(normalized_physical_key(physical));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let component = existing_ancestor.file_name().ok_or_else(|| {
                    format_error(format!(
                        "formatter output `{}` has no existing ancestor",
                        path.display(),
                    ))
                })?;
                unresolved_components.push(component.to_os_string());
                existing_ancestor = existing_ancestor
                    .parent()
                    .ok_or_else(|| {
                        format_error(format!(
                            "formatter output `{}` has no existing ancestor",
                            path.display(),
                        ))
                    })?
                    .to_path_buf();
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn ensure_unique_physical_destinations<'a>(
    destinations: impl IntoIterator<Item = &'a PathBuf>,
) -> MResult<()> {
    let mut physical_destinations = BTreeSet::new();
    for destination in destinations {
        let physical = physical_destination_identity(destination)?;
        if !physical_destinations.insert(physical.clone()) {
            return Err(format_error(format!(
                "formatter publication contains duplicate physical destination `{}`",
                physical.display(),
            )));
        }
    }
    Ok(())
}

fn publication_artifact_path(path: &Path, suffix: &str) -> MResult<PathBuf> {
    let parent = path.parent().ok_or_else(|| {
        format_error(format!(
            "formatter output `{}` has no parent directory",
            path.display(),
        ))
    })?;
    let file_name = path.file_name().ok_or_else(|| {
        format_error(format!(
            "formatter output `{}` has no file name",
            path.display(),
        ))
    })?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            format_error(format!(
                "system clock error while publishing formatter outputs: {error}",
            ))
        })?
        .as_nanos();
    let sequence = NEXT_PUBLICATION_ARTIFACT.fetch_add(1, Ordering::Relaxed);
    let mut artifact_name = OsString::from(".");
    artifact_name.push(file_name);
    artifact_name.push(format!(
        ".{}.{}.{sequence}.{suffix}",
        std::process::id(),
        stamp,
    ));
    Ok(parent.join(artifact_name))
}

fn unused_publication_artifact_path(path: &Path, suffix: &str) -> MResult<PathBuf> {
    loop {
        let candidate = publication_artifact_path(path, suffix)?;
        match fs::symlink_metadata(&candidate) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(candidate);
            }
            Ok(_) => continue,
            Err(error) => return Err(error.into()),
        }
    }
}

fn destination_exists(path: &Path) -> MResult<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(format_error(format!(
            "refusing to replace formatter output `{}` because the destination is a symlink",
            path.display(),
        ))),
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(format_error(format!(
            "refusing to replace formatter output `{}` because the destination is not a regular file",
            path.display(),
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn missing_parent_directories(path: &Path) -> MResult<Vec<PathBuf>> {
    let mut missing = Vec::new();
    let mut current = path
        .parent()
        .ok_or_else(|| {
            format_error(format!(
                "formatter output `{}` has no parent directory",
                path.display(),
            ))
        })?
        .to_path_buf();
    loop {
        match fs::metadata(&current) {
            Ok(metadata) if metadata.is_dir() => break,
            Ok(_) => {
                return Err(format_error(format!(
                    "formatter output parent `{}` is not a directory",
                    current.display(),
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(current.clone());
                current = current
                    .parent()
                    .ok_or_else(|| {
                        format_error(format!(
                            "formatter output parent `{}` has no existing ancestor",
                            path.display(),
                        ))
                    })?
                    .to_path_buf();
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(missing)
}

fn remove_file(operation: &'static str, path: &Path) -> Result<(), std::io::Error> {
    maybe_fail_publication_operation(operation, path)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn rename(operation: &'static str, from: &Path, to: &Path) -> Result<(), std::io::Error> {
    let fault_path = if operation == "backup" { from } else { to };
    maybe_fail_publication_operation(operation, fault_path)?;
    fs::rename(from, to)
}

fn remove_created_directories(created_directories: &[PathBuf], failures: &mut Vec<String>) {
    for directory in created_directories.iter().rev() {
        if let Err(error) = fs::remove_dir(directory) {
            if error.kind() != std::io::ErrorKind::NotFound {
                failures.push(format!(
                    "failed to remove created directory `{}`: {error}",
                    directory.display(),
                ));
            }
        }
    }
}

fn remove_staging_files(outputs: &[PreparedOutput], failures: &mut Vec<String>) {
    for output in outputs {
        if let Err(error) = remove_file("remove-staging", &output.staging) {
            failures.push(format!(
                "failed to remove staging file `{}`: {error}",
                output.staging.display(),
            ));
        }
    }
}

fn restore_backups(outputs: &[PreparedOutput], failures: &mut Vec<String>) {
    for output in outputs.iter().rev() {
        let Some(backup) = output.backup.as_ref() else {
            continue;
        };
        if let Err(error) = rename("restore-backup", backup, &output.destination) {
            failures.push(format!(
                "failed to restore formatter output `{}` from `{}`: {error}",
                output.destination.display(),
                backup.display(),
            ));
        }
    }
}

fn publication_failure(
    action: impl Into<String>,
    rollback_failures: Vec<String>,
) -> mech_core::MechError {
    let action = action.into();
    if rollback_failures.is_empty() {
        format_error(action)
    } else {
        format_error(format!(
            "{action}; rollback failures: {}",
            rollback_failures.join("; "),
        ))
    }
}

pub(super) fn publish_outputs_recoverably(outputs: Vec<PlannedOutput>) -> MResult<()> {
    let mut destinations = BTreeSet::new();
    let mut required_directories = BTreeSet::new();
    let mut prepared = Vec::with_capacity(outputs.len());

    for output in outputs {
        let destination = normalized_destination(&output.path)?;
        if !destinations.insert(destination.clone()) {
            return Err(format_error(format!(
                "formatter publication contains duplicate destination `{}`",
                destination.display(),
            )));
        }
        let existed = destination_exists(&destination)?;
        required_directories.extend(missing_parent_directories(&destination)?);
        let staging = unused_publication_artifact_path(&destination, "stage")?;
        let backup = if existed {
            Some(unused_publication_artifact_path(&destination, "backup")?)
        } else {
            None
        };
        prepared.push((destination, output.bytes, staging, backup));
    }
    ensure_unique_physical_destinations(prepared.iter().map(|(destination, _, _, _)| destination))?;

    let mut required_directories = required_directories.into_iter().collect::<Vec<_>>();
    required_directories.sort_by_key(|path| path.components().count());
    let mut created_directories = Vec::new();
    for directory in required_directories {
        if let Err(error) = maybe_fail_publication_operation("create-directory", &directory)
            .and_then(|()| fs::create_dir(&directory))
        {
            let mut rollback_failures = Vec::new();
            remove_created_directories(&created_directories, &mut rollback_failures);
            return Err(publication_failure(
                format!(
                    "failed to create formatter output directory `{}`: {error}",
                    directory.display(),
                ),
                rollback_failures,
            ));
        }
        created_directories.push(directory);
    }

    if let Err(error) = ensure_unique_physical_destinations(
        prepared.iter().map(|(destination, _, _, _)| destination),
    ) {
        let mut rollback_failures = Vec::new();
        remove_created_directories(&created_directories, &mut rollback_failures);
        return Err(publication_failure(
            format!("failed to validate formatter output destinations: {error:?}"),
            rollback_failures,
        ));
    }

    let mut outputs = Vec::with_capacity(prepared.len());
    for (destination, bytes, staging, planned_backup) in prepared {
        let stage_result = (|| -> Result<(), std::io::Error> {
            maybe_fail_publication_operation("stage", &destination)?;
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&staging)?;
            file.write_all(&bytes)?;
            file.flush()?;
            file.sync_all()?;
            Ok(())
        })();
        if let Err(error) = stage_result {
            let mut rollback_failures = Vec::new();
            if let Err(remove_error) = remove_file("remove-staging", &staging) {
                rollback_failures.push(format!(
                    "failed to remove staging file `{}`: {remove_error}",
                    staging.display(),
                ));
            }
            remove_staging_files(&outputs, &mut rollback_failures);
            remove_created_directories(&created_directories, &mut rollback_failures);
            return Err(publication_failure(
                format!(
                    "failed to stage formatter output `{}`: {error}",
                    destination.display(),
                ),
                rollback_failures,
            ));
        }
        outputs.push(PreparedOutput {
            destination,
            staging,
            planned_backup,
            backup: None,
            installed: false,
        });
    }

    for index in 0..outputs.len() {
        let Some(backup) = outputs[index].planned_backup.clone() else {
            continue;
        };
        if let Err(error) = rename("backup", &outputs[index].destination, &backup) {
            let mut rollback_failures = Vec::new();
            restore_backups(&outputs[..index], &mut rollback_failures);
            remove_staging_files(&outputs, &mut rollback_failures);
            remove_created_directories(&created_directories, &mut rollback_failures);
            return Err(publication_failure(
                format!(
                    "failed to back up formatter output `{}`: {error}",
                    outputs[index].destination.display(),
                ),
                rollback_failures,
            ));
        }
        outputs[index].backup = Some(backup);
    }

    for index in 0..outputs.len() {
        if let Err(error) = rename(
            "install",
            &outputs[index].staging,
            &outputs[index].destination,
        ) {
            let mut rollback_failures = Vec::new();
            for installed in outputs[..index].iter().rev() {
                if installed.installed {
                    if let Err(remove_error) =
                        remove_file("remove-installed", &installed.destination)
                    {
                        rollback_failures.push(format!(
                            "failed to remove newly installed formatter output `{}`: {remove_error}",
                            installed.destination.display(),
                        ));
                    }
                }
            }
            restore_backups(&outputs, &mut rollback_failures);
            remove_staging_files(&outputs, &mut rollback_failures);
            remove_created_directories(&created_directories, &mut rollback_failures);
            return Err(publication_failure(
                format!(
                    "failed to install formatter output `{}`: {error}",
                    outputs[index].destination.display(),
                ),
                rollback_failures,
            ));
        }
        outputs[index].installed = true;
    }

    let mut cleanup_failures = Vec::new();
    for output in &outputs {
        let Some(backup) = output.backup.as_ref() else {
            continue;
        };
        if let Err(error) = remove_file("cleanup-backup", backup) {
            cleanup_failures.push(format!(
                "failed to remove formatter output backup `{}`: {error}",
                backup.display(),
            ));
        }
    }
    if cleanup_failures.is_empty() {
        Ok(())
    } else {
        Err(format_error(format!(
            "formatter outputs were installed but backup cleanup failed: {}",
            cleanup_failures.join("; "),
        )))
    }
}

#[cfg(not(test))]
fn maybe_fail_publication_operation(
    _operation: &'static str,
    _path: &Path,
) -> Result<(), std::io::Error> {
    Ok(())
}

#[cfg(test)]
#[derive(Clone, Debug)]
struct PublicationFault {
    operation: &'static str,
    file_name: String,
}

#[cfg(test)]
thread_local! {
    static PUBLICATION_FAULTS: std::cell::RefCell<Vec<PublicationFault>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

#[cfg(test)]
fn maybe_fail_publication_operation(
    operation: &'static str,
    path: &Path,
) -> Result<(), std::io::Error> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    PUBLICATION_FAULTS.with(|faults| {
        let mut faults = faults.borrow_mut();
        let Some(index) = faults
            .iter()
            .position(|fault| fault.operation == operation && fault.file_name == file_name)
        else {
            return Ok(());
        };
        faults.remove(index);
        Err(std::io::Error::other(format!(
            "injected formatter publication {operation} failure for {file_name}",
        )))
    })
}

#[cfg(test)]
struct PublicationFaultGuard;

#[cfg(test)]
impl Drop for PublicationFaultGuard {
    fn drop(&mut self) {
        PUBLICATION_FAULTS.with(|faults| faults.borrow_mut().clear());
    }
}

#[cfg(test)]
fn inject_publication_faults(faults: &[(&'static str, &str)]) -> PublicationFaultGuard {
    PUBLICATION_FAULTS.with(|state| {
        let mut state = state.borrow_mut();
        assert!(state.is_empty(), "publication faults were already armed");
        state.extend(
            faults
                .iter()
                .map(|(operation, file_name)| PublicationFault {
                    operation,
                    file_name: (*file_name).to_string(),
                }),
        );
    });
    PublicationFaultGuard
}

#[cfg(test)]
#[path = "publication/tests.rs"]
mod tests;
