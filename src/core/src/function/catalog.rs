#[cfg(feature = "no_std")]
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec};

use crate::{
    FunctionArgs, FunctionValueRepresentation, GuardFunctionSafety, LegacyValue, MResult,
    MechError, MechErrorKind, MechFunction, MechFunctionFactory, NativeValueFeature,
    OperationContractDeclaration, ResidentKernelFactory, ResidentKernelFactoryEntry,
    ResidentOperationKey, RuntimeFunctionContract, RuntimeFunctionSignature,
    RuntimeOutputAliasPolicy, hash_str,
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

type RuntimeFunctionFactory = fn(FunctionArgs) -> MResult<Box<dyn MechFunction>>;

#[cfg(feature = "native-plan")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeFunctionLinkage {
    pub package: &'static str,
    pub crate_name: &'static str,
    pub installer_path: &'static str,
    pub cargo_features: Vec<&'static str>,
}

#[cfg(feature = "native-plan")]
impl NativeFunctionLinkage {
    #[doc(hidden)]
    pub fn for_factory<F: MechFunctionFactory>(
        package: &'static str,
        crate_name: &'static str,
        installer_path: &'static str,
        extra_cargo_features: &[&'static str],
    ) -> MResult<Self> {
        let mut cargo_features = F::SIGNATURE
            .required_native_features()
            .into_iter()
            .map(NativeValueFeature::cargo_feature)
            .collect::<Vec<_>>();
        for feature in extra_cargo_features {
            if NativeValueFeature::from_cargo_feature(feature).is_some() {
                return Err(MechError::new(
                    NativeRepresentationFeatureDeclaredAsExtra { feature: *feature },
                    None,
                )
                .with_compiler_loc());
            }
            cargo_features.push(feature);
        }
        cargo_features.extend(["native-link", "runtime"]);
        cargo_features.sort_unstable();
        cargo_features.dedup();
        Ok(Self {
            package,
            crate_name,
            installer_path,
            cargo_features,
        })
    }
}

pub trait FunctionSpecializer: Send + Sync {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>>;

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }
}

#[derive(Clone)]
pub struct RuntimeFunctionEntry {
    pub id: RuntimeFunctionId,
    pub name: String,
    factory: RuntimeFunctionFactory,
    signature: RuntimeFunctionSignature,
    contract: RuntimeFunctionContract,
    semantic_contract: Option<&'static OperationContractDeclaration>,

    #[cfg(feature = "native-plan")]
    pub native_linkage: Option<NativeFunctionLinkage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFunctionContractViolation {
    pub id: RuntimeFunctionId,
    pub name: String,
    pub reason: String,
}

impl MechErrorKind for RuntimeFunctionContractViolation {
    fn name(&self) -> &str {
        "RuntimeFunctionContractViolation"
    }

    fn message(&self) -> String {
        format!(
            "runtime function {} (0x{:016x}) rejected its argument contract: {}",
            self.name,
            self.id.raw(),
            self.reason,
        )
    }
}

impl RuntimeFunctionEntry {
    pub fn signature(&self) -> RuntimeFunctionSignature {
        self.signature
    }

    pub fn contract_kind(&self) -> &'static str {
        self.contract.kind
    }

    pub fn output_alias_policy(&self) -> RuntimeOutputAliasPolicy {
        self.contract.output_alias
    }

    pub const fn semantic_contract(&self) -> Option<&'static OperationContractDeclaration> {
        self.semantic_contract
    }

    fn wrap_contract_error(&self, error: MechError) -> MechError {
        MechError::new(
            RuntimeFunctionContractViolation {
                id: self.id,
                name: self.name.clone(),
                reason: error.simple_message(),
            },
            None,
        )
        .with_source(error)
        .with_compiler_loc()
    }

    pub fn validate_args(&self, args: &FunctionArgs) -> MResult<()> {
        args.validate_signature(self.signature)
            .map_err(|error| self.wrap_contract_error(error))?;
        args.validate_contract(self.contract)
            .map_err(|error| self.wrap_contract_error(error))?;
        let function =
            (self.factory)(args.clone()).map_err(|error| self.wrap_contract_error(error))?;
        drop(function);
        Ok(())
    }

    pub fn instantiate(&self, args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        args.validate_signature(self.signature)
            .map_err(|error| self.wrap_contract_error(error))?;
        args.validate_contract(self.contract)
            .map_err(|error| self.wrap_contract_error(error))?;
        (self.factory)(args).map_err(|error| self.wrap_contract_error(error))
    }
}

#[derive(Clone)]
pub struct FunctionSpecializerEntry {
    pub operation: OperationId,
    pub canonical_name: String,
    pub specializer: Arc<dyn FunctionSpecializer>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    resident_factories: BTreeMap<ResidentOperationKey, ResidentKernelFactoryEntry>,
    exports_by_module_item: BTreeMap<(String, String), FunctionExport>,
    exports_by_module: BTreeMap<String, Vec<FunctionExport>>,
    exports_by_operation: BTreeMap<OperationId, Vec<FunctionExport>>,
    all_exports: Vec<FunctionExport>,
}

