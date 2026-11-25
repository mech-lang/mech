//! Captured inputs for one native compilation.
use cranelift_codegen::isa::{Aarch64CompilationOptions, Aarch64MaskedFields};
use std::{collections::BTreeMap, env::VarError, ffi::OsString, sync::Arc};
#[cfg(feature = "canonical-native-program")]
use std::{
    io,
    path::{Path, PathBuf},
};

#[cfg(feature = "canonical-native-program")]
const SYMBOL_PROFILES: &[&str] = &[
    "MECH_COMPACT_PHASE_BLOCK_LAYOUT",
    "MECH_COMPACT_PHASE_REGISTER_WEIGHTS",
];
#[cfg(feature = "canonical-native-program")]
const ROOT_TAIL_PROFILE: &str = "MECH_COMPACT_ROOT_TAIL_PROFILE";

#[derive(Clone)]
pub(in crate::native_artifact) struct CompilationInputs(Arc<Captured>);

struct Captured {
    values: BTreeMap<OsString, OsString>,
    #[cfg(feature = "canonical-native-program")]
    profiles: BTreeMap<PathBuf, io::Result<Vec<u8>>>,
}

impl CompilationInputs {
    pub fn legacy() -> Self {
        Self::capture(std::env::vars_os())
    }

    pub fn capture(values: impl IntoIterator<Item = (OsString, OsString)>) -> Self {
        let values = values.into_iter();
        // Windows environment keys are case-insensitive.
        #[cfg(windows)]
        let values = values.map(|(key, value)| (key.to_ascii_uppercase(), value));
        let values: BTreeMap<_, _> = values
            .filter(|(key, _)| key.as_encoded_bytes().starts_with(b"MECH_"))
            .collect();
        #[cfg(feature = "canonical-native-program")]
        let profiles = {
            let mut profiles = BTreeMap::new();
            let mut capture = |path: &Path| {
                // Only the phase selecting a profile observes its read error.
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
            profiles
        };
        Self(Arc::new(Captured {
            values,
            #[cfg(feature = "canonical-native-program")]
            profiles,
        }))
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

    pub fn aarch64_options(&self) -> Aarch64CompilationOptions {
        let bytes = self.var("MECH_ARM64_REUSE_I8_DEFINITIONS").ok();
        let words = self.var("MECH_ARM64_REUSE_I32_ARITHMETIC").ok();
        let enabled = |key| self.var_os(key).is_some_and(|value| value == "1");
        let present = |key| self.var_os(key).is_some();
        Aarch64CompilationOptions {
            reuse_prefetch_bases: enabled("MECH_ARM64_REUSE_PREFETCH_BASES"),
            loop_live_ranges: present("MECH_ARM64_LOOP_LIVE_RANGES"),
            subtract_branch: present("MECH_ARM64_SUBTRACT_BRANCH"),
            subtract_immediate_branch: present("MECH_ARM64_SUBTRACT_IMMEDIATE_BRANCH"),
            inspect_spill_cache: self.var_os("MECH_INSPECT_FP_SPILL_CACHE").map(Into::into),
            reuse_i8_loads: matches!(bytes.as_deref(), Some("1" | "all" | "load")),
            reuse_i8_compares: matches!(bytes.as_deref(), Some("1" | "all" | "compare")),
            reuse_i8_booleans: enabled("MECH_ARM64_REUSE_I8_BOOLEAN_DEFINITIONS"),
            reuse_i32_add: matches!(words.as_deref(), Some("1" | "all" | "add")),
            reuse_i32_sub: matches!(words.as_deref(), Some("1" | "all" | "sub")),
            reuse_i32_mul: matches!(words.as_deref(), Some("1" | "all" | "mul")),
            reuse_i32_shifts: enabled("MECH_ARM64_REUSE_I32_SHIFTS"),
            pack_narrow_halves: enabled("MECH_ARM64_PACK_NARROW_HALVES"),
            fold_shifted_extensions: enabled("MECH_ARM64_FOLD_SHIFTED_EXTENSIONS"),
            ccmp_shared_boolean: present("MECH_ARM64_CCMP_SHARED_BOOLEAN"),
            ccmp_branch_select: present("MECH_ARM64_CCMP_BRANCH_SELECT"),
            masked_fields: match self.var("MECH_ARM64_INSERT_MASKED_FIELDS").as_deref() {
                Ok("1") => Aarch64MaskedFields::Narrow,
                Ok("wide") => Aarch64MaskedFields::Wide,
                Ok("wide-copy") => Aarch64MaskedFields::WideCopy,
                _ => Aarch64MaskedFields::Off,
            },
        }
    }

    #[cfg(feature = "canonical-native-program")]
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

    #[cfg(feature = "canonical-native-program")]
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
