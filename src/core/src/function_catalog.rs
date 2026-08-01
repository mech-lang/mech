#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};

use crate::{
    FunctionArgs, GuardFunctionSafety, MResult, MechError, MechErrorKind, MechFunction,
    NativeFunctionCompiler, Value, hash_str,
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(u64);

impl OperationId {
    pub fn from_name(name: &str) -> Self {
        Self(hash_str(name))
    }

    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeFunctionId(u64);

impl RuntimeFunctionId {
    pub fn from_name(name: &str) -> Self {
        Self(hash_str(name))
    }

    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }
}

pub type RuntimeFunctionFactory = fn(FunctionArgs) -> MResult<Box<dyn MechFunction>>;

pub trait FunctionSpecializer: Send + Sync {
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>>;

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }
}

/// Transitional adapter used while static source compilers move from the
/// legacy registry into explicit catalog fragments.
///
/// PR2 deletes this adapter together with [`NativeFunctionCompiler`] after all
/// concrete specializers implement the catalog interface directly.
pub struct LegacySourceSpecializer<T> {
    compiler: T,
}

impl<T> LegacySourceSpecializer<T> {
    pub fn new(compiler: T) -> Self {
        Self { compiler }
    }
}

impl<T> FunctionSpecializer for LegacySourceSpecializer<T>
where
    T: NativeFunctionCompiler,
{
    fn specialize(&self, arguments: &[Value]) -> MResult<Box<dyn MechFunction>> {
        self.compiler.compile(&arguments.to_vec())
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        self.compiler.guard_safety()
    }
}

pub fn legacy_source_specializer<T>(compiler: T) -> Arc<dyn FunctionSpecializer>
where
    T: NativeFunctionCompiler + 'static,
{
    Arc::new(LegacySourceSpecializer::new(compiler))
}

#[derive(Clone)]
pub struct RuntimeFunctionEntry {
    pub id: RuntimeFunctionId,
    pub name: String,
    pub factory: RuntimeFunctionFactory,
}

#[derive(Clone)]
pub struct FunctionSpecializerEntry {
    pub operation: OperationId,
    pub canonical_name: String,
    pub specializer: Arc<dyn FunctionSpecializer>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionExposure {
    Internal,
    Prelude,
    ModuleOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionExport {
    pub operation: OperationId,
    pub canonical_name: String,
    pub module: Option<String>,
    pub item: Option<String>,
    pub exposure: FunctionExposure,
}

pub struct FunctionCatalog {
    runtime_factories: BTreeMap<RuntimeFunctionId, RuntimeFunctionEntry>,
    specializers: BTreeMap<OperationId, FunctionSpecializerEntry>,
    intrinsic_specializers: BTreeMap<OperationId, FunctionSpecializerEntry>,
    exports_by_module_item: BTreeMap<(String, String), FunctionExport>,
    exports_by_operation: BTreeMap<OperationId, Vec<FunctionExport>>,
}

impl FunctionCatalog {
    pub fn runtime_factory(&self, id: RuntimeFunctionId) -> Option<RuntimeFunctionFactory> {
        self.runtime_factories.get(&id).map(|entry| entry.factory)
    }

    pub fn runtime_entry(&self, id: RuntimeFunctionId) -> Option<&RuntimeFunctionEntry> {
        self.runtime_factories.get(&id)
    }

    pub fn runtime_entries(&self) -> impl ExactSizeIterator<Item = &RuntimeFunctionEntry> + '_ {
        self.runtime_factories.values()
    }

    pub fn specializer(&self, operation: OperationId) -> Option<&FunctionSpecializerEntry> {
        self.specializers.get(&operation)
    }

    /// Resolves a parser-only language intrinsic. Intrinsics are deliberately
    /// absent from named source lookup and the frozen named-specializer count.
    pub fn intrinsic_specializer(
        &self,
        operation: OperationId,
    ) -> Option<&FunctionSpecializerEntry> {
        self.intrinsic_specializers.get(&operation)
    }

    /// Resolves either a named static specializer or a parser-only intrinsic.
    pub fn operation_specializer(
        &self,
        operation: OperationId,
    ) -> Option<&FunctionSpecializerEntry> {
        self.specializer(operation)
            .or_else(|| self.intrinsic_specializer(operation))
    }

    pub fn specializer_entries(
        &self,
    ) -> impl ExactSizeIterator<Item = &FunctionSpecializerEntry> + '_ {
        self.specializers.values()
    }