impl FunctionCatalog {
    /// Constructs a catalog with no runtime factories, source specializers, or exports.
    pub fn empty() -> Self {
        Self {
            runtime_factories: BTreeMap::new(),
            specializers: BTreeMap::new(),
            intrinsic_specializers: BTreeMap::new(),
            resident_factories: BTreeMap::new(),
            exports_by_module_item: BTreeMap::new(),
            exports_by_module: BTreeMap::new(),
            exports_by_operation: BTreeMap::new(),
            all_exports: Vec::new(),
        }
    }

    pub fn runtime_entry(&self, id: RuntimeFunctionId) -> Option<&RuntimeFunctionEntry> {
        self.runtime_factories.get(&id)
    }

    pub fn runtime_entry_by_raw(&self, raw: u64) -> Option<&RuntimeFunctionEntry> {
        let function = RuntimeFunctionId::from_raw(raw);
        self.runtime_entry(function)
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

    /// Returns every named specializer in ascending [`OperationId`] order.
    /// Parser-only intrinsics are intentionally excluded.
    pub fn all_specializers(
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

    /// Returns the exports for one exact module in ascending module/item order.
    pub fn module_exports(
        &self,
        module: &str,
    ) -> impl ExactSizeIterator<Item = &FunctionExport> + '_ {
        self.exports_by_module
            .get(module)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
    }

    pub fn has_module(&self, module: &str) -> bool {
        self.module_exports(module).next().is_some()
    }

    /// Returns every export exactly once in deterministic order: first by
    /// [`OperationId`], then by module, item, exposure, and canonical name.
    pub fn all_exports(&self) -> impl ExactSizeIterator<Item = &FunctionExport> + '_ {
        self.all_exports.iter()
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

    pub fn resident_factory(
        &self,
        module_path: &[String],
        operation_name: &str,
    ) -> Option<&ResidentKernelFactoryEntry> {
        self.resident_factories.iter().find_map(|(key, entry)| {
            (key.module_path.as_ref() == module_path && key.operation_name == operation_name)
                .then_some(entry)
        })
    }

    pub fn resident_factory_count(&self) -> usize {
        self.resident_factories.len()
    }
}

impl Default for FunctionCatalog {
    fn default() -> Self {
        Self::empty()
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

#[cfg(feature = "native-plan")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogInvalidNativeLinkage {
    pub id: RuntimeFunctionId,
    pub name: String,
    pub linkage: NativeFunctionLinkage,
    pub reason: String,
}

#[cfg(feature = "native-plan")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeRepresentationFeatureDeclaredAsExtra {
    pub feature: &'static str,
}

#[cfg(feature = "native-plan")]
impl MechErrorKind for NativeRepresentationFeatureDeclaredAsExtra {
    fn name(&self) -> &str {
        "NativeRepresentationFeatureDeclaredAsExtra"
    }

    fn message(&self) -> String {
        format!(
            "native representation feature {:?} must be derived from the runtime signature",
            self.feature,
        )
    }
}

#[cfg(feature = "native-plan")]
impl MechErrorKind for FunctionCatalogInvalidNativeLinkage {
    fn name(&self) -> &str {
        "FunctionCatalogInvalidNativeLinkage"
    }

    fn message(&self) -> String {
        format!(
            "invalid native linkage for runtime factory {:?} at ID 0x{:016x}: {}",
            self.name,
            self.id.raw(),
            self.reason,
        )
    }
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

pub struct FunctionCatalogBuilder {
    runtime_factories: BTreeMap<RuntimeFunctionId, RuntimeFunctionEntry>,
    specializers: BTreeMap<OperationId, FunctionSpecializerEntry>,
    intrinsic_specializers: BTreeMap<OperationId, FunctionSpecializerEntry>,
    resident_factories: BTreeMap<ResidentOperationKey, ResidentKernelFactoryEntry>,
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
            resident_factories: BTreeMap::new(),
            exports_by_module_item: BTreeMap::new(),
            exports_by_operation: BTreeMap::new(),
        }
    }

    pub fn insert_runtime_factory<F>(
        &mut self,
        name: impl Into<String>,
        contract: RuntimeFunctionContract,
    ) -> MResult<()>
    where
        F: MechFunctionFactory,
    {
        let name = name.into();
        let id = RuntimeFunctionId::from_name(&name);
        self.insert_runtime_entry(RuntimeFunctionEntry {
            id,
            name,
            factory: F::new,
            signature: F::SIGNATURE,
            contract,
            semantic_contract: None,
            #[cfg(feature = "native-plan")]
            native_linkage: None,
        })
    }

