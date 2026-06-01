use ignore::WalkBuilder;

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RuntimeWorkspaceDiscoveredFile {
  pub specifier: String,
  pub canonical_path: PathBuf,
}

pub(super) fn discover_workspace_files(
  root: &Path,
  folders: &[RuntimeWorkspaceFolder],
  runtime: &mut MechRuntime,
  capability_subject: Option<&str>,
) -> MResult<(Vec<RuntimeWorkspaceDiscoveredFile>, Vec<RuntimeWorkspaceDiagnostic>)> {
  let mut diagnostics = Vec::new();
  let mut discovered = BTreeMap::new();

  for folder in folders {
    let folder_path = workspace_folder_path(root, folder)?;
    if let Some(subject) = capability_subject {
      if let Err(error) = crate::check_fs_capability(runtime.capability_kernel_mut(), subject, crate::FS_LIST, &folder_path) {
        diagnostics.push(capability_diagnostic(&folder.specifier, error));
        continue;
      }
    }

    let mut builder = WalkBuilder::new(&folder_path);
    builder
      .hidden(false)
      .git_ignore(true)
      .git_exclude(true)
      .parents(true);

    if !folder.recursive {
      builder.max_depth(Some(1));
    }

    for result in builder.build() {
      let entry = result.map_err(|error| {
        MechError::new(
          RuntimeWorkspaceInvalidConfig {
            reason: format!(
              "workspace folder `{}` could not be read: {}",
              folder.specifier,
              error,
            ),
          },
          None,
        )
      })?;

      let path = entry.path();

      if !path.is_file() {
        continue;
      }

      if !matches!(path.extension().and_then(|ext| ext.to_str()), Some("mec") | Some("🤖")) {
        continue;
      }

      let canonical_path = path.canonicalize().map_err(|error| {
        MechError::new(
          RuntimeWorkspaceInvalidConfig {
            reason: format!(
              "workspace file `{}` could not be canonicalized: {}",
              path.display(),
              error,
            ),
          },
          None,
        )
      })?;

      if let Some(subject) = capability_subject {
        if let Err(error) = crate::check_fs_capability(runtime.capability_kernel_mut(), subject, crate::FS_READ, &canonical_path) {
          diagnostics.push(capability_diagnostic(&canonical_path.display().to_string(), error));
          continue;
        }
      }

      if !canonical_path.starts_with(root) {
        return Err(MechError::new(
          RuntimeWorkspaceInvalidConfig {
            reason: format!(
              "workspace file `{}` resolves outside workspace root `{}`",
              canonical_path.display(),
              root.display(),
            ),
          },
          None,
        ));
      }

      discovered.insert(canonical_path.clone(), RuntimeWorkspaceDiscoveredFile {
        specifier: canonical_path.to_string_lossy().to_string(),
        canonical_path,
      });
    }
  }

  Ok((discovered.into_values().collect(), diagnostics))
}

fn capability_diagnostic(target: &str, error: MechError) -> RuntimeWorkspaceDiagnostic {
  RuntimeWorkspaceDiagnostic { severity: RuntimeWorkspaceDiagnosticSeverity::Error, target: Some(target.to_string()), canonical_uri: None, message: format!("{:?}", error) }
}

