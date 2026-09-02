#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::{collections::BTreeMap, string::String, sync::Arc};
use mech_core::{CanonicalFunctionSpecializer, MResult, MechError, MechErrorKind, hash_str};
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::collections::BTreeMap;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::string::String;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::sync::Arc;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExtensionFunctionId(u64);

impl ExtensionFunctionId {
    pub fn from_name(name: &str) -> Self {
        Self(hash_str(name))
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[derive(Clone)]
pub struct FunctionExtensionEntry {
    pub id: ExtensionFunctionId,
    pub canonical_name: String,
    pub specializer: Arc<dyn CanonicalFunctionSpecializer>,
}

impl FunctionExtensionEntry {
    pub fn new(
        canonical_name: impl Into<String>,
        specializer: Arc<dyn CanonicalFunctionSpecializer>,
    ) -> Self {
        let canonical_name = canonical_name.into();
        Self {
            id: ExtensionFunctionId::from_name(&canonical_name),
            canonical_name,
            specializer,
        }
    }
}

#[derive(Clone, Default)]
pub struct FunctionExtensions {
    entries: BTreeMap<ExtensionFunctionId, FunctionExtensionEntry>,
    dictionary: BTreeMap<ExtensionFunctionId, String>,
    module_exports: BTreeMap<(String, String), ExtensionFunctionId>,
}

impl FunctionExtensions {
    pub fn entry(&self, id: ExtensionFunctionId) -> Option<&FunctionExtensionEntry> {
        self.entries.get(&id)
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = &FunctionExtensionEntry> + '_ {
        self.entries.values()
    }

    /// Inserts a program-local extension, deliberately replacing an existing
    /// implementation only when the stable ID belongs to the exact same name.
    pub fn insert_or_replace(
        &mut self,
        entry: FunctionExtensionEntry,
    ) -> MResult<Option<FunctionExtensionEntry>> {
        validate_entry(&entry)?;

        if let Some(existing_name) = self.dictionary.get(&entry.id)
            && existing_name != &entry.canonical_name
        {
            return Err(MechError::new(
                FunctionExtensionIdCollision {
                    id: entry.id,
                    existing_name: existing_name.clone(),
                    incoming_name: entry.canonical_name,
                },
                None,
            )
            .with_compiler_loc());
        }

        self.dictionary
            .insert(entry.id, entry.canonical_name.clone());
        Ok(self.entries.insert(entry.id, entry))
    }

    pub fn module_export(&self, module: &str, item: &str) -> Option<ExtensionFunctionId> {
        self.module_exports
            .get(&(String::from(module), String::from(item)))
            .copied()
    }

    /// Installs or deliberately replaces one exact dynamic module export.
    pub fn insert_module_export_or_replace(
        &mut self,
        module: impl Into<String>,
        item: impl Into<String>,
        extension: ExtensionFunctionId,
    ) -> MResult<Option<ExtensionFunctionId>> {
        let module = module.into();
        let item = item.into();

        if module.is_empty() || item.is_empty() {
            return Err(invalid_module_export(
                module,
                item,
                extension,
                "module and item names must not be empty",
            ));
        }
        if !self.entries.contains_key(&extension) {
            return Err(invalid_module_export(
                module,
                item,
                extension,
                "the referenced extension is not installed",
            ));
        }

        Ok(self.module_exports.insert((module, item), extension))
    }

    pub fn module_exports(
        &self,
    ) -> impl ExactSizeIterator<Item = (&(String, String), &ExtensionFunctionId)> + '_ {
        self.module_exports.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn validate_entry(entry: &FunctionExtensionEntry) -> MResult<()> {
    if entry.canonical_name.is_empty() {
        return Err(MechError::new(
            FunctionExtensionInvalidEntry {
                id: entry.id,
                canonical_name: entry.canonical_name.clone(),
                reason: String::from("canonical name must not be empty"),
            },
            None,
        )
        .with_compiler_loc());
    }

    let expected = ExtensionFunctionId::from_name(&entry.canonical_name);
    if expected != entry.id {
        return Err(MechError::new(
            FunctionExtensionInvalidEntry {
                id: entry.id,
                canonical_name: entry.canonical_name.clone(),
                reason: format!(
                    "canonical name hashes to 0x{:016x}, not 0x{:016x}",
                    expected.raw(),
                    entry.id.raw(),
                ),
            },
            None,
        )
        .with_compiler_loc());
    }

    Ok(())
}

fn invalid_module_export(
    module: String,
    item: String,
    extension: ExtensionFunctionId,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        FunctionExtensionInvalidModuleExport {
            module,
            item,
            extension,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionExtensionInvalidEntry {
    pub id: ExtensionFunctionId,
    pub canonical_name: String,
    pub reason: String,
}

impl MechErrorKind for FunctionExtensionInvalidEntry {
    fn name(&self) -> &str {
        "FunctionExtensionInvalidEntry"
    }

    fn message(&self) -> String {
        format!(
            "invalid function extension {:?} at ID 0x{:016x}: {}",
            self.canonical_name,
            self.id.raw(),
            self.reason,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionExtensionIdCollision {
    pub id: ExtensionFunctionId,
    pub existing_name: String,
    pub incoming_name: String,
}

impl MechErrorKind for FunctionExtensionIdCollision {
    fn name(&self) -> &str {
        "FunctionExtensionIdCollision"
    }

    fn message(&self) -> String {
        format!(
            "function extension names {:?} and {:?} collide at ID 0x{:016x}",
            self.existing_name,
            self.incoming_name,
            self.id.raw(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionExtensionInvalidModuleExport {
    pub module: String,
    pub item: String,
    pub extension: ExtensionFunctionId,
    pub reason: String,
}

impl MechErrorKind for FunctionExtensionInvalidModuleExport {
    fn name(&self) -> &str {
        "FunctionExtensionInvalidModuleExport"
    }

    fn message(&self) -> String {
        format!(
            "invalid function extension export {:?}/{:?} for ID 0x{:016x}: {}",
            self.module,
            self.item,
            self.extension.raw(),
            self.reason,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionExtensionUnavailable {
    pub id: ExtensionFunctionId,
    pub canonical_name: Option<String>,
}

impl MechErrorKind for FunctionExtensionUnavailable {
    fn name(&self) -> &str {
        "FunctionExtensionUnavailable"
    }

    fn message(&self) -> String {
        match &self.canonical_name {
            Some(canonical_name) => format!(
                "function extension `{canonical_name}` (0x{:016x}) is unavailable in this program",
                self.id.raw(),
            ),
            None => format!(
                "function extension 0x{:016x} is unavailable in this program",
                self.id.raw(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{
        OperationId, SpecializationContext, SpecializationInvocation, SpecializedFunction,
    };

    struct TestSpecializer;

    impl CanonicalFunctionSpecializer for TestSpecializer {
        fn specialize_invocation(
            &self,
            _: &SpecializationInvocation,
            _: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            unreachable!("extension-store tests do not specialize functions")
        }
    }

    fn specializer() -> Arc<dyn CanonicalFunctionSpecializer> {
        Arc::new(TestSpecializer)
    }

    #[test]
    fn exact_same_name_replacement_preserves_the_stable_id() {
        let first = specializer();
        let replacement = specializer();
        let mut extensions = FunctionExtensions::default();

        assert!(
            extensions
                .insert_or_replace(FunctionExtensionEntry::new("host/read", first.clone()))
                .unwrap()
                .is_none()
        );
        let first_canonical = extensions
            .entry(ExtensionFunctionId::from_name("host/read"))
            .unwrap()
            .specializer
            .clone();
        let replaced = extensions
            .insert_or_replace(FunctionExtensionEntry::new(
                "host/read",
                replacement.clone(),
            ))
            .unwrap()
            .unwrap();
        let replacement_canonical = extensions
            .entry(ExtensionFunctionId::from_name("host/read"))
            .unwrap()
            .specializer
            .clone();

        let id = ExtensionFunctionId::from_name("host/read");
        assert!(Arc::ptr_eq(&replaced.specializer, &first_canonical));
        assert!(Arc::ptr_eq(
            &extensions.entry(id).unwrap().specializer,
            &replacement_canonical,
        ));
    }

    #[test]
    fn distinct_names_at_one_forced_id_are_rejected_without_replacement() {
        let id = ExtensionFunctionId::from_name("second");
        let mut extensions = FunctionExtensions::default();
        extensions.dictionary.insert(id, String::from("first"));
        extensions.entries.insert(
            id,
            FunctionExtensionEntry {
                id,
                canonical_name: String::from("first"),
                specializer: specializer(),
            },
        );

        let error = match extensions.insert_or_replace(FunctionExtensionEntry {
            id,
            canonical_name: String::from("second"),
            specializer: specializer(),
        }) {
            Ok(_) => panic!("colliding extension name unexpectedly replaced the entry"),
            Err(error) => error,
        };

        assert_eq!(error.kind_name(), "FunctionExtensionIdCollision");
        assert_eq!(extensions.entry(id).unwrap().canonical_name, "first");
    }

    #[test]
    fn module_exports_are_exact_and_clone_the_entry_ownership() {
        let mut extensions = FunctionExtensions::default();
        let specializer = specializer();
        let id = ExtensionFunctionId::from_name("dynamic/math/sin");
        extensions
            .insert_or_replace(FunctionExtensionEntry::new(
                "dynamic/math/sin",
                specializer.clone(),
            ))
            .unwrap();
        let installed = extensions.entry(id).unwrap().specializer.clone();
        extensions
            .insert_module_export_or_replace("dynamic/math", "sin", id)
            .unwrap();

        let cloned = extensions.clone();
        assert_eq!(cloned.module_export("dynamic/math", "sin"), Some(id),);
        assert_eq!(cloned.module_export("dynamic", "math/sin"), None);
        assert!(Arc::ptr_eq(
            &cloned.entry(id).unwrap().specializer,
            &installed,
        ));
    }

    #[test]
    fn module_exports_require_an_installed_extension() {
        let mut extensions = FunctionExtensions::default();
        let error = extensions
            .insert_module_export_or_replace(
                "dynamic/math",
                "sin",
                ExtensionFunctionId::from_name("dynamic/math/sin"),
            )
            .unwrap_err();

        assert_eq!(error.kind_name(), "FunctionExtensionInvalidModuleExport",);
        assert!(extensions.module_exports.is_empty());
    }

    #[test]
    fn extension_ids_do_not_conflate_with_catalog_operation_types() {
        let name = "host/read";
        assert_eq!(
            ExtensionFunctionId::from_name(name).raw(),
            OperationId::from_name(name).raw(),
        );
    }
}