    pub fn insert_resident_factory(
        &mut self,
        module_path: impl IntoIterator<Item = impl Into<String>>,
        operation_name: impl Into<String>,
        factory: ResidentKernelFactory,
    ) -> MResult<()> {
        let module_path = module_path
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let operation_name = operation_name.into();
        let Some(key) = ResidentOperationKey::new(module_path, operation_name) else {
            return Err(MechError::new(
                FunctionCatalogInvalidName {
                    category: "resident function",
                    name: String::new(),
                    id: 0,
                },
                None,
            )
            .with_compiler_loc());
        };
        if self.resident_factories.contains_key(&key) {
            return Err(MechError::new(
                FunctionCatalogInvalidName {
                    category: "duplicate resident function",
                    name: key.operation_name.clone(),
                    id: hash_str(&key.operation_name),
                },
                None,
            )
            .with_compiler_loc());
        }
        self.resident_factories
            .insert(key.clone(), ResidentKernelFactoryEntry { key, factory });
        Ok(())
    }

    pub fn insert_runtime_factory_with_semantic_contract<F>(
        &mut self,
        name: impl Into<String>,
        contract: RuntimeFunctionContract,
        semantic_contract: &'static OperationContractDeclaration,
    ) -> MResult<()>
    where
        F: MechFunctionFactory,
    {
        let name = name.into();
        let id = RuntimeFunctionId::from_name(&name);
        self.insert_runtime_entry(RuntimeFunctionEntry {
            id,
            name,
            factory: F::new,
            signature: F::SIGNATURE,
            contract,
            semantic_contract: Some(semantic_contract),
            #[cfg(feature = "native-plan")]
            native_linkage: None,
        })
    }

    #[cfg(feature = "native-plan")]
    pub fn insert_runtime_factory_with_linkage<F>(
        &mut self,
        name: impl Into<String>,
        contract: RuntimeFunctionContract,
        linkage: NativeFunctionLinkage,
    ) -> MResult<()>
    where
        F: MechFunctionFactory,
    {
        let name = name.into();
        let id = RuntimeFunctionId::from_name(&name);
        self.insert_runtime_entry(RuntimeFunctionEntry {
            id,
            name,
            factory: F::new,
            signature: F::SIGNATURE,
            contract,
            semantic_contract: None,
            native_linkage: Some(linkage),
        })
    }