fn workspace_folder_path(
  root: &Path,
  folder: &RuntimeWorkspaceFolder,
) -> MResult<PathBuf> {
  if folder.specifier.contains("://") {
    return Err(MechError::new(
      RuntimeWorkspaceInvalidConfig {
        reason: format!(
          "workspace folder `{}` must be a local path",
          folder.specifier,
        ),
      },
      None,
    ));
  }

  let path = PathBuf::from(&folder.specifier);
  let joined = if path.is_absolute() {
    path
  } else {
    root.join(path)
  };

  let canonical = joined.canonicalize().map_err(|error| {
    MechError::new(
      RuntimeWorkspaceInvalidConfig {
        reason: format!(
          "workspace folder `{}` could not be canonicalized against root `{}`: {}",
          folder.specifier,
          root.display(),
          error,
        ),
      },
      None,
    )
  })?;

  if !canonical.is_dir() {
    return Err(MechError::new(
      RuntimeWorkspaceInvalidConfig {
        reason: format!(
          "workspace folder `{}` is not a directory",
          canonical.display(),
        ),
      },
      None,
    ));
  }

  if !canonical.starts_with(root) {
    return Err(MechError::new(
      RuntimeWorkspaceInvalidConfig {
        reason: format!(
          "workspace folder `{}` resolves outside workspace root `{}`",
          folder.specifier,
          root.display(),
        ),
      },
      None,
    ));
  }

  Ok(canonical)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::RuntimeBuilder;

  fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-runtime-workspace-discovery-{}-{}",
      label,
      SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root.canonicalize().unwrap()
  }

  #[test]
  fn discovery_denial_returns_diagnostic_without_files() {
    let root = temp_root("denied");
    std::fs::write(root.join("secret.mec"), "x := 1\n").unwrap();
    let mut runtime = RuntimeBuilder::new().capability_kernel(crate::SharedCapabilityKernel::new()).build().unwrap();
    let (files, diagnostics) = discover_workspace_files(&root, &[RuntimeWorkspaceFolder { specifier: ".".to_string(), recursive: true }], &mut runtime, Some(crate::SERVE_HOST_SUBJECT)).unwrap();
    assert!(files.is_empty());
    assert!(diagnostics.iter().any(|diagnostic| diagnostic.message.contains("CapabilityDenied")));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn discover_workspace_files_finds_recursive_mec_files() {
    let root = temp_root("recursive");
    std::fs::create_dir_all(root.join("src/nested")).unwrap();
    std::fs::write(root.join("src/main.mec"), "result := true\n").unwrap();
    std::fs::write(root.join("src/nested/lib.mec"), "value := true\n").unwrap();
    std::fs::write(root.join("src/readme.txt"), "ignore me\n").unwrap();

    let files = discover_workspace_files(
      &root,
      &[RuntimeWorkspaceFolder {
        specifier: "src".to_string(),
        recursive: true,
      }],
      &mut RuntimeBuilder::new().build().unwrap(),
      None,
    ).unwrap().0;

    let paths = files
      .iter()
      .map(|file| file.canonical_path.strip_prefix(&root).unwrap().to_path_buf())
      .collect::<BTreeSet<_>>();

    assert!(paths.contains(&PathBuf::from("src/main.mec")));
    assert!(paths.contains(&PathBuf::from("src/nested/lib.mec")));
    assert!(!paths.contains(&PathBuf::from("src/readme.txt")));
  }

  #[test]
  fn discover_workspace_files_honors_non_recursive_folder() {
    let root = temp_root("non-recursive");
    std::fs::create_dir_all(root.join("src/nested")).unwrap();
    std::fs::write(root.join("src/main.mec"), "result := true\n").unwrap();
    std::fs::write(root.join("src/nested/lib.mec"), "value := true\n").unwrap();

    let files = discover_workspace_files(
      &root,
      &[RuntimeWorkspaceFolder {
        specifier: "src".to_string(),
        recursive: false,
      }],
      &mut RuntimeBuilder::new().build().unwrap(),
      None,
    ).unwrap().0;

    let paths = files
      .iter()
      .map(|file| file.canonical_path.strip_prefix(&root).unwrap().to_path_buf())
      .collect::<BTreeSet<_>>();

    assert!(paths.contains(&PathBuf::from("src/main.mec")));
    assert!(!paths.contains(&PathBuf::from("src/nested/lib.mec")));
  }

  #[test]
  fn discover_workspace_files_rejects_outside_folder() {
    let parent = temp_root("outside-parent");
    let root = parent.join("workspace");
    let outside = parent.join("outside");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    let error = format!(
      "{:?}",
      discover_workspace_files(
        &root.canonicalize().unwrap(),
        &[RuntimeWorkspaceFolder {
          specifier: "../outside".to_string(),
          recursive: true,
        }],
        &mut RuntimeBuilder::new().build().unwrap(),
        None,
      ).err().unwrap()
    );

    assert!(error.contains("RuntimeWorkspaceInvalidConfig"));
    assert!(error.contains("outside workspace root"));
  }
}