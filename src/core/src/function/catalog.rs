#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
    sync::Arc,
    vec,
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
    sync::Arc,
    vec::Vec,
};

use crate::{
    FunctionInstance, FunctionInvocation, GuardFunctionSafety, InputKindScheme, KindScheme,
    MResult, MechError, MechErrorKind, MechFunction, MechFunctionFactory,
    OperationContractDeclaration, ResidentKernelFactory, ResidentKernelFactoryEntry,
    ResidentOperationKey, RuntimeFunctionContract, RuntimeFunctionSignature,
    RuntimeOutputAliasPolicy, SourceSchemeTemplate, SpecializationContext,
    SpecializationInvocation, SpecializedFunction, hash_str,
};

#[cfg(feature = "native-plan")]
use crate::NativeValueFeature;

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

/// An executable destination is a capability claim, not a consequence of an
/// operation having semantic meaning.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ExecutionTarget {
    DirectRuntime,
    ResidentCpu,
    Native,
    GpuBatch,
}

/// Compact target availability for one concrete runtime signature. Semantic
/// contract presence never adds a target implicitly.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct ExecutionTargetSet(u8);

impl ExecutionTargetSet {
    pub const NONE: Self = Self(0);
    pub const DIRECT_RUNTIME: Self = Self(1 << 0);
    pub const RESIDENT_CPU: Self = Self(1 << 1);
    pub const NATIVE: Self = Self(1 << 2);
    pub const GPU_BATCH: Self = Self(1 << 3);

    pub const fn contains(self, target: ExecutionTarget) -> bool {
        let bit = match target {
            ExecutionTarget::DirectRuntime => Self::DIRECT_RUNTIME.0,
            ExecutionTarget::ResidentCpu => Self::RESIDENT_CPU.0,
            ExecutionTarget::Native => Self::NATIVE.0,
            ExecutionTarget::GpuBatch => Self::GPU_BATCH.0,
        };
        self.0 & bit != 0
    }

    pub const fn with(self, target: ExecutionTarget) -> Self {
        let bit = match target {
            ExecutionTarget::DirectRuntime => Self::DIRECT_RUNTIME.0,
            ExecutionTarget::ResidentCpu => Self::RESIDENT_CPU.0,
            ExecutionTarget::Native => Self::NATIVE.0,
            ExecutionTarget::GpuBatch => Self::GPU_BATCH.0,
        };
        Self(self.0 | bit)
    }

    pub fn iter(self) -> impl Iterator<Item = ExecutionTarget> {
        [
            ExecutionTarget::DirectRuntime,
            ExecutionTarget::ResidentCpu,
            ExecutionTarget::Native,
            ExecutionTarget::GpuBatch,
        ]
        .into_iter()
        .filter(move |target| self.contains(*target))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeExecutionCapability {
    pub runtime_factory: RuntimeFunctionId,
    pub operation_binding: RuntimeOperationBinding,
    pub signature: RuntimeFunctionSignature,
    pub targets: ExecutionTargetSet,
}

/// Declares whether a concrete runtime factory owns one fixed semantic
/// identity or receives the identity selected by the source compiler. This is
/// explicit capability data; absence never implies target support.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeOperationBinding {
    CompilerResolved,
    Fixed(OperationId),
}

type RuntimeFunctionInvocationFactory = fn(FunctionInvocation) -> MResult<Box<dyn MechFunction>>;

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

/// Production source specialization over canonical cells and explicit source
/// control inputs.
pub trait CanonicalFunctionSpecializer: Send + Sync {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction>;

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::Unsupported
    }
}

pub struct RuntimeFunctionEntry {
    pub id: RuntimeFunctionId,
    pub name: String,
    invocation_factory: RuntimeFunctionInvocationFactory,
    signature: RuntimeFunctionSignature,
    contract: RuntimeFunctionContract,
    semantic_contract: Option<&'static OperationContractDeclaration>,
    operation_binding: RuntimeOperationBinding,
    execution_targets: ExecutionTargetSet,

    #[cfg(feature = "native-plan")]
    pub native_linkage: Option<NativeFunctionLinkage>,
}

