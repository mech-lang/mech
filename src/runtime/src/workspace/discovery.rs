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
) -> MResult<Vec<RuntimeWorkspaceDiscoveredFile>> {
  let mut discovered = BTreeMap::new();

  for folder in folders {
    let folder_path = workspace_folder_path(root, folder)?;

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

      if path.extension().and_then(|ext| ext.to_str()) != Some("mec") {
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

  Ok(discovered.into_values().collect())
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
    ).unwrap();

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
    ).unwrap();

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
      ).err().unwrap()
    );

    assert!(error.contains("RuntimeWorkspaceInvalidConfig"));
    assert!(error.contains("outside workspace root"));
  }
}