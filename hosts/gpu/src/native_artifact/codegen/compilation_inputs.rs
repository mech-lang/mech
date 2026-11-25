//! Captured inputs for one native compilation.
use std::{
    collections::BTreeMap,
    env::VarError,
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

const SYMBOL_PROFILES: &[&str] = &[
    "MECH_COMPACT_PHASE_BLOCK_LAYOUT",
    "MECH_COMPACT_PHASE_REGISTER_WEIGHTS",
];
const ROOT_TAIL_PROFILE: &str = "MECH_COMPACT_ROOT_TAIL_PROFILE";

#[derive(Clone)]
pub(in crate::native_artifact) struct CompilationInputs(Arc<Captured>);

struct Captured {
    values: BTreeMap<OsString, OsString>,
    profiles: BTreeMap<PathBuf, io::Result<Vec<u8>>>,
}

impl CompilationInputs {
    pub fn legacy() -> Self {
        Self::capture(std::env::vars_os())
    }

    pub fn capture(values: impl IntoIterator<Item = (OsString, OsString)>) -> Self {
        let values: BTreeMap<_, _> = values
            .into_iter()
            .filter(|(key, _)| key.as_encoded_bytes().starts_with(b"MECH_"))
            .collect();
        let mut profiles = BTreeMap::new();
        let mut capture = |path: &Path| {
            // Store errors alongside bytes. Only the phase selecting this
            // profile will observe a read error or parse its contents.
            if !profiles.contains_key(path) {
                profiles.insert(path.to_owned(), std::fs::read(path));
            }
        };
        for key in SYMBOL_PROFILES {
            if let Some(value) = values
                .get(std::ffi::OsStr::new(key))
                .and_then(|value| value.to_str())
            {
                for (_, path) in value.split(';').filter_map(|entry| entry.split_once('@')) {
                    capture(Path::new(path));
                }
            }
        }
        if let Some(path) = values.get(std::ffi::OsStr::new(ROOT_TAIL_PROFILE)) {
            capture(Path::new(path));
        }
        Self(Arc::new(Captured { values, profiles }))
    }

    pub fn var_os(&self, key: &str) -> Option<OsString> {
        self.0.values.get(std::ffi::OsStr::new(key)).cloned()
    }

    pub fn var(&self, key: &str) -> Result<String, VarError> {
        self.var_os(key)
            .ok_or(VarError::NotPresent)?
            .into_string()
            .map_err(VarError::NotUnicode)
    }

    pub fn profile(&self, path: &Path) -> io::Result<&[u8]> {
        let record = self.0.profiles.get(path).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "profile path was not captured")
        })?;
        match record {
            Ok(bytes) => Ok(bytes),
            Err(error) => Err(match error.raw_os_error() {
                Some(code) => io::Error::from_raw_os_error(code),
                None => io::Error::new(error.kind(), error.to_string()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_profile_retains_bytes_across_replacement_and_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("layout.json");
        std::fs::write(&path, b"first").unwrap();
        let capture = || {
            CompilationInputs::capture([(ROOT_TAIL_PROFILE.into(), path.as_os_str().to_owned())])
        };
        let first = capture();
        std::fs::write(&path, b"second").unwrap();
        let second = capture();
        std::fs::remove_file(&path).unwrap();
        let unavailable = capture();
        assert_eq!(first.profile(&path).unwrap(), b"first");
        assert_eq!(second.profile(&path).unwrap(), b"second");
        assert_eq!(
            unavailable.profile(&path).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn empty_value_is_present_and_missing_value_is_absent() {
        let setting = "MECH_COMPACT_FOLD_REDUNDANT_ZERO";
        let present = CompilationInputs::capture([(setting.into(), "".into())]);
        let absent = CompilationInputs::capture([]);
        assert_eq!(present.var_os(setting), Some("".into()));
        assert_eq!(present.var(setting).unwrap(), "");
        assert!(matches!(absent.var(setting), Err(VarError::NotPresent)));
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_value_keeps_its_original_bytes() {
        use std::os::unix::ffi::OsStringExt;
        let key = "MECH_COMPACT_PHASE_BLOCK_LAYOUT";
        let original = OsString::from_vec(vec![255]);
        let inputs = CompilationInputs::capture([(key.into(), original.clone())]);
        assert_eq!(inputs.var_os(key), Some(original.clone()));
        assert!(matches!(inputs.var(key), Err(VarError::NotUnicode(value)) if value == original));
    }
}