    pub fn intrinsic_specializer_entries(
        &self,
    ) -> impl ExactSizeIterator<Item = &FunctionSpecializerEntry> + '_ {
        self.intrinsic_specializers.values()
    }

    pub fn exports_for_operation(&self, operation: OperationId) -> &[FunctionExport] {
        self.exports_by_operation
            .get(&operation)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn module_export(&self, module: &str, item: &str) -> Option<&FunctionExport> {
        self.exports_by_module_item
            .get(&(String::from(module), String::from(item)))
    }

    pub fn runtime_factory_count(&self) -> usize {
        self.runtime_factories.len()
    }

    pub fn specializer_count(&self) -> usize {
        self.specializers.len()
    }

    pub fn intrinsic_specializer_count(&self) -> usize {
        self.intrinsic_specializers.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogRuntimeIdCollision {
    pub id: RuntimeFunctionId,
    pub existing_name: String,
    pub incoming_name: String,
}

impl MechErrorKind for FunctionCatalogRuntimeIdCollision {
    fn name(&self) -> &str {
        "FunctionCatalogRuntimeIdCollision"
    }

    fn message(&self) -> String {
        format!(
            "runtime function names {:?} and {:?} collide at ID 0x{:016x}",
            self.existing_name,
            self.incoming_name,
            self.id.raw(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogOperationIdCollision {
    pub operation: OperationId,
    pub existing_name: String,
    pub incoming_name: String,
}

impl MechErrorKind for FunctionCatalogOperationIdCollision {
    fn name(&self) -> &str {
        "FunctionCatalogOperationIdCollision"
    }

    fn message(&self) -> String {
        format!(
            "function operation names {:?} and {:?} collide at ID 0x{:016x}",
            self.existing_name,
            self.incoming_name,
            self.operation.raw(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogDuplicateRuntimeFactory {
    pub id: RuntimeFunctionId,
    pub name: String,
}

impl MechErrorKind for FunctionCatalogDuplicateRuntimeFactory {
    fn name(&self) -> &str {
        "FunctionCatalogDuplicateRuntimeFactory"
    }

    fn message(&self) -> String {
        format!(
            "runtime factory {:?} is already registered at ID 0x{:016x}",
            self.name,
            self.id.raw(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogDuplicateSpecializer {
    pub operation: OperationId,
    pub canonical_name: String,
}

impl MechErrorKind for FunctionCatalogDuplicateSpecializer {
    fn name(&self) -> &str {
        "FunctionCatalogDuplicateSpecializer"
    }

    fn message(&self) -> String {
        format!(
            "specializer {:?} is already registered for operation 0x{:016x}",
            self.canonical_name,
            self.operation.raw(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogInvalidExport {
    pub operation: OperationId,
    pub canonical_name: String,
    pub module: Option<String>,
    pub item: Option<String>,
    pub reason: String,
}

impl MechErrorKind for FunctionCatalogInvalidExport {
    fn name(&self) -> &str {
        "FunctionCatalogInvalidExport"
    }

    fn message(&self) -> String {
        format!(
            "invalid export {:?} for operation 0x{:016x} (module {:?}, item {:?}): {}",
            self.canonical_name,
            self.operation.raw(),
            self.module,
            self.item,
            self.reason,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogMissingExportSpecializer {
    pub operation: OperationId,
    pub canonical_name: String,
}

impl MechErrorKind for FunctionCatalogMissingExportSpecializer {
    fn name(&self) -> &str {
        "FunctionCatalogMissingExportSpecializer"
    }

    fn message(&self) -> String {
        format!(
            "export {:?} refers to operation 0x{:016x}, which has no registered specializer",
            self.canonical_name,
            self.operation.raw(),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogInvalidName {
    pub category: &'static str,
    pub name: String,
    pub id: u64,
}

impl MechErrorKind for FunctionCatalogInvalidName {
    fn name(&self) -> &str {
        "FunctionCatalogInvalidName"
    }

    fn message(&self) -> String {
        format!(
            "{} name {:?} is invalid at ID 0x{:016x}: names must not be empty",
            self.category, self.name, self.id,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionOperationNotVisible {
    pub operation: OperationId,
    pub canonical_name: Option<String>,
}

impl MechErrorKind for FunctionOperationNotVisible {
    fn name(&self) -> &str {
        "FunctionOperationNotVisible"
    }

    fn message(&self) -> String {
        match &self.canonical_name {
            Some(canonical_name) => format!(
                "function operation `{canonical_name}` (0x{:016x}) is not visible in this program",
                self.operation.raw(),
            ),
            None => format!(
                "function operation 0x{:016x} is not visible in this program",
                self.operation.raw(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionOperationUnavailable {
    pub operation: OperationId,
    pub canonical_name: Option<String>,
}

impl MechErrorKind for FunctionOperationUnavailable {
    fn name(&self) -> &str {
        "FunctionOperationUnavailable"
    }

    fn message(&self) -> String {
        match &self.canonical_name {
            Some(canonical_name) => format!(
                "function operation `{canonical_name}` (0x{:016x}) is unavailable in the catalog",
                self.operation.raw(),
            ),
            None => format!(
                "function operation 0x{:016x} is unavailable in the catalog",
                self.operation.raw(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFunctionUnavailable {
    pub id: RuntimeFunctionId,
}

impl MechErrorKind for RuntimeFunctionUnavailable {
    fn name(&self) -> &str {
        "RuntimeFunctionUnavailable"
    }

    fn message(&self) -> String {
        format!(
            "runtime function 0x{:016x} is unavailable in the catalog",
            self.id.raw(),
        )
    }
}

pub struct FunctionCatalogBuilder {
    runtime_factories: BTreeMap<RuntimeFunctionId, RuntimeFunctionEntry>,
    specializers: BTreeMap<OperationId, FunctionSpecializerEntry>,
    intrinsic_specializers: BTreeMap<OperationId, FunctionSpecializerEntry>,
    exports_by_module_item: BTreeMap<(String, String), FunctionExport>,
    exports_by_operation: BTreeMap<OperationId, Vec<FunctionExport>>,
}

impl Default for FunctionCatalogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionCatalogBuilder {
    pub fn new() -> Self {
        Self {
            runtime_factories: BTreeMap::new(),
            specializers: BTreeMap::new(),
            intrinsic_specializers: BTreeMap::new(),
            exports_by_module_item: BTreeMap::new(),
            exports_by_operation: BTreeMap::new(),
        }
    }

    pub fn insert_runtime_factory(
        &mut self,
        name: impl Into<String>,
        factory: RuntimeFunctionFactory,
    ) -> MResult<()> {
        let name = name.into();
        let id = RuntimeFunctionId::from_name(&name);
        self.insert_runtime_entry(RuntimeFunctionEntry { id, name, factory })
    }

    pub fn insert_specializer(
        &mut self,
        canonical_name: impl Into<String>,
        specializer: Arc<dyn FunctionSpecializer>,
    ) -> MResult<OperationId> {
        let canonical_name = canonical_name.into();
        let operation = OperationId::from_name(&canonical_name);
        self.insert_specializer_entry(FunctionSpecializerEntry {
            operation,
            canonical_name,
            specializer,
        })
    }

    pub fn insert_intrinsic_specializer(
        &mut self,
        canonical_name: impl Into<String>,
        specializer: Arc<dyn FunctionSpecializer>,
    ) -> MResult<OperationId> {
        let canonical_name = canonical_name.into();
        let operation = OperationId::from_name(&canonical_name);
        self.insert_intrinsic_specializer_entry(FunctionSpecializerEntry {
            operation,
            canonical_name,
            specializer,
        })
    }

    pub fn insert_export(&mut self, export: FunctionExport) -> MResult<()> {
        validate_export(&export)?;

        if let Some(existing) = self.specializers.get(&export.operation)
            && existing.canonical_name != export.canonical_name
        {
            return Err(MechError::new(
                FunctionCatalogOperationIdCollision {
                    operation: export.operation,
                    existing_name: existing.canonical_name.clone(),
                    incoming_name: export.canonical_name.clone(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let module_key = match (&export.module, &export.item) {
            (Some(module), Some(item)) => Some((module.clone(), item.clone())),
            (None, None) => None,
            _ => unreachable!("validate_export rejects incomplete module/item pairs"),
        };

        if let Some(key) = &module_key
            && let Some(existing) = self.exports_by_module_item.get(key)
        {
            return Err(invalid_export_error(
                &export,
                format!(
                    "module item {:?}/{:?} conflicts with existing export {:?} (operation 0x{:016x})",
                    key.0,
                    key.1,
                    existing.canonical_name,
                    existing.operation.raw(),
                ),
            ));
        }

        if self
            .exports_by_operation
            .get(&export.operation)
            .is_some_and(|exports| exports.contains(&export))
        {
            return Err(invalid_export_error(
                &export,
                String::from("duplicate export"),
            ));
        }

        if let Some(key) = module_key {
            self.exports_by_module_item.insert(key, export.clone());
        }
        self.exports_by_operation
            .entry(export.operation)
            .or_default()
            .push(export);
        Ok(())
    }

    pub fn build(self) -> MResult<FunctionCatalog> {
        for (operation, exports) in &self.exports_by_operation {
            let Some(specializer) = self.specializers.get(operation) else {
                let export = &exports[0];
                return Err(MechError::new(
                    FunctionCatalogMissingExportSpecializer {
                        operation: *operation,
                        canonical_name: export.canonical_name.clone(),
                    },
                    None,
                )
                .with_compiler_loc());
            };

            for export in exports {
                validate_export(export)?;
                if specializer.canonical_name != export.canonical_name {
                    return Err(MechError::new(
                        FunctionCatalogOperationIdCollision {
                            operation: *operation,
                            existing_name: specializer.canonical_name.clone(),
                            incoming_name: export.canonical_name.clone(),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            }
        }

        let catalog = FunctionCatalog {
            runtime_factories: self.runtime_factories,
            specializers: self.specializers,
            intrinsic_specializers: self.intrinsic_specializers,
            exports_by_module_item: self.exports_by_module_item,
            exports_by_operation: self.exports_by_operation,
        };

        Ok(catalog)
    }

    fn insert_runtime_entry(&mut self, entry: RuntimeFunctionEntry) -> MResult<()> {
        if entry.name.is_empty() {
            return Err(MechError::new(
                FunctionCatalogInvalidName {
                    category: "runtime function",
                    name: entry.name,
                    id: entry.id.raw(),
                },
                None,
            )
            .with_compiler_loc());
        }

        if let Some(existing) = self.runtime_factories.get(&entry.id) {
            let kind = if existing.name == entry.name {
                MechError::new(
                    FunctionCatalogDuplicateRuntimeFactory {
                        id: entry.id,
                        name: entry.name,
                    },
                    None,
                )
            } else {
                MechError::new(
                    FunctionCatalogRuntimeIdCollision {
                        id: entry.id,
                        existing_name: existing.name.clone(),
                        incoming_name: entry.name,
                    },
                    None,
                )
            };
            return Err(kind.with_compiler_loc());
        }

        self.runtime_factories.insert(entry.id, entry);
        Ok(())
    }

    fn insert_specializer_entry(
        &mut self,
        entry: FunctionSpecializerEntry,
    ) -> MResult<OperationId> {
        if entry.canonical_name.is_empty() {
            return Err(MechError::new(
                FunctionCatalogInvalidName {
                    category: "function operation",
                    name: entry.canonical_name,
                    id: entry.operation.raw(),
                },
                None,
            )
            .with_compiler_loc());
        }

        if let Some(existing) = self
            .specializers
            .get(&entry.operation)
            .or_else(|| self.intrinsic_specializers.get(&entry.operation))
        {
            let kind = if existing.canonical_name == entry.canonical_name {
                MechError::new(
                    FunctionCatalogDuplicateSpecializer {
                        operation: entry.operation,
                        canonical_name: entry.canonical_name,
                    },
                    None,
                )
            } else {
                MechError::new(
                    FunctionCatalogOperationIdCollision {
                        operation: entry.operation,
                        existing_name: existing.canonical_name.clone(),
                        incoming_name: entry.canonical_name,
                    },
                    None,
                )
            };
            return Err(kind.with_compiler_loc());
        }

        let operation = entry.operation;
        self.specializers.insert(operation, entry);
        Ok(operation)
    }

    fn insert_intrinsic_specializer_entry(
        &mut self,
        entry: FunctionSpecializerEntry,
    ) -> MResult<OperationId> {
        if entry.canonical_name.is_empty() {
            return Err(MechError::new(
                FunctionCatalogInvalidName {
                    category: "function intrinsic",
                    name: entry.canonical_name,
                    id: entry.operation.raw(),
                },
                None,
            )
            .with_compiler_loc());
        }

        if let Some(existing) = self
            .intrinsic_specializers
            .get(&entry.operation)
            .or_else(|| self.specializers.get(&entry.operation))
        {
            let kind = if existing.canonical_name == entry.canonical_name {
                MechError::new(
                    FunctionCatalogDuplicateSpecializer {
                        operation: entry.operation,
                        canonical_name: entry.canonical_name,
                    },
                    None,
                )
            } else {
                MechError::new(
                    FunctionCatalogOperationIdCollision {
                        operation: entry.operation,
                        existing_name: existing.canonical_name.clone(),
                        incoming_name: entry.canonical_name,
                    },
                    None,
                )
            };
            return Err(kind.with_compiler_loc());
        }

        let operation = entry.operation;
        self.intrinsic_specializers.insert(operation, entry);
        Ok(operation)
    }
}

fn invalid_export_error(export: &FunctionExport, reason: String) -> MechError {
    MechError::new(
        FunctionCatalogInvalidExport {
            operation: export.operation,
            canonical_name: export.canonical_name.clone(),
            module: export.module.clone(),
            item: export.item.clone(),
            reason,
        },
        None,
    )
    .with_compiler_loc()
}

fn validate_export(export: &FunctionExport) -> MResult<()> {
    if export.canonical_name.is_empty() {
        return Err(invalid_export_error(
            export,
            String::from("canonical name must not be empty"),
        ));
    }

    let expected = OperationId::from_name(&export.canonical_name);
    if expected != export.operation {
        return Err(invalid_export_error(
            export,
            format!(
                "canonical name hashes to 0x{:016x}, not 0x{:016x}",
                expected.raw(),
                export.operation.raw(),
            ),
        ));
    }

    match (&export.module, &export.item) {
        (Some(module), Some(item)) if module.is_empty() => Err(invalid_export_error(
            export,
            String::from("module name must not be empty"),
        )),
        (Some(_), Some(item)) if item.is_empty() => Err(invalid_export_error(
            export,
            String::from("item name must not be empty"),
        )),
        (Some(_), Some(_)) | (None, None) => Ok(()),
        _ => Err(invalid_export_error(
            export,
            String::from("module and item must either both be present or both be absent"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_factory(_: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        unreachable!("catalog tests do not instantiate runtime functions")
    }

    struct TestSpecializer;

    impl FunctionSpecializer for TestSpecializer {
        fn specialize(&self, _: &[Value]) -> MResult<Box<dyn MechFunction>> {
            unreachable!("catalog tests do not run specializers")
        }
    }

    fn test_specializer() -> Arc<dyn FunctionSpecializer> {
        Arc::new(TestSpecializer)
    }

    fn export(
        canonical_name: &str,
        module: Option<&str>,
        item: Option<&str>,
        exposure: FunctionExposure,
    ) -> FunctionExport {
        FunctionExport {
            operation: OperationId::from_name(canonical_name),
            canonical_name: String::from(canonical_name),
            module: module.map(String::from),
            item: item.map(String::from),
            exposure,
        }
    }

    #[test]
    fn operation_id_preserves_existing_math_add_hash() {
        assert_eq!(
            OperationId::from_name("math/add").raw(),
            0x00cc_5290_41cb_60c3
        );
    }

    #[test]
    fn runtime_function_id_preserves_existing_add_ss_f64_hash() {
        assert_eq!(
            RuntimeFunctionId::from_name("AddSS<f64>").raw(),
            0x000a_2c77_6884_86f3,
        );
    }

    #[test]
    fn distinct_runtime_names_with_the_same_raw_id_are_rejected() {
        let mut builder = FunctionCatalogBuilder::new();
        let id = RuntimeFunctionId::from_raw(0x2a);
        builder
            .insert_runtime_entry(RuntimeFunctionEntry {
                id,
                name: String::from("first"),
                factory: test_factory,
            })
            .unwrap();

        let error = builder
            .insert_runtime_entry(RuntimeFunctionEntry {
                id,
                name: String::from("second"),
                factory: test_factory,
            })
            .unwrap_err();

        assert_eq!(error.kind_name(), "FunctionCatalogRuntimeIdCollision");
        assert!(error.kind_message().contains("0x000000000000002a"));
        assert!(error.kind_message().contains("first"));
        assert!(error.kind_message().contains("second"));
    }

    #[test]
    fn distinct_specializer_names_with_the_same_raw_id_are_rejected() {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = OperationId::from_raw(0x2a);
        builder
            .insert_specializer_entry(FunctionSpecializerEntry {
                operation,
                canonical_name: String::from("first"),
                specializer: test_specializer(),
            })
            .unwrap();

        let error = builder
            .insert_specializer_entry(FunctionSpecializerEntry {
                operation,
                canonical_name: String::from("second"),
                specializer: test_specializer(),
            })
            .unwrap_err();

        assert_eq!(error.kind_name(), "FunctionCatalogOperationIdCollision");
        assert!(error.kind_message().contains("0x000000000000002a"));
    }

    #[test]
    fn duplicate_runtime_factories_are_rejected() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory("AddSS<f64>", test_factory)
            .unwrap();
        let error = builder
            .insert_runtime_factory("AddSS<f64>", test_factory)
            .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionCatalogDuplicateRuntimeFactory");
    }

    #[test]
    fn duplicate_specializers_are_rejected() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_specializer("math/add", test_specializer())
            .unwrap();
        let error = builder
            .insert_specializer("math/add", test_specializer())
            .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionCatalogDuplicateSpecializer");
    }

    #[test]
    fn parser_intrinsics_are_separate_from_named_specializers() {
        let mut builder = FunctionCatalogBuilder::new();
        let intrinsic = builder
            .insert_intrinsic_specializer("assign", test_specializer())
            .unwrap();
        builder
            .insert_specializer("math/add", test_specializer())
            .unwrap();

        let catalog = builder.build().unwrap();

        assert_eq!(catalog.specializer_count(), 1);
        assert_eq!(catalog.intrinsic_specializer_count(), 1);
        assert!(catalog.specializer(intrinsic).is_none());
        assert_eq!(
            catalog
                .intrinsic_specializer(intrinsic)
                .unwrap()
                .canonical_name,
            "assign",
        );
        assert_eq!(
            catalog
                .operation_specializer(intrinsic)
                .unwrap()
                .canonical_name,
            "assign",
        );
    }

    #[test]
    fn named_and_intrinsic_specializers_cannot_claim_the_same_operation() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_intrinsic_specializer("assign", test_specializer())
            .unwrap();

        let error = builder
            .insert_specializer("assign", test_specializer())
            .unwrap_err();

        assert_eq!(error.kind_name(), "FunctionCatalogDuplicateSpecializer");
    }

    #[test]
    fn exports_require_a_registered_specializer_at_build_time() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_export(export("math/add", None, None, FunctionExposure::Prelude))
            .unwrap();
        let error = builder.build().err().unwrap();
        assert_eq!(error.kind_name(), "FunctionCatalogMissingExportSpecializer");
        assert!(error.kind_message().contains("0x00cc529041cb60c3"));
    }

    #[test]
    fn malformed_module_item_pairs_are_rejected() {
        for malformed in [
            export("math/add", Some("math"), None, FunctionExposure::ModuleOnly),
            export("math/add", None, Some("add"), FunctionExposure::ModuleOnly),
            export(
                "math/add",
                Some(""),
                Some("add"),
                FunctionExposure::ModuleOnly,
            ),
            export(
                "math/add",
                Some("math"),
                Some(""),
                FunctionExposure::ModuleOnly,
            ),
        ] {
            let mut builder = FunctionCatalogBuilder::new();
            let error = builder.insert_export(malformed).unwrap_err();
            assert_eq!(error.kind_name(), "FunctionCatalogInvalidExport");
        }
    }

    #[test]
    fn conflicting_module_exports_are_rejected_without_overwriting() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_specializer("math/add", test_specializer())
            .unwrap();
        builder
            .insert_specializer("math/subtract", test_specializer())
            .unwrap();
        builder
            .insert_export(export(
                "math/add",
                Some("math"),
                Some("operator"),
                FunctionExposure::ModuleOnly,
            ))
            .unwrap();
        let error = builder
            .insert_export(export(
                "math/subtract",
                Some("math"),
                Some("operator"),
                FunctionExposure::ModuleOnly,
            ))
            .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionCatalogInvalidExport");

        let catalog = builder.build().unwrap();
        assert_eq!(
            catalog
                .module_export("math", "operator")
                .unwrap()
                .canonical_name,
            "math/add"
        );
    }

    #[test]
    fn exports_must_match_their_canonical_operation_id() {
        let mut invalid = export(
            "math/add",
            Some("math"),
            Some("add"),
            FunctionExposure::ModuleOnly,
        );
        invalid.operation = OperationId::from_name("math/subtract");
        let mut builder = FunctionCatalogBuilder::new();
        let error = builder.insert_export(invalid).unwrap_err();
        assert_eq!(error.kind_name(), "FunctionCatalogInvalidExport");
        assert!(error.kind_message().contains("0x00cc529041cb60c3"));
    }

    #[test]
    fn empty_canonical_names_are_rejected() {
        let mut builder = FunctionCatalogBuilder::new();
        let error = builder
            .insert_specializer("", test_specializer())
            .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionCatalogInvalidName");

        let error = builder
            .insert_export(export("", None, None, FunctionExposure::Prelude))
            .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionCatalogInvalidExport");
    }

    #[test]
    fn operation_diagnostics_include_names_when_available() {
        let operation = OperationId::from_name("math/add");
        assert_eq!(
            FunctionOperationNotVisible {
                operation,
                canonical_name: Some(String::from("math/add")),
            }
            .message(),
            "function operation `math/add` (0x00cc529041cb60c3) is not visible in this program",
        );
        assert_eq!(
            FunctionOperationUnavailable {
                operation,
                canonical_name: Some(String::from("math/add")),
            }
            .message(),
            "function operation `math/add` (0x00cc529041cb60c3) is unavailable in the catalog",
        );
        assert_eq!(
            FunctionOperationNotVisible {
                operation,
                canonical_name: None,
            }
            .message(),
            "function operation 0x00cc529041cb60c3 is not visible in this program",
        );
    }

    #[test]
    fn built_catalog_indexes_are_read_only_and_consistent() {
        let mut builder = FunctionCatalogBuilder::new();
        let operation = builder
            .insert_specializer("math/add", test_specializer())
            .unwrap();
        let runtime_id = RuntimeFunctionId::from_name("AddSS<f64>");
        builder
            .insert_runtime_factory("AddSS<f64>", test_factory)
            .unwrap();
        builder
            .insert_export(export(
                "math/add",
                Some("math"),
                Some("add"),
                FunctionExposure::ModuleOnly,
            ))
            .unwrap();

        let catalog = builder.build().unwrap();
        assert_eq!(catalog.runtime_factory_count(), 1);
        assert_eq!(catalog.specializer_count(), 1);
        assert_eq!(catalog.runtime_entries().count(), 1);
        assert!(catalog.runtime_factory(runtime_id).is_some());
        assert_eq!(
            catalog.runtime_entry(runtime_id).unwrap().name,
            "AddSS<f64>"
        );
        assert_eq!(
            catalog.specializer(operation).unwrap().canonical_name,
            "math/add"
        );
        assert_eq!(catalog.exports_for_operation(operation).len(), 1);
        assert_eq!(
            catalog.module_export("math", "add").unwrap().canonical_name,
            "math/add"
        );
    }
}