impl Clone for RuntimeFunctionEntry {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            invocation_factory: self.invocation_factory,
            signature: self.signature,
            contract: self.contract,
            semantic_contract: self.semantic_contract,
            operation_binding: self.operation_binding,
            execution_targets: self.execution_targets,
            #[cfg(feature = "native-plan")]
            native_linkage: self.native_linkage.clone(),
        }
    }
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

    pub const fn execution_capability(&self) -> RuntimeExecutionCapability {
        RuntimeExecutionCapability {
            runtime_factory: self.id,
            operation_binding: self.operation_binding,
            signature: self.signature,
            targets: self.execution_targets,
        }
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

    pub fn validate_invocation(&self, invocation: &FunctionInvocation) -> MResult<()> {
        let invocation = invocation.clone().normalize_for_signature(self.signature);
        invocation
            .validate_signature(self.signature)
            .map_err(|error| self.wrap_contract_error(error))?;
        invocation
            .validate_contract(self.contract)
            .map_err(|error| self.wrap_contract_error(error))?;
        let function = (self.invocation_factory)(invocation)
            .map_err(|error| self.wrap_contract_error(error))?;
        drop(function);
        Ok(())
    }

    pub fn instantiate_invocation(
        &self,
        invocation: FunctionInvocation,
    ) -> MResult<Box<dyn MechFunction>> {
        self.bind_invocation(invocation)
            .map(FunctionInstance::into_implementation)
    }

    pub fn bind_invocation(&self, invocation: FunctionInvocation) -> MResult<FunctionInstance> {
        let invocation = invocation.normalize_for_signature(self.signature);
        invocation
            .validate_signature(self.signature)
            .map_err(|error| self.wrap_contract_error(error))?;
        invocation
            .validate_contract(self.contract)
            .map_err(|error| self.wrap_contract_error(error))?;
        let implementation = (self.invocation_factory)(invocation.clone())
            .map_err(|error| self.wrap_contract_error(error))?;
        Ok(FunctionInstance::new(implementation, invocation))
    }
}