    #[cfg(feature = "native-plan")]
    pub fn insert_runtime_factory_with_linkage_and_semantic_contract<F>(
        &mut self,
        name: impl Into<String>,
        contract: RuntimeFunctionContract,
        linkage: NativeFunctionLinkage,
        semantic_contract: &'static OperationContractDeclaration,
    ) -> MResult<()>
    where
        F: MechFunctionFactory,
    {
        let name = name.into();
        let id = RuntimeFunctionId::from_name(&name);
        self.insert_runtime_entry(RuntimeFunctionEntry {
            id,
            name,
            factory: F::new,
            signature: F::SIGNATURE,
            contract,
            semantic_contract: Some(semantic_contract),
            native_linkage: Some(linkage),
        })
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

    pub fn build(mut self) -> MResult<FunctionCatalog> {
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

        for exports in self.exports_by_operation.values_mut() {
            exports.sort_by(|left, right| {
                left.module
                    .cmp(&right.module)
                    .then_with(|| left.item.cmp(&right.item))
                    .then_with(|| left.exposure.cmp(&right.exposure))
                    .then_with(|| left.canonical_name.cmp(&right.canonical_name))
            });
        }

        let mut exports_by_module = BTreeMap::<String, Vec<FunctionExport>>::new();
        for ((module, _), export) in &self.exports_by_module_item {
            exports_by_module
                .entry(module.clone())
                .or_default()
                .push(export.clone());
        }
        let all_exports = self
            .exports_by_operation
            .values()
            .flatten()
            .cloned()
            .collect();

        let catalog = FunctionCatalog {
            runtime_factories: self.runtime_factories,
            specializers: self.specializers,
            intrinsic_specializers: self.intrinsic_specializers,
            resident_factories: self.resident_factories,
            exports_by_module_item: self.exports_by_module_item,
            exports_by_module,
            exports_by_operation: self.exports_by_operation,
            all_exports,
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

        #[cfg(feature = "native-plan")]
        if let Some(linkage) = &entry.native_linkage {
            validate_native_linkage(entry.id, &entry.name, linkage)?;
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

#[cfg(feature = "native-plan")]
fn validate_native_linkage(
    id: RuntimeFunctionId,
    name: &str,
    linkage: &NativeFunctionLinkage,
) -> MResult<()> {
    if !is_cargo_package_name(linkage.package) {
        return Err(invalid_native_linkage_error(
            id,
            name,
            linkage,
            format!("package {:?} must match [a-z][a-z0-9-]*", linkage.package,),
        ));
    }

    if !is_rust_identifier(linkage.crate_name) {
        return Err(invalid_native_linkage_error(
            id,
            name,
            linkage,
            format!(
                "crate name {:?} must match [A-Za-z_][A-Za-z0-9_]*",
                linkage.crate_name,
            ),
        ));
    }

    let installer_components = linkage.installer_path.split("::").collect::<Vec<_>>();
    if installer_components.len() < 2
        || installer_components
            .iter()
            .any(|component| !is_rust_identifier(component))
    {
        return Err(invalid_native_linkage_error(
            id,
            name,
            linkage,
            format!(
                "installer path {:?} must contain at least two `::`-separated identifiers",
                linkage.installer_path,
            ),
        ));
    }

    if linkage.cargo_features.is_empty() {
        return Err(invalid_native_linkage_error(
            id,
            name,
            linkage,
            String::from("Cargo features must not be empty"),
        ));
    }

    for feature in &linkage.cargo_features {
        if !is_cargo_feature_name(feature) {
            return Err(invalid_native_linkage_error(
                id,
                name,
                linkage,
                format!(
                    "Cargo feature {:?} must match [A-Za-z_][A-Za-z0-9_-]*",
                    feature,
                ),
            ));
        }
    }

    for features in linkage.cargo_features.windows(2) {
        if features[0] == features[1] {
            return Err(invalid_native_linkage_error(
                id,
                name,
                linkage,
                format!("Cargo feature {:?} is duplicated", features[0]),
            ));
        }
        if features[0] > features[1] {
            return Err(invalid_native_linkage_error(
                id,
                name,
                linkage,
                String::from("Cargo features must be sorted"),
            ));
        }
    }

    Ok(())
}

#[cfg(feature = "native-plan")]
fn invalid_native_linkage_error(
    id: RuntimeFunctionId,
    name: &str,
    linkage: &NativeFunctionLinkage,
    reason: String,
) -> MechError {
    MechError::new(
        FunctionCatalogInvalidNativeLinkage {
            id,
            name: String::from(name),
            linkage: linkage.clone(),
            reason,
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(feature = "native-plan")]
fn is_cargo_package_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(feature = "native-plan")]
fn is_rust_identifier(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(feature = "native-plan")]
fn is_cargo_feature_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Returns a lexicographically canonical feature set for declarative native
/// factory families. The public linkage validator deliberately still rejects
/// non-canonical input; this helper lets a declaration family state the union
/// of its feature sources without duplicating sort-order or de-duplication
/// logic at every call site.
#[doc(hidden)]
pub struct CanonicalNativeCargoFeatures<const N: usize> {
    values: [&'static str; N],
    len: usize,
}

impl<const N: usize> CanonicalNativeCargoFeatures<N> {
    /// Returns the sorted, duplicate-free Cargo feature set.
    #[doc(hidden)]
    pub fn as_slice(&'static self) -> &'static [&'static str] {
        &self.values[..self.len]
    }
}

#[doc(hidden)]
pub const fn canonical_native_cargo_features<const N: usize>(
    mut features: [&'static str; N],
) -> CanonicalNativeCargoFeatures<N> {
    let mut index = 1;
    while index < N {
        let mut cursor = index;
        while cursor > 0 && native_cargo_feature_compare(features[cursor], features[cursor - 1]) < 0
        {
            let previous = features[cursor - 1];
            features[cursor - 1] = features[cursor];
            features[cursor] = previous;
            cursor -= 1;
        }
        index += 1;
    }
    let mut len = 0;
    let mut source = 0;
    while source < N {
        if source == 0 || native_cargo_feature_compare(features[source - 1], features[source]) != 0
        {
            features[len] = features[source];
            len += 1;
        }
        source += 1;
    }

    CanonicalNativeCargoFeatures {
        values: features,
        len,
    }
}

const fn native_cargo_feature_compare(left: &str, right: &str) -> i8 {
    let left_bytes = left.as_bytes();
    let right_bytes = right.as_bytes();
    let mut index = 0;
    while index < left_bytes.len() && index < right_bytes.len() {
        if left_bytes[index] < right_bytes[index] {
            return -1;
        }
        if left_bytes[index] > right_bytes[index] {
            return 1;
        }
        index += 1;
    }
    if left_bytes.len() < right_bytes.len() {
        -1
    } else if left_bytes.len() > right_bytes.len() {
        1
    } else {
        0
    }
}

/// Declares one runtime factory together with its exact native-build linkage.
///
/// The private registration function is intended for the owner's aggregate
/// runtime installer. The public, documentation-hidden installer is the only
/// entry point generated native applications call.
#[macro_export]
macro_rules! declare_native_runtime_factory {
    (
        cfg: $cfg:meta,

        registration: $registration:ident,
        installer: $installer:ident,

        name: $name:expr,
        factory_type: $factory:ty,
        contract: $contract:expr,
        semantic_contract: $semantic_contract:expr,

        package: $package:literal,
        crate_name: $crate_name:literal,
        installer_path: $installer_path:expr,

        extra_cargo_features: [$($cargo_feature:expr),* $(,)?],
    ) => {
        #[cfg($cfg)]
        pub(crate) fn $registration(
            builder: &mut $crate::FunctionCatalogBuilder,
        ) -> $crate::MResult<()> {
            #[cfg(feature = "native-plan")]
            {
                return builder.insert_runtime_factory_with_linkage_and_semantic_contract::<$factory>(
                    $name,
                    $contract,
                    $crate::NativeFunctionLinkage::for_factory::<$factory>(
                        $package,
                        $crate_name,
                        $installer_path,
                        &[$($cargo_feature),*],
                    )?,
                    $semantic_contract,
                );
            }

            #[cfg(not(feature = "native-plan"))]
            {
                builder.insert_runtime_factory_with_semantic_contract::<$factory>(
                    $name,
                    $contract,
                    $semantic_contract,
                )
            }
        }

        #[doc(hidden)]
        #[cfg(feature = "native-link")]
        #[cfg($cfg)]
        pub fn $installer(
            builder: &mut $crate::FunctionCatalogBuilder,
        ) -> $crate::MResult<()> {
            builder.insert_runtime_factory_with_semantic_contract::<$factory>(
                $name,
                $contract,
                $semantic_contract,
            )
        }
    };

    (
        cfg: $cfg:meta,

        registration: $registration:ident,
        installer: $installer:ident,

        name: $name:expr,
        factory_type: $factory:ty,
        contract: $contract:expr,

        package: $package:literal,
        crate_name: $crate_name:literal,
        installer_path: $installer_path:expr,

        extra_cargo_features: [$($cargo_feature:expr),* $(,)?],
    ) => {
        #[cfg($cfg)]
        pub(crate) fn $registration(
            builder: &mut $crate::FunctionCatalogBuilder,
        ) -> $crate::MResult<()> {
            #[cfg(feature = "native-plan")]
            {
                return builder.insert_runtime_factory_with_linkage::<$factory>(
                    $name,
                    $contract,
                    $crate::NativeFunctionLinkage::for_factory::<$factory>(
                        $package,
                        $crate_name,
                        $installer_path,
                        &[$($cargo_feature),*],
                    )?,
                );
            }

            #[cfg(not(feature = "native-plan"))]
            {
                builder.insert_runtime_factory::<$factory>($name, $contract)
            }
        }

        #[doc(hidden)]
        #[cfg(feature = "native-link")]
        #[cfg($cfg)]
        pub fn $installer(
            builder: &mut $crate::FunctionCatalogBuilder,
        ) -> $crate::MResult<()> {
            builder.insert_runtime_factory::<$factory>($name, $contract)
        }
    };

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

    match export.exposure {
        FunctionExposure::Internal | FunctionExposure::Prelude => {
            if export.module.is_none() && export.item.is_none() {
                Ok(())
            } else {
                Err(invalid_export_error(
                    export,
                    String::from("internal and prelude exports cannot declare a module or item"),
                ))
            }
        }
        FunctionExposure::ModuleOnly => match (&export.module, &export.item) {
            (Some(module), Some(_)) if module.is_empty() => Err(invalid_export_error(
                export,
                String::from("module name must not be empty"),
            )),
            (Some(_), Some(item)) if item.is_empty() => Err(invalid_export_error(
                export,
                String::from("item name must not be empty"),
            )),
            (Some(_), Some(_)) => Ok(()),
            _ => Err(invalid_export_error(
                export,
                String::from("module-only exports must declare both a module and an item"),
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_runtime_contract() -> RuntimeFunctionContract {
        RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias)
    }

    struct TestFactory;

    impl crate::MechFunctionFactory for TestFactory {
        const SIGNATURE: RuntimeFunctionSignature =
            RuntimeFunctionSignature::nullary(FunctionValueRepresentation::Empty);

        fn new(_: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            unreachable!("catalog tests do not instantiate runtime functions")
        }
    }

    struct RejectingFactory;

    impl crate::MechFunctionFactory for RejectingFactory {
        const SIGNATURE: RuntimeFunctionSignature =
            RuntimeFunctionSignature::nullary(FunctionValueRepresentation::Empty);

        fn new(args: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            Err(MechError::new(
                crate::IncorrectNumberOfArguments {
                    expected: 2,
                    found: args.len(),
                },
                None,
            )
            .with_compiler_loc())
        }
    }

    struct MacroFactory;

    impl crate::MechFunctionFactory for MacroFactory {
        const SIGNATURE: RuntimeFunctionSignature =
            RuntimeFunctionSignature::nullary(FunctionValueRepresentation::F64);

        fn new(_: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
            unreachable!("catalog tests do not instantiate runtime functions")
        }
    }

    crate::declare_native_runtime_factory! {
        cfg: test,

        registration: register_macro_factory,
        installer: install_macro_factory,

        name: concat!("MacroFactory", "<f64>"),
        factory_type: MacroFactory,
        contract: RuntimeFunctionContract::no_matrix(
            RuntimeOutputAliasPolicy::DisallowInputAlias,
        ),

        package: "mech-core",
        crate_name: "mech_core",
        installer_path: concat!("mech_core::__mech_native::", "install_macro_factory"),

        extra_cargo_features: [],
    }

    struct TestSpecializer;

    impl FunctionSpecializer for TestSpecializer {
        fn specialize(&self, _: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
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
    fn empty_and_default_catalogs_have_no_function_surface() {
        for catalog in [FunctionCatalog::empty(), FunctionCatalog::default()] {
            assert_eq!(catalog.runtime_factory_count(), 0);
            assert_eq!(catalog.specializer_count(), 0);
            assert_eq!(catalog.intrinsic_specializer_count(), 0);
            assert_eq!(catalog.resident_factory_count(), 0);
            assert_eq!(catalog.all_exports().len(), 0);
        }
    }

    #[test]
    fn resident_factories_are_separate_and_resolve_by_semantic_operation_path() {
        fn binder(
            _request: &crate::ResidentKernelBindRequest<'_>,
        ) -> Result<crate::BoundResidentKernel, crate::ResidentKernelBindError> {
            Err(crate::ResidentKernelBindError::UnsupportedLayout)
        }

        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_resident_factory(["numeric", "matrix"], "add", binder)
            .unwrap();
        let catalog = builder.build().unwrap();

        assert_eq!(catalog.runtime_factory_count(), 0);
        assert_eq!(catalog.specializer_count(), 0);
        assert_eq!(catalog.resident_factory_count(), 1);
        let module = [String::from("numeric"), String::from("matrix")];
        assert_eq!(
            catalog
                .resident_factory(&module, "add")
                .expect("resident operation is independently addressable")
                .key
                .operation_name,
            "add"
        );
        assert!(catalog.resident_factory(&module, "subtract").is_none());
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
                factory: TestFactory::new,
                signature: TestFactory::SIGNATURE,
                contract: test_runtime_contract(),
                semantic_contract: None,
                #[cfg(feature = "native-plan")]
                native_linkage: None,
            })
            .unwrap();

        let error = builder
            .insert_runtime_entry(RuntimeFunctionEntry {
                id,
                name: String::from("second"),
                factory: TestFactory::new,
                signature: TestFactory::SIGNATURE,
                contract: test_runtime_contract(),
                semantic_contract: None,
                #[cfg(feature = "native-plan")]
                native_linkage: None,
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
            .insert_runtime_factory::<TestFactory>("AddSS<f64>", test_runtime_contract())
            .unwrap();
        let error = builder
            .insert_runtime_factory::<TestFactory>("AddSS<f64>", test_runtime_contract())
            .unwrap_err();
        assert_eq!(error.kind_name(), "FunctionCatalogDuplicateRuntimeFactory");
    }

    #[test]
    fn runtime_entry_wraps_factory_argument_failures_with_identity() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory::<RejectingFactory>("RejectingFactory", test_runtime_contract())
            .unwrap();
        let catalog = builder.build().unwrap();
        let entry = catalog
            .runtime_entry(RuntimeFunctionId::from_name("RejectingFactory"))
            .unwrap();

        let error = entry
            .instantiate(FunctionArgs::Nullary(LegacyValue::Empty))
            .err()
            .expect("rejecting factory must fail");
        assert_eq!(error.kind_name(), "RuntimeFunctionContractViolation");
        let violation = error.kind_as::<RuntimeFunctionContractViolation>().unwrap();
        assert_eq!(
            violation.id,
            RuntimeFunctionId::from_name("RejectingFactory")
        );
        assert_eq!(violation.name, "RejectingFactory");
        assert!(violation.reason.contains("IncorrectNumberOfArguments"));
    }

    #[test]
    fn declaration_macro_registers_exactly_one_factory_and_rejects_duplicates() {
        let mut builder = FunctionCatalogBuilder::new();
        register_macro_factory(&mut builder).unwrap();

        let error = register_macro_factory(&mut builder).unwrap_err();
        assert_eq!(error.kind_name(), "FunctionCatalogDuplicateRuntimeFactory");

        let catalog = builder.build().unwrap();
        assert_eq!(catalog.runtime_factory_count(), 1);
        let entry = catalog
            .runtime_entry(RuntimeFunctionId::from_name("MacroFactory<f64>"))
            .unwrap();
        assert_eq!(entry.name, "MacroFactory<f64>");

        #[cfg(feature = "native-plan")]
        assert_eq!(
            entry.native_linkage,
            Some(NativeFunctionLinkage {
                package: "mech-core",
                crate_name: "mech_core",
                installer_path: "mech_core::__mech_native::install_macro_factory",
                cargo_features: vec!["f64", "native-link", "runtime"],
            }),
        );
    }

    #[cfg(feature = "native-plan")]
    #[test]
    fn native_linkage_rejects_representation_features_as_extras() {
        let error = NativeFunctionLinkage::for_factory::<MacroFactory>(
            "mech-core",
            "mech_core",
            "mech_core::__mech_native::install_macro_factory",
            &["f64"],
        )
        .unwrap_err();

        assert_eq!(
            error.kind_name(),
            "NativeRepresentationFeatureDeclaredAsExtra"
        );
    }

    #[cfg(feature = "native-plan")]
    #[test]
    fn plain_runtime_registration_keeps_native_linkage_absent() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory::<TestFactory>("PlainFactory", test_runtime_contract())
            .unwrap();

        let catalog = builder.build().unwrap();
        assert_eq!(
            catalog
                .runtime_entry(RuntimeFunctionId::from_name("PlainFactory"))
                .unwrap()
                .native_linkage,
            None,
        );
    }

    #[cfg(feature = "native-plan")]
    #[test]
    fn linked_runtime_registration_preserves_trusted_metadata_exactly() {
        let linkage = NativeFunctionLinkage {
            package: "mech-math",
            crate_name: "Mech_Math",
            installer_path: "Mech_Math::__mech_native::install_Add_2",
            cargo_features: vec!["F64", "add-2", "native_link", "runtime"],
        };
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_runtime_factory_with_linkage::<TestFactory>(
                "AddSS<f64>",
                test_runtime_contract(),
                linkage.clone(),
            )
            .unwrap();

        let catalog = builder.build().unwrap();
        assert_eq!(
            catalog
                .runtime_entry(RuntimeFunctionId::from_name("AddSS<f64>"))
                .unwrap()
                .native_linkage
                .as_ref(),
            Some(&linkage),
        );
    }

    #[cfg(feature = "native-plan")]
    #[test]
    fn native_linkage_validation_accepts_each_frozen_name_grammar() {
        for linkage in [
            NativeFunctionLinkage {
                package: "m",
                crate_name: "_",
                installer_path: "_::i",
                cargo_features: vec!["_"],
            },
            NativeFunctionLinkage {
                package: "mech-2-native-",
                crate_name: "Mech_2",
                installer_path: "Mech_2::__mech_native::install_2",
                cargo_features: vec!["A-1", "_f64", "native-link"],
            },
        ] {
            let mut builder = FunctionCatalogBuilder::new();
            builder
                .insert_runtime_factory_with_linkage::<TestFactory>(
                    "ValidFactory",
                    test_runtime_contract(),
                    linkage,
                )
                .unwrap();
        }
    }

    #[cfg(feature = "native-plan")]
    #[test]
    fn native_linkage_validation_rejects_every_malformed_field() {
        let invalid = [
            (
                NativeFunctionLinkage {
                    package: "",
                    crate_name: "mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["runtime"],
                },
                "package",
            ),
            (
                NativeFunctionLinkage {
                    package: "Mech-core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["runtime"],
                },
                "package",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech_core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["runtime"],
                },
                "package",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "2mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["runtime"],
                },
                "crate name",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech-core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["runtime"],
                },
                "crate name",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech_core",
                    installer_path: "install",
                    cargo_features: vec!["runtime"],
                },
                "installer path",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::::install",
                    cargo_features: vec!["runtime"],
                },
                "installer path",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::2install",
                    cargo_features: vec!["runtime"],
                },
                "installer path",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec![],
                },
                "must not be empty",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["2runtime"],
                },
                "Cargo feature",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["native/link"],
                },
                "Cargo feature",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["runtime", "f64"],
                },
                "sorted",
            ),
            (
                NativeFunctionLinkage {
                    package: "mech-core",
                    crate_name: "mech_core",
                    installer_path: "mech_core::install",
                    cargo_features: vec!["runtime", "runtime"],
                },
                "duplicated",
            ),
        ];

        for (linkage, expected_reason) in invalid {
            let mut builder = FunctionCatalogBuilder::new();
            let error = builder
                .insert_runtime_factory_with_linkage::<TestFactory>(
                    "InvalidFactory",
                    test_runtime_contract(),
                    linkage,
                )
                .unwrap_err();
            assert_eq!(error.kind_name(), "FunctionCatalogInvalidNativeLinkage");
            assert!(
                error.kind_message().contains(expected_reason),
                "expected {:?} in {:?}",
                expected_reason,
                error.kind_message(),
            );
        }
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
            export("math/add", None, None, FunctionExposure::ModuleOnly),
            export(
                "math/add",
                Some("math"),
                Some("add"),
                FunctionExposure::Internal,
            ),
            export(
                "math/add",
                Some("math"),
                Some("add"),
                FunctionExposure::Prelude,
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
            .insert_runtime_factory::<TestFactory>("AddSS<f64>", test_runtime_contract())
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
        assert!(catalog.runtime_entry(runtime_id).is_some());
        assert_eq!(
            catalog.runtime_entry(runtime_id).unwrap().name,
            "AddSS<f64>"
        );
        assert!(std::ptr::eq(
            catalog.runtime_entry_by_raw(runtime_id.raw()).unwrap(),
            catalog.runtime_entry(runtime_id).unwrap(),
        ));
        assert!(catalog.runtime_entry_by_raw(u64::MAX).is_none());
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

    #[test]
    fn module_queries_are_exact_and_sorted_by_item() {
        let mut builder = FunctionCatalogBuilder::new();
        for name in ["math/zeta", "math/alpha", "math-extra/only"] {
            builder
                .insert_specializer(name, test_specializer())
                .unwrap();
        }
        for (name, module, item) in [
            ("math/zeta", "math", "zeta"),
            ("math-extra/only", "math-extra", "only"),
            ("math/alpha", "math", "alpha"),
        ] {
            builder
                .insert_export(export(
                    name,
                    Some(module),
                    Some(item),
                    FunctionExposure::ModuleOnly,
                ))
                .unwrap();
        }

        let catalog = builder.build().unwrap();
        let items = catalog
            .module_exports("math")
            .map(|export| export.item.as_deref().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(items, ["alpha", "zeta"]);
        assert!(catalog.has_module("math"));
        assert!(catalog.has_module("math-extra"));
        assert!(!catalog.has_module("mat"));
        assert!(!catalog.has_module("missing"));
    }

    #[test]
    fn all_specializers_are_named_only_and_sorted_by_operation_id() {
        let mut builder = FunctionCatalogBuilder::new();
        builder
            .insert_specializer("stats/mean", test_specializer())
            .unwrap();
        builder
            .insert_specializer("math/add", test_specializer())
            .unwrap();
        builder
            .insert_specializer("logic/and", test_specializer())
            .unwrap();
        builder
            .insert_intrinsic_specializer("assign", test_specializer())
            .unwrap();

        let catalog = builder.build().unwrap();
        let specializers = catalog.all_specializers().collect::<Vec<_>>();
        let operation_ids = specializers
            .iter()
            .map(|entry| entry.operation)
            .collect::<Vec<_>>();

        assert_eq!(specializers.len(), 3);
        assert!(operation_ids.is_sorted());
        assert!(
            specializers
                .iter()
                .all(|entry| entry.canonical_name != "assign")
        );
    }

    #[test]
    fn all_exports_are_complete_unique_and_independent_of_insertion_order() {
        fn build_catalog(reverse_exports: bool) -> FunctionCatalog {
            let mut builder = FunctionCatalogBuilder::new();
            for name in ["stats/mean", "math/add"] {
                builder
                    .insert_specializer(name, test_specializer())
                    .unwrap();
            }

            let mut exports = vec![
                export("math/add", None, None, FunctionExposure::Prelude),
                export(
                    "stats/mean",
                    Some("stats"),
                    Some("mean"),
                    FunctionExposure::ModuleOnly,
                ),
                export("math/add", None, None, FunctionExposure::Internal),
                export(
                    "math/add",
                    Some("math"),
                    Some("add"),
                    FunctionExposure::ModuleOnly,
                ),
            ];
            if reverse_exports {
                exports.reverse();
            }
            for export in exports {
                builder.insert_export(export).unwrap();
            }
            builder.build().unwrap()
        }

        fn signatures(
            catalog: &FunctionCatalog,
        ) -> Vec<(
            OperationId,
            Option<String>,
            Option<String>,
            FunctionExposure,
            String,
        )> {
            catalog
                .all_exports()
                .map(|export| {
                    (
                        export.operation,
                        export.module.clone(),
                        export.item.clone(),
                        export.exposure,
                        export.canonical_name.clone(),
                    )
                })
                .collect()
        }

        let forward = build_catalog(false);
        let reverse = build_catalog(true);
        let forward_signatures = signatures(&forward);

        assert_eq!(forward_signatures, signatures(&reverse));
        assert_eq!(forward_signatures.len(), 4);
        assert!(forward_signatures.windows(2).all(|pair| pair[0] != pair[1]));
        assert_eq!(
            forward
                .exports_for_operation(OperationId::from_name("math/add"))
                .iter()
                .map(|export| export.exposure)
                .collect::<Vec<_>>(),
            [
                FunctionExposure::Internal,
                FunctionExposure::Prelude,
                FunctionExposure::ModuleOnly,
            ],
        );
    }
}
