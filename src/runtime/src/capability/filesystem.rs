use std::{path::{Component, Path, PathBuf}, sync::Arc};

use mech_core::{MResult, MechError};

use crate::*;

pub const MECH_TOOL_SUBJECT: &str = "host://mech";
pub const SERVE_HOST_SUBJECT: &str = "host://serve";
pub const FS_READ: &str = ":read";
pub const FS_LIST: &str = ":list";
pub const FS_WATCH: &str = ":watch";
pub const FS_RESOLVE: &str = ":resolve";
pub const FS_IMPORT: &str = ":import";
pub const FS_SERVE: &str = ":serve";

pub fn normalized_fs_path(path: &Path) -> MResult<String> {
  let absolute = if path.exists() {
    path.canonicalize()?
  } else {
    let path = if path.is_absolute() { path.to_path_buf() } else { std::env::current_dir()?.join(path) };
    lexical_normalize(path)
  };
  let normalized = absolute.to_string_lossy().replace('\\', "/");
  Ok(normalized.strip_prefix("//?/").unwrap_or(&normalized).to_string())
}

fn lexical_normalize(path: PathBuf) -> PathBuf {
  let mut normalized = PathBuf::new();
  for component in path.components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir => { normalized.pop(); }
      other => normalized.push(other.as_os_str()),
    }
  }
  normalized
}

pub fn fs_resource_key(path: &Path) -> MResult<String> { Ok(format!("fs://{}", normalized_fs_path(path)?)) }

pub fn fs_resource_prefix_for_dir(path: &Path) -> MResult<String> {
  Ok(format!("{}/", fs_resource_key(path)?.trim_end_matches('/')))
}

pub fn fs_request(subject: &str, operation: &str, path: &Path) -> MResult<CapabilityRequest> {
  Ok(CapabilityRequest::from_keys(subject, operation, fs_resource_key(path)?))
}

pub fn check_fs_capability(kernel: &mut dyn CapabilityKernel, subject: &str, operation: &str, path: &Path) -> MResult<CapabilityId> {
  kernel.check(&fs_request(subject, operation, path)?)
}

#[derive(Clone, Debug)]
pub struct HostFilesystemAuthority {
  pub kernel: SharedCapabilityKernel,
  pub subject: String,
  pub source_capabilities: Vec<CapabilityId>,
}

impl HostFilesystemAuthority {
  pub fn new(subject: impl Into<String>, kernel: SharedCapabilityKernel) -> Self {
    Self { kernel, subject: subject.into(), source_capabilities: Vec::new() }
  }

  pub fn grant_path(&mut self, id_generator: &mut dyn IdGenerator, path: &Path, recursive: bool, operations: impl IntoIterator<Item = &'static str>) -> MResult<CapabilityId> {
    let operations = operations.into_iter().collect::<Vec<_>>();
    let mut capability = BasicCapability::from_keys(id_generator.capability_id(), &self.subject, fs_resource_key(path)?, operations)
      .delegable(true).attenuable(true);
    if recursive { capability = capability.with_constraints(BasicConstraints::default().with_resource_prefix(fs_resource_prefix_for_dir(path)?)); }
    let id = self.kernel.grant(CapabilityGrant::new(Arc::new(capability)))?;
    self.source_capabilities.push(id);
    Ok(id)
  }

  pub fn delegate_path_to(&self, id_generator: &mut dyn IdGenerator, target_subject: &str, path: &Path, recursive: bool, operations: impl IntoIterator<Item = &'static str>) -> MResult<CapabilityId> {
    let operations = operations.into_iter().collect::<Vec<_>>();
    let resource = fs_resource_key(path)?;
    let constraints = if recursive { BasicConstraints::default().with_resource_prefix(fs_resource_prefix_for_dir(path)?) } else { BasicConstraints::default() };
    let mut last_error = None;
    for source in &self.source_capabilities {
      let mut derivation = CapabilityDerivation::attenuate(*source, id_generator.capability_id(), &BasicSubject::new(&self.subject))
        .with_subject(&BasicSubject::new(target_subject))
        .with_resource(&BasicResource::new(&resource))
        .with_operations(operations.iter().copied());
      derivation = derivation.with_constraints(constraints.clone());
      match self.kernel.clone().derive_capability(derivation) {
        Ok(id) => return Ok(id),
        Err(error) => last_error = Some(error),
      }
    }
    Err(last_error.unwrap_or_else(|| MechError::new(CapabilityDeniedError { subject: self.subject.clone(), operation: "attenuate".to_string(), resource, reason: "requested path is outside the default Mech tool grant".to_string() }, None)))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::SequentialIdGenerator;

  fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("mech-fs-capability-{}-{}", label, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap(); root.canonicalize().unwrap()
  }

  #[test]
  fn recursive_delegation_allows_nested_read() {
    let root = temp_root("nested"); let child = root.join("child"); std::fs::create_dir_all(&child).unwrap(); let file = child.join("main.mec"); std::fs::write(&file, "x := 1\n").unwrap();
    let mut ids = SequentialIdGenerator::new(); let mut authority = HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, SharedCapabilityKernel::new());
    authority.grant_path(&mut ids, &root, true, [FS_READ]).unwrap(); authority.delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &child, true, [FS_READ]).unwrap();
    check_fs_capability(&mut authority.kernel, SERVE_HOST_SUBJECT, FS_READ, &file).unwrap(); std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn delegation_outside_default_grant_fails() {
    let root = temp_root("outside"); let allowed = root.join("allowed"); let outside = root.join("outside"); std::fs::create_dir_all(&allowed).unwrap(); std::fs::create_dir_all(&outside).unwrap();
    let mut ids = SequentialIdGenerator::new(); let mut authority = HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, SharedCapabilityKernel::new()); authority.grant_path(&mut ids, &allowed, true, [FS_READ]).unwrap();
    assert!(authority.delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &outside, true, [FS_READ]).is_err()); std::fs::remove_dir_all(root).unwrap();
  }
}