#[derive(Clone)]
pub struct FunctionSpecializerEntry {
    pub operation: OperationId,
    pub canonical_name: String,
    pub type_authority: SourceTypeAuthority,
    pub specializer: Arc<dyn CanonicalFunctionSpecializer>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceInputKind {
    Value,
    Absent,
    MatrixAllSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionTypeOverload {
    pub id: u32,
    pub input_layout: Box<[SourceInputKind]>,
    pub scheme: KindScheme,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionTypeDeclaration {
    pub overloads: Box<[FunctionTypeOverload]>,
    pub template: Option<SourceSchemeTemplate>,
}

impl FunctionTypeDeclaration {
    pub fn from_schemes(schemes: Vec<KindScheme>) -> Self {
        let overloads = schemes
            .into_iter()
            .enumerate()
            .map(|(index, scheme)| {
                let value_count = match scheme.inputs() {
                    InputKindScheme::Fixed(inputs) => inputs.len(),
                    InputKindScheme::Variadic { prefix, .. } => prefix.len() + 1,
                };
                FunctionTypeOverload {
                    id: index as u32,
                    input_layout: vec![SourceInputKind::Value; value_count].into_boxed_slice(),
                    scheme,
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            overloads,
            template: None,
        }
    }

    pub fn from_template(template: SourceSchemeTemplate) -> Self {
        Self {
            overloads: Box::new([]),
            template: Some(template),
        }
    }

    pub fn with_layout(mut self, layout: Box<[SourceInputKind]>) -> Self {
        for overload in &mut self.overloads {
            overload.input_layout = layout.clone();
        }
        self
    }
}

pub fn maintained_source_type_declaration(
    canonical_name: &str,
) -> MResult<FunctionTypeDeclaration> {
    if let Some(template) = crate::type_system::maintained_source_scheme_template(canonical_name) {
        return Ok(FunctionTypeDeclaration::from_template(template));
    }
    let Some(schemes) = crate::type_system::maintained_source_schemes(canonical_name)? else {
        return Err(MechError::new(
            FunctionCatalogInvalidTypeDeclaration {
                canonical_name: canonical_name.into(),
                reason: "the maintained source operation has no explicit semantic scheme".into(),
            },
            None,
        )
        .with_compiler_loc());
    };
    let declaration = FunctionTypeDeclaration::from_schemes(schemes);
    if canonical_name.ends_with("/range-all") {
        Ok(declaration.with_layout(
            vec![
                SourceInputKind::Value,
                SourceInputKind::Value,
                SourceInputKind::Value,
                SourceInputKind::MatrixAllSelection,
            ]
            .into_boxed_slice(),
        ))
    } else {
        Ok(declaration)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceTypeAuthority {
    Schemes(FunctionTypeDeclaration),
    SyntaxDirectedIntrinsic,
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

    pub fn runtime_execution_capabilities(
        &self,
    ) -> impl ExactSizeIterator<Item = RuntimeExecutionCapability> + '_ {
        self.runtime_factories
            .values()
            .map(RuntimeFunctionEntry::execution_capability)
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionCatalogInvalidTypeDeclaration {
    pub canonical_name: String,
    pub reason: String,
}

impl MechErrorKind for FunctionCatalogInvalidTypeDeclaration {
    fn name(&self) -> &str {
        "FunctionCatalogInvalidTypeDeclaration"
    }

    fn message(&self) -> String {
        format!(
            "invalid semantic declaration for {:?}: {}",
            self.canonical_name, self.reason,
        )
    }
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

    pub fn contains_runtime_factory(&self, id: RuntimeFunctionId) -> bool {
        self.runtime_factories.contains_key(&id)
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
            invocation_factory: F::new_invocation,
            signature: F::SIGNATURE,
            contract,
            semantic_contract: None,
            operation_binding: RuntimeOperationBinding::CompilerResolved,
            execution_targets: ExecutionTargetSet::DIRECT_RUNTIME,
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
        semantic_operation: OperationId,
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
            invocation_factory: F::new_invocation,
            signature: F::SIGNATURE,
            contract,
            semantic_contract: Some(semantic_contract),
            operation_binding: RuntimeOperationBinding::Fixed(semantic_operation),
            execution_targets: ExecutionTargetSet::DIRECT_RUNTIME,
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
            invocation_factory: F::new_invocation,
            signature: F::SIGNATURE,
            contract,
            semantic_contract: None,
            operation_binding: RuntimeOperationBinding::CompilerResolved,
            execution_targets: ExecutionTargetSet::DIRECT_RUNTIME.with(ExecutionTarget::Native),
            native_linkage: Some(linkage),
        })
    }

    #[cfg(feature = "native-plan")]
    pub fn insert_runtime_factory_with_linkage_and_semantic_contract<F>(
        &mut self,
        name: impl Into<String>,
        contract: RuntimeFunctionContract,
        linkage: NativeFunctionLinkage,
        semantic_operation: OperationId,
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
            invocation_factory: F::new_invocation,
            signature: F::SIGNATURE,
            contract,
            semantic_contract: Some(semantic_contract),
            operation_binding: RuntimeOperationBinding::Fixed(semantic_operation),
            execution_targets: ExecutionTargetSet::DIRECT_RUNTIME.with(ExecutionTarget::Native),
            native_linkage: Some(linkage),
        })
    }

    pub fn insert_canonical_specializer(
        &mut self,
        canonical_name: impl Into<String>,
        type_declaration: FunctionTypeDeclaration,
        specializer: Arc<dyn CanonicalFunctionSpecializer>,
    ) -> MResult<OperationId> {
        let canonical_name = canonical_name.into();
        let operation = OperationId::from_name(&canonical_name);
        self.insert_specializer_entry(FunctionSpecializerEntry {
            operation,
            canonical_name,
            type_authority: SourceTypeAuthority::Schemes(type_declaration),
            specializer,
        })
    }

    pub fn insert_canonical_intrinsic_specializer(
        &mut self,
        canonical_name: impl Into<String>,
        specializer: Arc<dyn CanonicalFunctionSpecializer>,
    ) -> MResult<OperationId> {
        let canonical_name = canonical_name.into();
        let operation = OperationId::from_name(&canonical_name);
        self.insert_intrinsic_specializer_entry(FunctionSpecializerEntry {
            operation,
            canonical_name,
            type_authority: SourceTypeAuthority::SyntaxDirectedIntrinsic,
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

        let SourceTypeAuthority::Schemes(declaration) = &entry.type_authority else {
            return Err(invalid_type_declaration(
                &entry.canonical_name,
                "named source operations must be scheme-authoritative",
            ));
        };
        validate_type_declaration(&entry.canonical_name, declaration)?;

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

        if !matches!(
            entry.type_authority,
            SourceTypeAuthority::SyntaxDirectedIntrinsic
        ) {
            return Err(invalid_type_declaration(
                &entry.canonical_name,
                "parser-only intrinsics must be explicitly syntax-directed",
            ));
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

fn invalid_type_declaration(canonical_name: &str, reason: impl Into<String>) -> MechError {
    MechError::new(
        FunctionCatalogInvalidTypeDeclaration {
            canonical_name: canonical_name.into(),
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

fn validate_type_declaration(
    canonical_name: &str,
    declaration: &FunctionTypeDeclaration,
) -> MResult<()> {
    if declaration.overloads.is_empty() && declaration.template.is_none() {
        return Err(invalid_type_declaration(
            canonical_name,
            "at least one overload is required",
        ));
    }
    if declaration.template.is_some() && !declaration.overloads.is_empty() {
        return Err(invalid_type_declaration(
            canonical_name,
            "a declaration cannot combine fixed overloads with a source scheme template",
        ));
    }
    let mut ids = BTreeSet::new();
    for (index, overload) in declaration.overloads.iter().enumerate() {
        if !ids.insert(overload.id) {
            return Err(invalid_type_declaration(
                canonical_name,
                format!("overload ID {} is repeated", overload.id),
            ));
        }
        let value_slots = overload
            .input_layout
            .iter()
            .filter(|input| **input == SourceInputKind::Value)
            .count();
        let expected_values = match overload.scheme.inputs() {
            InputKindScheme::Fixed(inputs) => inputs.len(),
            InputKindScheme::Variadic { prefix, .. } => prefix.len() + 1,
        };
        if value_slots != expected_values {
            return Err(invalid_type_declaration(
                canonical_name,
                format!(
                    "overload {} has {value_slots} Value slots but its scheme has {expected_values} inputs",
                    overload.id,
                ),
            ));
        }
        if declaration.overloads[..index].iter().any(|existing| {
            existing.input_layout == overload.input_layout && existing.scheme == overload.scheme
        }) {
            return Err(invalid_type_declaration(
                canonical_name,
                format!(
                    "overload {} duplicates an earlier semantic declaration",
                    overload.id
                ),
            ));
        }
    }
    Ok(())
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
#[path = "tests/catalog.rs"]
mod tests;
