//! Selected compiler profiles owned by the compact coordinator.
use crate::native_artifact::identity::{hex, transcript::bytes};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    io,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
pub(in crate::native_artifact) struct SelectedProfiles(Rc<RefCell<ProfileCache>>);

#[derive(Clone)]
pub(in crate::native_artifact::program) struct ResolvedProfile(Arc<ProfileRead>);

enum ProfileRead {
    Bytes(Vec<u8>),
    Error {
        kind: io::ErrorKind,
        os_code: Option<i32>,
        message: String,
    },
}

#[cfg(test)]
type TestReader = Box<dyn FnMut(&Path) -> io::Result<Vec<u8>>>;

#[derive(Default)]
struct ProfileCache {
    reads: BTreeMap<PathBuf, ResolvedProfile>,
    #[cfg(test)]
    reader: Option<TestReader>,
}

impl ProfileCache {
    fn read(&mut self, path: &Path) -> io::Result<Vec<u8>> {
        #[cfg(test)]
        if let Some(reader) = &mut self.reader {
            return reader(path);
        }
        std::fs::read(path)
    }
}

impl SelectedProfiles {
    pub(in crate::native_artifact) fn new() -> Self {
        Self(Rc::new(RefCell::new(ProfileCache::default())))
    }

    #[cfg(test)]
    pub(in crate::native_artifact::program) fn with_reader(
        reader: impl FnMut(&Path) -> io::Result<Vec<u8>> + 'static,
    ) -> Self {
        Self(Rc::new(RefCell::new(ProfileCache {
            reads: BTreeMap::new(),
            reader: Some(Box::new(reader)),
        })))
    }

    pub(in crate::native_artifact::program) fn resolve_selected(
        &self,
        path: &Path,
    ) -> ResolvedProfile {
        let mut cache = self.0.borrow_mut();
        if let Some(resolved) = cache.reads.get(path) {
            return resolved.clone();
        }
        let result = match cache.read(path) {
            Ok(contents) => ProfileRead::Bytes(contents),
            Err(error) => ProfileRead::Error {
                kind: error.kind(),
                os_code: error.raw_os_error(),
                message: error.to_string(),
            },
        };
        let resolved = ResolvedProfile(Arc::new(result));
        cache.reads.insert(path.to_owned(), resolved.clone());
        resolved
    }

    pub(in crate::native_artifact::program) fn report_identity(
        &self,
    ) -> ([u8; 32], Vec<serde_json::Value>) {
        let mut hash = Sha256::new();
        bytes(&mut hash, b"mech-native-selected-profiles-v2");
        let cache = self.0.borrow();
        bytes(&mut hash, &(cache.reads.len() as u64).to_le_bytes());
        let rows = cache
            .reads
            .iter()
            .map(|(path, resolved)| {
                let os_bytes = path.as_os_str().as_encoded_bytes();
                bytes(&mut hash, os_bytes);
                let mut row = serde_json::json!({
                    "path": path.to_string_lossy(), "path_os_bytes_hex": hex(os_bytes),
                });
                match resolved.0.as_ref() {
                    ProfileRead::Bytes(contents) => {
                        bytes(&mut hash, b"read");
                        let digest = Sha256::digest(contents);
                        bytes(&mut hash, &digest);
                        row["sha256"] = hex(&digest).into();
                        row["bytes"] = contents.len().into();
                    }
                    ProfileRead::Error { kind, os_code, .. } => {
                        bytes(&mut hash, b"error");
                        let kind = format!("{kind:?}");
                        bytes(&mut hash, kind.as_bytes());
                        if let Some(code) = os_code {
                            bytes(&mut hash, b"some");
                            bytes(&mut hash, &code.to_le_bytes());
                        } else {
                            bytes(&mut hash, b"none");
                        }
                        row["error_kind"] = kind.into();
                        row["os_error"] = serde_json::json!(os_code);
                    }
                }
                row
            })
            .collect();
        (hash.finalize().into(), rows)
    }
}

impl ResolvedProfile {
    pub(in crate::native_artifact::program) fn bytes(&self) -> io::Result<&[u8]> {
        match self.0.as_ref() {
            ProfileRead::Bytes(contents) => Ok(contents),
            ProfileRead::Error {
                kind,
                os_code: None,
                message,
            } => Err(io::Error::new(*kind, message.clone())),
            ProfileRead::Error {
                os_code: Some(code),
                ..
            } => Err(io::Error::from_raw_os_error(*code)),
        }
    }
}
