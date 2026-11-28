//! Captured inputs for one native compilation.
use cranelift_codegen::isa::{Aarch64CompilationOptions, Aarch64MaskedFields};
use std::{collections::BTreeMap, env::VarError, ffi::OsString, sync::Arc};
#[derive(Clone)]
pub(in crate::native_artifact) struct CompilationInputs(Arc<BTreeMap<OsString, OsString>>);

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
        Self(Arc::new(values))
    }

    pub fn var_os(&self, key: &str) -> Option<OsString> {
        self.0.get(std::ffi::OsStr::new(key)).cloned()
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
    pub fn raw_settings_identity(&self) -> ([u8; 32], usize) {
        use crate::native_artifact::identity::transcript::bytes;
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        bytes(&mut hash, b"mech-native-settings-v2");
        bytes(&mut hash, &(self.0.len() as u64).to_le_bytes());
        for (key, value) in self.0.iter() {
            bytes(&mut hash, key.as_encoded_bytes());
            bytes(&mut hash, value.as_encoded_bytes());
        }
        (hash.finalize().into(), self.0.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_settings_remain_lossless_and_shareable() {
        fn require_send_sync<T: Send + Sync>() {}
        require_send_sync::<CompilationInputs>();
        let key = "MECH_COMPACT_ROOT_TAIL_PROFILE";
        let path = OsString::from("a deliberately unresolved profile.json");
        let original = CompilationInputs::capture([
            (key.into(), path.clone()),
            ("UNRELATED_SETTINGS".into(), "ignored".into()),
        ]);
        let cloned = original.clone();
        assert_eq!(original.var_os(key), Some(path.clone()));
        assert_eq!(cloned.var_os(key), Some(path));
        assert_eq!(cloned.var_os("UNRELATED_SETTINGS"), None);
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
