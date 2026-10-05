#[cfg(feature = "dynamic-modules")]
pub use super::dynamic::DynamicModuleLoader;
#[cfg(feature = "dynamic-modules")]
use super::dynamic::{
    ValidatedDynamicKernelKind, check_dynamic_kernel_status, dynamic_null_binary_f64_f64_to_f64,
    dynamic_trace, mech_str_to_string,
};
use crate::*;
#[cfg(any(test, feature = "dynamic-modules"))]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "dynamic-modules")]
use std::collections::{HashMap, HashSet};
#[cfg(all(test, not(feature = "dynamic-modules")))]
use std::sync::Arc;
#[cfg(feature = "dynamic-modules")]
use std::sync::{Arc, LazyLock};

#[cfg(feature = "dynamic-modules")]
static DYNAMIC_UNARY_SCALAR_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| dynamic_unary_contract(ChangeDetectionPolicy::ExactScalar));
#[cfg(feature = "dynamic-modules")]
static DYNAMIC_UNARY_VIEW_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| dynamic_unary_contract(ChangeDetectionPolicy::KernelReported));
#[cfg(feature = "dynamic-modules")]
static DYNAMIC_BINARY_SCALAR_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| dynamic_binary_contract(ChangeDetectionPolicy::ExactScalar));
#[cfg(feature = "dynamic-modules")]
static DYNAMIC_BINARY_BROADCAST_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| dynamic_binary_contract(ChangeDetectionPolicy::KernelReported));

#[cfg(feature = "dynamic-modules")]
fn dynamic_unary_contract(change_detection: ChangeDetectionPolicy) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::SameAsInput { input: 0 },
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_binary_contract(
    change_detection: ChangeDetectionPolicy,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                },
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

#[derive(Clone, Debug)]
pub struct ModuleManifest {
    pub module: String,
    pub items: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DynamicFunctionExport {
    pub item: String,
    pub extension: ExtensionFunctionId,
}

#[derive(Clone)]
pub struct DynamicFunctionModuleFragment {
    pub module: String,
    pub entries: Vec<FunctionExtensionEntry>,
    pub exports: Vec<DynamicFunctionExport>,
    #[cfg(feature = "dynamic-modules")]
    source_declarations: BTreeMap<ExtensionFunctionId, FunctionTypeDeclaration>,
    #[cfg(feature = "dynamic-modules")]
    source_contracts: BTreeMap<ExtensionFunctionId, Box<[OperationContractDeclaration]>>,
}

impl DynamicFunctionModuleFragment {
    #[cfg(any(test, feature = "dynamic-modules"))]
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            module: self.module.clone(),
            items: self
                .exports
                .iter()
                .map(|export| export.item.clone())
                .collect(),
        }
    }
}

pub trait ModuleLoader {
    fn can_load(&self, module: &str) -> bool;
    fn load(&self, module: &str) -> MResult<DynamicFunctionModuleFragment>;
}

#[cfg(feature = "dynamic-modules")]
impl ModuleLoader for DynamicModuleLoader {
    fn can_load(&self, module: &str) -> bool {
        Self::find_library(module).is_some()
    }

    fn load(&self, module: &str) -> MResult<DynamicFunctionModuleFragment> {
        let path = Self::find_library(module).ok_or_else(|| {
            MechError::new(MissingFunctionError::named(module), None).with_compiler_loc()
        })?;

        dynamic_trace(format!(
            "loading dynamic module `{}` from {}",
            module,
            path.display()
        ));

        let library = unsafe { libloading::Library::new(&path) }.map_err(|err| {
            Self::dynamic_error(format!(
                "failed to open dynamic module `{}` at {}: {err}",
                module,
                path.display()
            ))
        })?;

        let (abi_version, module_name_fn, export_count_fn, get_export_fn) = unsafe {
            let abi_version = *library
                .get::<mech_abi::MechModuleAbiVersionFnV1>(b"mech_module_abi_version_v1\0")
                .map_err(|err| {
                    Self::dynamic_error(format!("missing mech_module_abi_version_v1: {err}"))
                })?;
            let module_name_fn = *library
                .get::<mech_abi::MechModuleNameFnV1>(b"mech_module_name_v1\0")
                .map_err(|err| {
                    Self::dynamic_error(format!("missing mech_module_name_v1: {err}"))
                })?;
            let export_count_fn = *library
                .get::<mech_abi::MechModuleExportCountFnV1>(b"mech_module_export_count_v1\0")
                .map_err(|err| {
                    Self::dynamic_error(format!("missing mech_module_export_count_v1: {err}"))
                })?;
            let get_export_fn = *library
                .get::<mech_abi::MechModuleGetExportFnV1>(b"mech_module_get_export_v1\0")
                .map_err(|err| {
                    Self::dynamic_error(format!("missing mech_module_get_export_v1: {err}"))
                })?;
            (abi_version, module_name_fn, export_count_fn, get_export_fn)
        };

        let version = unsafe { abi_version() };
        if version != mech_abi::MECH_MODULE_ABI_VERSION_V1 {
            return Err(Self::dynamic_error(format!(
                "unsupported dynamic module ABI version {version}; expected {}",
                mech_abi::MECH_MODULE_ABI_VERSION_V1
            )));
        }

        let mut module_name = mech_abi::MechStrV1 {
            ptr: std::ptr::null(),
            len: 0,
        };
        Self::call_status(
            unsafe { module_name_fn(&mut module_name) },
            "mech_module_name_v1",
        )?;
        let module_name = unsafe { mech_str_to_string(module_name) }?;
        if module_name != module {
            return Err(Self::dynamic_error(format!(
                "dynamic module name `{module_name}` did not match requested module `{module}`"
            )));
        }

        let library = Arc::new(library);
        let module_prefix = format!("{module}/");
        let export_count = unsafe { export_count_fn() };
        if export_count == 0 {
            return Err(Self::dynamic_error(format!(
                "dynamic module `{module}` exported no functions"
            )));
        }

        dynamic_trace(format!(
            "dynamic module `{module}` exports {export_count} function(s)"
        ));

        let mut seen_exports = HashSet::<(String, mech_abi::MechKernelKindV1)>::new();
        let mut canonical_order = Vec::<String>::new();
        let mut dynamic_specializers =
            HashMap::<String, Vec<Arc<dyn CanonicalFunctionSpecializer>>>::new();
        let mut dynamic_kinds = HashMap::<String, BTreeSet<ValidatedDynamicKernelKind>>::new();

        for index in 0..export_count {
            let mut export = mech_abi::MechExportV1 {
                name: mech_abi::MechStrV1 {
                    ptr: std::ptr::null(),
                    len: 0,
                },
                kind: mech_abi::MechKernelKindV1::BINARY_F64_F64_TO_F64,
                function: mech_abi::MechKernelFnV1 {
                    binary_f64_f64_to_f64: dynamic_null_binary_f64_f64_to_f64,
                },
            };
            Self::call_status(
                unsafe { get_export_fn(index, &mut export) },
                format!("mech_module_get_export_v1({index})"),
            )?;

            let export_name = unsafe { mech_str_to_string(export.name) }?;
            if !seen_exports.insert((export_name.clone(), export.kind)) {
                return Err(Self::dynamic_error(format!(
                    "dynamic module `{module}` exported duplicate function `{export_name}` with kind {:?}",
                    export.kind
                )));
            }

            let Some(item) = export_name.strip_prefix(&module_prefix) else {
                return Err(Self::dynamic_error(format!(
                    "dynamic module `{module}` exported `{export_name}`, which is outside `{module}/`"
                )));
            };

            if item.is_empty() {
                return Err(Self::dynamic_error(format!(
                    "dynamic module `{module}` exported an empty item name via `{export_name}`"
                )));
            }

            let item = item.to_string();

            let kernel_kind = Self::validate_dynamic_kernel_kind(export.kind)?;
            dynamic_kinds
                .entry(export_name.clone())
                .or_default()
                .insert(kernel_kind);
            match kernel_kind {
                ValidatedDynamicKernelKind::BinaryF64F64ToF64 => {
                    let kernel = unsafe { export.function.binary_f64_f64_to_f64 };
                    let specializer_name = export_name.clone();

                    if !dynamic_specializers.contains_key(&specializer_name) {
                        canonical_order.push(specializer_name.clone());
                    }
                    dynamic_specializers
                        .entry(specializer_name.clone())
                        .or_default()
                        .push(Arc::new(DynamicBinaryF64F64ToF64Specializer {
                            name: specializer_name.clone(),
                            kernel,
                            _library: library.clone(),
                        }));

                    dynamic_trace(format!(
                        "registered dynamic export `{}` as item `{}`",
                        specializer_name, item
                    ));
                }
                ValidatedDynamicKernelKind::UnaryF64ToF64 => {
                    let kernel = unsafe { export.function.unary_f64_to_f64 };
                    let specializer_name = export_name.clone();

                    if !dynamic_specializers.contains_key(&specializer_name) {
                        canonical_order.push(specializer_name.clone());
                    }
                    dynamic_specializers
                        .entry(specializer_name.clone())
                        .or_default()
                        .push(Arc::new(DynamicUnaryF64ToF64Specializer {
                            name: specializer_name.clone(),
                            kernel,
                            _library: library.clone(),
                        }));

                    dynamic_trace(format!(
                        "registered dynamic export `{}` as item `{}`",
                        specializer_name, item
                    ));
                }
                ValidatedDynamicKernelKind::UnaryF64ViewToF64View => {
                    let kernel = unsafe { export.function.unary_f64_view_to_f64_view };
                    let specializer_name = export_name.clone();

                    if !dynamic_specializers.contains_key(&specializer_name) {
                        canonical_order.push(specializer_name.clone());
                    }
                    dynamic_specializers
                        .entry(specializer_name.clone())
                        .or_default()
                        .push(Arc::new(DynamicUnaryF64ViewToF64ViewSpecializer {
                            name: specializer_name.clone(),
                            kernel,
                            _library: library.clone(),
                        }));

                    dynamic_trace(format!(
                        "registered dynamic export `{}` as item `{}`",
                        specializer_name, item
                    ));
                }
            }
        }

        let mut entries = Vec::with_capacity(canonical_order.len());
        let mut exports = Vec::with_capacity(canonical_order.len());
        let mut source_declarations = BTreeMap::new();
        let mut source_contracts = BTreeMap::new();
        for canonical_name in canonical_order {
            let mut specializers = dynamic_specializers
                .remove(&canonical_name)
                .expect("every ordered dynamic function has specializers");
            let specializer: Arc<dyn CanonicalFunctionSpecializer> = if specializers.len() == 1 {
                specializers.pop().expect("one dynamic specializer")
            } else {
                Arc::new(DynamicOverloadedSpecializer {
                    name: canonical_name.clone(),
                    specializers,
                })
            };
            let entry = FunctionExtensionEntry::new(canonical_name.clone(), specializer);
            let item = canonical_name
                .strip_prefix(&module_prefix)
                .expect("validated dynamic export belongs to its module")
                .to_string();
            exports.push(DynamicFunctionExport {
                item,
                extension: entry.id,
            });
            let kinds = dynamic_kinds
                .remove(&canonical_name)
                .expect("every ordered dynamic function has ABI kinds");
            let mut schemes = Vec::new();
            if kinds.contains(&ValidatedDynamicKernelKind::UnaryF64ToF64)
                || kinds.contains(&ValidatedDynamicKernelKind::UnaryF64ViewToF64View)
            {
                schemes.extend(
                    mech_core::type_system::predicate_unary_same(
                        mech_core::BuiltinKindPredicate::FloatingPoint,
                    )
                    .map_err(|error| MechError::new(error, None).with_compiler_loc())?,
                );
            }
            if kinds.contains(&ValidatedDynamicKernelKind::BinaryF64F64ToF64) {
                schemes.extend(
                    mech_core::type_system::promoted_binary_elementwise()
                        .map_err(|error| MechError::new(error, None).with_compiler_loc())?,
                );
            }
            source_declarations.insert(entry.id, FunctionTypeDeclaration::from_schemes(schemes));
            let mut contracts = Vec::new();
            if kinds.contains(&ValidatedDynamicKernelKind::UnaryF64ToF64) {
                contracts.push(DYNAMIC_UNARY_SCALAR_CONTRACT.clone());
            }
            if kinds.contains(&ValidatedDynamicKernelKind::UnaryF64ViewToF64View) {
                contracts.push(DYNAMIC_UNARY_VIEW_CONTRACT.clone());
            }
            if kinds.contains(&ValidatedDynamicKernelKind::BinaryF64F64ToF64) {
                contracts.push(DYNAMIC_BINARY_SCALAR_CONTRACT.clone());
                contracts.push(DYNAMIC_BINARY_BROADCAST_CONTRACT.clone());
            }
            source_contracts.insert(entry.id, contracts.into_boxed_slice());
            entries.push(entry);
        }

        Ok(DynamicFunctionModuleFragment {
            module: module.to_string(),
            entries,
            exports,
            source_declarations,
            source_contracts,
        })
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicOverloadedSpecializer {
    name: String,
    specializers: Vec<Arc<dyn CanonicalFunctionSpecializer>>,
}

#[cfg(feature = "dynamic-modules")]
impl CanonicalFunctionSpecializer for DynamicOverloadedSpecializer {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        let mut last_error = None;

        for specializer in &self.specializers {
            match specializer.specialize_invocation(invocation, context) {
                Ok(function) => return Ok(function),
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            MechError::new(
                GenericError {
                    msg: format!("no dynamic overload matched for `{}`", self.name),
                },
                None,
            )
            .with_compiler_loc()
        }))
    }
}

#[cfg(feature = "dynamic-modules")]
#[derive(Clone)]
enum DynamicF64Arg {
    Scalar(ValueCell),
    Matrix(ValueCell),
}

#[cfg(feature = "dynamic-modules")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DynamicF64BinaryBroadcastKind {
    MatrixScalar,
    ScalarMatrix,
    MatrixMatrix,
}

#[cfg(feature = "dynamic-modules")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DynamicF64BinaryBroadcastPlan {
    kind: DynamicF64BinaryBroadcastKind,
    rows: usize,
    cols: usize,
    len: usize,
}

#[cfg(feature = "dynamic-modules")]
impl DynamicF64BinaryBroadcastPlan {
    fn new(
        kind: DynamicF64BinaryBroadcastKind,
        rows: usize,
        cols: usize,
        fxn_name: &str,
    ) -> MResult<Self> {
        let Some(len) = rows.checked_mul(cols) else {
            return Err(MechError::new(
                GenericError {
                    msg: format!(
                        "dynamic function `{}` broadcast shape overflowed: {} x {}",
                        fxn_name, rows, cols
                    ),
                },
                None,
            )
            .with_compiler_loc());
        };

        Ok(Self {
            kind,
            rows,
            cols,
            len,
        })
    }
}

#[cfg(feature = "dynamic-modules")]
impl DynamicF64Arg {
    fn matrix_shape(&self) -> Option<(usize, usize)> {
        match self {
            DynamicF64Arg::Scalar(value) => {
                // Retain and validate the semantic scalar while deriving the
                // broadcast category; execution resolves its physical lane
                // from the managed call frame.
                let _scalar = canonical_f64(value).ok()?;
                None
            }
            DynamicF64Arg::Matrix(matrix) => canonical_f64_matrix_values(matrix)
                .ok()
                .map(|(rows, columns, _)| (rows, columns)),
        }
    }

    #[cfg(test)]
    fn value_at(&self, index: usize) -> MResult<f64> {
        match self {
            DynamicF64Arg::Scalar(value) => canonical_f64(value),
            DynamicF64Arg::Matrix(matrix) => canonical_f64_matrix_values(matrix)?
                .2
                .get(index - 1)
                .copied()
                .ok_or_else(|| {
                    MechError::new(
                        GenericError {
                            msg: "dynamic matrix argument index is out of bounds".to_owned(),
                        },
                        None,
                    )
                    .with_compiler_loc()
                }),
        }
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicBinaryF64F64ToF64Specializer {
    name: String,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl CanonicalFunctionSpecializer for DynamicBinaryF64F64ToF64Specializer {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let lhs_cell = invocation.input(0).expect("validated lhs").cell()?.clone();
        let rhs_cell = invocation.input(1).expect("validated rhs").cell()?.clone();
        let lhs = dynamic_arg_as_f64_scalar_or_matrix(lhs_cell.clone(), &self.name)?;
        let rhs = dynamic_arg_as_f64_scalar_or_matrix(rhs_cell.clone(), &self.name)?;
        let scalar = matches!(
            (&lhs, &rhs),
            (DynamicF64Arg::Scalar(_), DynamicF64Arg::Scalar(_))
        );

        let (implementation, output): (Box<dyn MechFunction>, ValueCell) = match (&lhs, &rhs) {
            (DynamicF64Arg::Scalar(_), DynamicF64Arg::Scalar(_)) => {
                let output = ValueCell::from_exact(0.0_f64)?;
                let runtime_invocation =
                    FunctionInvocation::binary(output.clone(), lhs_cell.clone(), rhs_cell.clone());
                let (out, n, k) = runtime_invocation.expect_binary()?;
                (
                    Box::new(DynamicBinaryF64F64ToF64Function {
                        name: self.name.clone(),
                        n: n.try_managed_element::<f64>()?,
                        k: k.try_managed_element::<f64>()?,
                        output: out.try_managed_element::<f64>()?,
                        kernel: self.kernel,
                        _library: self._library.clone(),
                    }),
                    output,
                )
            }

            _ => {
                let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, &self.name)?;
                let output = dynamic_f64_matrix_output(plan.rows, plan.cols)?;
                let runtime_invocation =
                    FunctionInvocation::binary(output.clone(), lhs_cell.clone(), rhs_cell.clone());
                let (out, lhs, rhs) = runtime_invocation.expect_binary()?;

                (
                    Box::new(DynamicBinaryF64F64BroadcastFunction {
                        name: self.name.clone(),
                        lhs: lhs.try_managed_element::<f64>()?,
                        rhs: rhs.try_managed_element::<f64>()?,
                        output: out.try_managed_element::<f64>()?,
                        kernel: self.kernel,
                        _library: self._library.clone(),
                    }),
                    output,
                )
            }
        };
        let contract = if scalar {
            &*DYNAMIC_BINARY_SCALAR_CONTRACT
        } else {
            &*DYNAMIC_BINARY_BROADCAST_CONTRACT
        };
        context.resolve_syntax_operation_contract(contract)?;
        context.certify_instance(
            (
                implementation,
                FunctionInvocation::binary(output, lhs_cell, rhs_cell),
            ),
            mech_core::RuntimeFunctionId::from_name(&self.name),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::NoAdditionalScratch,
        )
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ToF64Specializer {
    name: String,
    kernel: mech_abi::MechUnaryF64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl CanonicalFunctionSpecializer for DynamicUnaryF64ToF64Specializer {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let input = invocation
            .input(0)
            .expect("validated input")
            .cell()?
            .clone();
        dynamic_arg_as_f64_ref(&input, &self.name)?;
        let output = ValueCell::from_exact(0.0_f64)?;
        let runtime_invocation = FunctionInvocation::unary(output.clone(), input.clone());
        let (out, managed_input) = runtime_invocation.expect_unary()?;

        context.resolve_syntax_operation_contract(&DYNAMIC_UNARY_SCALAR_CONTRACT)?;
        context.certify_instance(
            (
                Box::new(DynamicUnaryF64ToF64Function {
                    name: self.name.clone(),
                    input: managed_input.try_managed_element::<f64>()?,
                    output: out.try_managed_element::<f64>()?,
                    kernel: self.kernel,
                    _library: self._library.clone(),
                }),
                runtime_invocation,
            ),
            mech_core::RuntimeFunctionId::from_name(&self.name),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::NoAdditionalScratch,
        )
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ViewToF64ViewSpecializer {
    name: String,
    kernel: mech_abi::MechUnaryF64ViewToF64ViewKernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl CanonicalFunctionSpecializer for DynamicUnaryF64ViewToF64ViewSpecializer {
    fn specialize_invocation(
        &self,
        invocation: &SpecializationInvocation,
        context: &mut SpecializationContext<'_>,
    ) -> MResult<SpecializedFunction> {
        if invocation.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: invocation.len(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let input = invocation
            .input(0)
            .expect("validated input")
            .cell()?
            .clone();
        let (rows, cols, _) = dynamic_arg_as_f64_matrix(&input, &self.name)?;
        let output = dynamic_f64_matrix_output(rows, cols)?;
        let runtime_invocation = FunctionInvocation::unary(output.clone(), input.clone());
        let (out, managed_input) = runtime_invocation.expect_unary()?;

        context.resolve_syntax_operation_contract(&DYNAMIC_UNARY_VIEW_CONTRACT)?;
        context.certify_instance(
            (
                Box::new(DynamicUnaryF64ViewToF64ViewFunction {
                    name: self.name.clone(),
                    input: managed_input.try_managed_element::<f64>()?,
                    output: out.try_managed_element::<f64>()?,
                    kernel: self.kernel,
                    _library: self._library.clone(),
                }),
                runtime_invocation,
            ),
            mech_core::RuntimeFunctionId::from_name(&self.name),
            mech_core::ExecutionTarget::DirectRuntime,
            mech_core::ImplementationMemoryClass::AbiContiguousBridge {
                input: 0,
                output: 0,
            },
        )
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_arg_as_f64_ref(value: &ValueCell, fxn_name: &str) -> MResult<()> {
    if value.closed_schema_body()? == SchemaBody::FloatingPoint(FloatWidth::W64) {
        Ok(())
    } else {
        Err(dynamic_argument_error(value, fxn_name, "f64 scalar"))
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_arg_as_f64_scalar_or_matrix(value: ValueCell, fxn_name: &str) -> MResult<DynamicF64Arg> {
    match value.closed_schema_body()? {
        SchemaBody::FloatingPoint(FloatWidth::W64) => Ok(DynamicF64Arg::Scalar(value)),
        SchemaBody::Matrix { element, .. }
            if *element == SchemaBody::FloatingPoint(FloatWidth::W64) =>
        {
            Ok(DynamicF64Arg::Matrix(value))
        }
        _ => Err(dynamic_argument_error(
            &value,
            fxn_name,
            "f64 scalar or matrix",
        )),
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_argument_error(value: &ValueCell, fxn_name: &str, expected: &str) -> MechError {
    MechError::new(
        GenericError {
            msg: format!(
                "dynamic function `{fxn_name}` expected {expected}, found {:?}",
                value.representation(),
            ),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(feature = "dynamic-modules")]
fn canonical_f64(value: &ValueCell) -> MResult<f64> {
    let snapshot = value.snapshot()?;
    let ValueData::F64(value) = snapshot.data() else {
        return Err(dynamic_argument_error(
            value,
            "dynamic kernel",
            "f64 scalar",
        ));
    };
    Ok(value.to_f64())
}

#[cfg(feature = "dynamic-modules")]
fn canonical_f64_matrix_values(value: &ValueCell) -> MResult<(usize, usize, Vec<f64>)> {
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = value.closed_schema_body()?
    else {
        return Err(dynamic_argument_error(
            value,
            "dynamic kernel",
            "f64 matrix",
        ));
    };
    if *element != SchemaBody::FloatingPoint(FloatWidth::W64) {
        return Err(dynamic_argument_error(
            value,
            "dynamic kernel",
            "f64 matrix",
        ));
    }
    let [
        DimensionExpr::Constant(rows),
        DimensionExpr::Constant(columns),
    ] = dimensions.as_ref()
    else {
        unreachable!("closed matrix schemas have concrete dimensions")
    };
    let draft = value.snapshot()?.canonical_data_draft().map_err(|error| {
        MechError::new(ValueCellSnapshotFailure { error }, None).with_compiler_loc()
    })?;
    let ValueDataDraft::Matrix(elements) = draft else {
        return Err(dynamic_argument_error(
            value,
            "dynamic kernel",
            "f64 matrix",
        ));
    };
    let mut values = Vec::with_capacity(elements.len());
    for element in elements {
        let ValueDataDraft::F64(element) = element else {
            return Err(dynamic_argument_error(
                value,
                "dynamic kernel",
                "f64 matrix",
            ));
        };
        values.push(element.to_f64());
    }
    Ok((
        usize::try_from(*rows)
            .map_err(|_| dynamic_argument_error(value, "dynamic kernel", "host-sized matrix"))?,
        usize::try_from(*columns)
            .map_err(|_| dynamic_argument_error(value, "dynamic kernel", "host-sized matrix"))?,
        values,
    ))
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_f64_matrix_output(rows: usize, columns: usize) -> MResult<ValueCell> {
    let count = rows.checked_mul(columns).ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: "dynamic matrix output shape overflowed".to_owned(),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    ValueCell::dynamic_rank_matrix(
        SchemaBody::FloatingPoint(FloatWidth::W64),
        vec![rows as u64, columns as u64].into_boxed_slice(),
        vec![ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(0.0)); count]
            .into_boxed_slice(),
    )
}

#[cfg(all(test, feature = "dynamic-modules"))]
fn replace_dynamic_matrix_output(
    output: &ValueCell,
    rows: usize,
    cols: usize,
    values: Vec<f64>,
    function_name: &str,
) -> MResult<()> {
    let expected_len = rows.checked_mul(cols).ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: format!(
                    "dynamic function `{}` matrix shape overflows",
                    function_name
                ),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    if values.len() != expected_len {
        return Err(MechError::new(
            GenericError {
                msg: format!(
                    "dynamic function `{}` produced {} values for shape {}x{}",
                    function_name,
                    values.len(),
                    rows,
                    cols
                ),
            },
            None,
        )
        .with_compiler_loc());
    }

    let next = output.rebuild_matrix_drafts(
        vec![rows as u64, cols as u64].into_boxed_slice(),
        values
            .into_iter()
            .map(|value| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value)))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )?;
    output.replace(&next)
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_binary_broadcast_plan(
    lhs: &DynamicF64Arg,
    rhs: &DynamicF64Arg,
    fxn_name: &str,
) -> MResult<DynamicF64BinaryBroadcastPlan> {
    match (lhs.matrix_shape(), rhs.matrix_shape()) {
        (Some((lhs_rows, lhs_cols)), Some((rhs_rows, rhs_cols))) => {
            if lhs_rows == rhs_rows && lhs_cols == rhs_cols {
                DynamicF64BinaryBroadcastPlan::new(
                    DynamicF64BinaryBroadcastKind::MatrixMatrix,
                    lhs_rows,
                    lhs_cols,
                    fxn_name,
                )
            } else {
                Err(MechError::new(
                    GenericError {
                        msg: format!(
                            "dynamic function `{}` cannot broadcast matrix shapes {}x{} and {}x{}; exact shape match is required",
                            fxn_name, lhs_rows, lhs_cols, rhs_rows, rhs_cols
                        ),
                    },
                    None,
                )
                .with_compiler_loc())
            }
        }
        (Some((rows, cols)), None) => DynamicF64BinaryBroadcastPlan::new(
            DynamicF64BinaryBroadcastKind::MatrixScalar,
            rows,
            cols,
            fxn_name,
        ),
        (None, Some((rows, cols))) => DynamicF64BinaryBroadcastPlan::new(
            DynamicF64BinaryBroadcastKind::ScalarMatrix,
            rows,
            cols,
            fxn_name,
        ),
        (None, None) => Err(MechError::new(
            GenericError {
                msg: format!(
                    "dynamic function `{}` does not need a broadcast plan for scalar/scalar arguments",
                    fxn_name
                ),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_arg_as_f64_matrix(
    value: &ValueCell,
    fxn_name: &str,
) -> MResult<(usize, usize, Vec<f64>)> {
    canonical_f64_matrix_values(value)
        .map_err(|_| dynamic_argument_error(value, fxn_name, "f64 matrix"))
}

#[cfg(feature = "dynamic-modules")]
struct DynamicBinaryF64F64ToF64Function {
    name: String,
    n: ManagedPort<f64>,
    k: ManagedPort<f64>,
    output: ManagedPort<f64>,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicBinaryF64F64ToF64Function {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_binary_port_views(&self.n, &self.k, &self.output, |n, k, output| {
            if n.len() != 1 || k.len() != 1 || output.len() != 1 {
                return Err(MechError::new(
                    GenericError {
                        msg: format!(
                            "dynamic function `{}` requires scalar managed ports",
                            self.name
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            output.try_fill_column_major(|_| {
                let mut next = 0.0;
                let status = unsafe {
                    (self.kernel)(
                        n.get_column_major(0).expect("validated scalar input"),
                        k.get_column_major(0).expect("validated scalar input"),
                        &mut next as *mut f64,
                    )
                };
                check_dynamic_kernel_status(&self.name, status)?;
                Ok(next)
            })
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&DYNAMIC_BINARY_SCALAR_CONTRACT)
    }

    fn to_string(&self) -> String {
        format!("dynamic {}", self.name)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "semantic-compiler"))]
impl MechFunctionCompiler for DynamicBinaryF64F64ToF64Function {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(self.output.cell(), ctx)?;
        let lhs = compile_value_cell_register(self.n.cell(), ctx)?;
        let rhs = compile_value_cell_register(self.k.cell(), ctx)?;
        let function = ctx.function_id(&self.name)?;
        ctx.emit_binop(function, output, lhs, rhs);
        Ok(output)
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicBinaryF64F64BroadcastFunction {
    name: String,
    lhs: ManagedPort<f64>,
    rhs: ManagedPort<f64>,
    output: ManagedPort<f64>,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(all(test, feature = "dynamic-modules"))]
fn solve_dynamic_binary_broadcast(
    lhs: &DynamicF64Arg,
    rhs: &DynamicF64Arg,
    output: &ValueCell,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    name: &str,
) -> MResult<()> {
    let plan = dynamic_binary_broadcast_plan(lhs, rhs, name)?;
    let mut values = Vec::with_capacity(plan.len);
    for index in 1..=plan.len {
        let mut value = 0.0;
        let status = unsafe { (kernel)(lhs.value_at(index)?, rhs.value_at(index)?, &mut value) };
        check_dynamic_kernel_status(name, status)?;
        values.push(value);
    }
    replace_dynamic_matrix_output(output, plan.rows, plan.cols, values, name)
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_managed_broadcast_element(
    input: &mech_core::ManagedValueView<'_, f64>,
    row: usize,
    column: usize,
    output_rows: usize,
    output_columns: usize,
    name: &str,
) -> MResult<f64> {
    let position = match (input.rows(), input.columns()) {
        (1, 1) => Some((0, 0)),
        (rows, columns) if rows == output_rows && columns == output_columns => Some((row, column)),
        _ => None,
    };
    let Some((input_row, input_column)) = position else {
        return Err(MechError::new(
            GenericError {
                msg: format!(
                    "dynamic function `{name}` cannot broadcast managed input shape {}x{} to {output_rows}x{output_columns}",
                    input.rows(),
                    input.columns(),
                ),
            },
            None,
        )
        .with_compiler_loc());
    };
    input.get(input_row, input_column).ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: format!("dynamic function `{name}` input coordinate is out of bounds"),
            },
            None,
        )
        .with_compiler_loc()
    })
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicBinaryF64F64BroadcastFunction {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_binary_port_views(&self.lhs, &self.rhs, &self.output, |lhs, rhs, output| {
            let rows = output.rows();
            let columns = output.columns();
            output.try_fill_column_major(|index| {
                let row = index % rows;
                let column = index / rows;
                let lhs = dynamic_managed_broadcast_element(
                    &lhs, row, column, rows, columns, &self.name,
                )?;
                let rhs = dynamic_managed_broadcast_element(
                    &rhs, row, column, rows, columns, &self.name,
                )?;
                let mut next = 0.0;
                let status = unsafe { (self.kernel)(lhs, rhs, &mut next as *mut f64) };
                check_dynamic_kernel_status(&self.name, status)?;
                Ok(next)
            })
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&DYNAMIC_BINARY_BROADCAST_CONTRACT)
    }

    fn to_string(&self) -> String {
        format!("dynamic {}", self.name)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "semantic-compiler"))]
impl MechFunctionCompiler for DynamicBinaryF64F64BroadcastFunction {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(self.output.cell(), ctx)?;
        let lhs = compile_value_cell_register(self.lhs.cell(), ctx)?;
        let rhs = compile_value_cell_register(self.rhs.cell(), ctx)?;
        let function = ctx.function_id(&self.name)?;
        ctx.emit_binop(function, output, lhs, rhs);
        Ok(output)
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ToF64Function {
    name: String,
    input: ManagedPort<f64>,
    output: ManagedPort<f64>,
    kernel: mech_abi::MechUnaryF64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicUnaryF64ToF64Function {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_unary_port_views(&self.input, &self.output, |input, output| {
            if input.len() != 1 || output.len() != 1 {
                return Err(MechError::new(
                    GenericError {
                        msg: format!(
                            "dynamic function `{}` requires scalar managed ports",
                            self.name
                        ),
                    },
                    None,
                )
                .with_compiler_loc());
            }
            output.try_fill_column_major(|_| {
                let mut next = 0.0;
                let status = unsafe {
                    (self.kernel)(
                        input.get_column_major(0).expect("validated scalar input"),
                        &mut next as *mut f64,
                    )
                };
                check_dynamic_kernel_status(&self.name, status)?;
                Ok(next)
            })
        })?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&DYNAMIC_UNARY_SCALAR_CONTRACT)
    }

    fn to_string(&self) -> String {
        format!("dynamic {}", self.name)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "semantic-compiler"))]
impl MechFunctionCompiler for DynamicUnaryF64ToF64Function {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(self.output.cell(), ctx)?;
        let input = compile_value_cell_register(self.input.cell(), ctx)?;
        let function = ctx.function_id(&self.name)?;
        ctx.emit_unop(function, output, input);
        Ok(output)
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ViewToF64ViewFunction {
    name: String,
    input: ManagedPort<f64>,
    output: ManagedPort<f64>,
    kernel: mech_abi::MechUnaryF64ViewToF64ViewKernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(all(test, feature = "dynamic-modules"))]
fn solve_dynamic_unary_view(
    input: &ValueCell,
    output: &ValueCell,
    kernel: mech_abi::MechUnaryF64ViewToF64ViewKernelV1,
    name: &str,
) -> MResult<()> {
    let (rows, cols, input_values) = canonical_f64_matrix_values(input)?;
    let len = rows.checked_mul(cols).ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: format!("dynamic function `{name}` matrix shape overflows"),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let mut output_values = vec![0.0; len];
    let status = unsafe {
        (kernel)(
            mech_abi::MechF64ViewV1 {
                ptr: input_values.as_ptr(),
                len,
                rows,
                cols,
            },
            mech_abi::MechF64ViewMutV1 {
                ptr: output_values.as_mut_ptr(),
                len,
                rows,
                cols,
            },
        )
    };
    check_dynamic_kernel_status(name, status)?;
    replace_dynamic_matrix_output(output, rows, cols, output_values, name)
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicUnaryF64ViewToF64ViewFunction {
    fn solve_managed(
        &self,
        frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        frame.with_f64_abi_contiguous_bridge(
            &self.input,
            &self.output,
            |input, rows, cols, output| {
                let status = unsafe {
                    (self.kernel)(
                        mech_abi::MechF64ViewV1 {
                            ptr: input.as_ptr(),
                            len: input.len(),
                            rows,
                            cols,
                        },
                        mech_abi::MechF64ViewMutV1 {
                            ptr: output.as_mut_ptr(),
                            len: output.len(),
                            rows,
                            cols,
                        },
                    )
                };
                check_dynamic_kernel_status(&self.name, status)
            },
        )?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
    }

    fn semantic_operation_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(&DYNAMIC_UNARY_VIEW_CONTRACT)
    }

    fn to_string(&self) -> String {
        format!("dynamic {}", self.name)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "semantic-compiler"))]
impl MechFunctionCompiler for DynamicUnaryF64ViewToF64ViewFunction {
    fn compile(&self, ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = compile_value_cell_register(self.output.cell(), ctx)?;
        let input = compile_value_cell_register(self.input.cell(), ctx)?;
        let function = ctx.function_id(&self.name)?;
        ctx.emit_unop(function, output, input);
        Ok(output)
    }
}

pub struct ModuleRegistry {
    loaders: Vec<Box<dyn ModuleLoader>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            loaders: Vec::new(),
        }
    }

    pub fn with_loader(mut self, loader: Box<dyn ModuleLoader>) -> Self {
        self.loaders.push(loader);
        self
    }

    pub fn available() -> Self {
        let registry = Self::new();
        #[cfg(feature = "dynamic-modules")]
        let registry = registry.with_loader(Box::new(DynamicModuleLoader::default()));
        registry
    }

    pub fn load(&self, module: &str) -> MResult<DynamicFunctionModuleFragment> {
        for loader in &self.loaders {
            if loader.can_load(module) {
                let fragment = loader.load(module)?;
                if fragment.module != module {
                    return Err(invalid_dynamic_fragment(
                        &fragment,
                        format!(
                            "loader returned module `{}` for request `{module}`",
                            fragment.module,
                        ),
                    ));
                }
                return Ok(fragment);
            }
        }

        Err(missing_module(module))
    }
}

/// Load one validated dynamic ABI module into the immutable source catalog.
/// Resident activation binds the same operation names directly from the ABI,
/// so no interpreter extension store participates in compilation or turns.
#[cfg(feature = "dynamic-modules")]
pub fn install_dynamic_source_module(
    builder: &mut FunctionCatalogBuilder,
    module: &str,
) -> MResult<ModuleManifest> {
    let fragment = ModuleRegistry::available().load(module)?;
    validate_dynamic_fragment(&fragment)?;
    let manifest = fragment.manifest();
    let entries = fragment
        .entries
        .iter()
        .map(|entry| (entry.id, entry))
        .collect::<BTreeMap<_, _>>();
    for export in &fragment.exports {
        let entry = entries.get(&export.extension).ok_or_else(|| {
            invalid_dynamic_fragment(
                &fragment,
                format!("export `{}` has no source entry", export.item),
            )
        })?;
        let declaration = fragment
            .source_declarations
            .get(&export.extension)
            .cloned()
            .ok_or_else(|| {
                invalid_dynamic_fragment(
                    &fragment,
                    format!("export `{}` has no source type declaration", export.item),
                )
            })?;
        let contracts = fragment
            .source_contracts
            .get(&export.extension)
            .cloned()
            .ok_or_else(|| {
                invalid_dynamic_fragment(
                    &fragment,
                    format!("export `{}` has no source operation contracts", export.item),
                )
            })?;
        let operation = builder.insert_canonical_specializer_with_contracts(
            entry.canonical_name.clone(),
            declaration,
            contracts.into_vec(),
            Arc::clone(&entry.specializer),
        )?;
        builder.insert_export(FunctionExport {
            operation,
            canonical_name: entry.canonical_name.clone(),
            module: Some(fragment.module.clone()),
            item: Some(export.item.clone()),
            exposure: FunctionExposure::ModuleOnly,
        })?;
    }
    Ok(manifest)
}

fn missing_module(module: &str) -> MechError {
    MechError::new(MissingFunctionError::named(module), None).with_compiler_loc()
}

fn invalid_dynamic_fragment(
    fragment: &DynamicFunctionModuleFragment,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        GenericError {
            msg: format!(
                "invalid dynamic function module fragment `{}`: {}",
                fragment.module,
                reason.into(),
            ),
        },
        None,
    )
    .with_compiler_loc()
}

#[cfg(any(test, feature = "dynamic-modules"))]
fn validate_dynamic_fragment(
    fragment: &DynamicFunctionModuleFragment,
) -> MResult<BTreeMap<String, (ExtensionFunctionId, String)>> {
    if fragment.module.is_empty() {
        return Err(invalid_dynamic_fragment(
            fragment,
            "module name must not be empty",
        ));
    }
    if fragment.entries.is_empty() || fragment.exports.is_empty() {
        return Err(invalid_dynamic_fragment(
            fragment,
            "at least one exact function entry and export is required",
        ));
    }

    let mut entries_by_id = BTreeMap::new();
    let mut entry_names = BTreeSet::new();
    for entry in &fragment.entries {
        if entries_by_id.insert(entry.id, entry).is_some() {
            return Err(invalid_dynamic_fragment(
                fragment,
                format!("duplicate extension entry ID 0x{:016x}", entry.id.raw()),
            ));
        }
        if !entry_names.insert(entry.canonical_name.as_str()) {
            return Err(invalid_dynamic_fragment(
                fragment,
                format!("duplicate extension entry `{}`", entry.canonical_name),
            ));
        }
    }

    let mut exports_by_item = BTreeMap::new();
    let mut referenced_entries = BTreeSet::new();
    for export in &fragment.exports {
        if export.item.is_empty() {
            return Err(invalid_dynamic_fragment(
                fragment,
                "export item names must not be empty",
            ));
        }
        let Some(entry) = entries_by_id.get(&export.extension).copied() else {
            return Err(invalid_dynamic_fragment(
                fragment,
                format!(
                    "export `{}` references missing extension ID 0x{:016x}",
                    export.item,
                    export.extension.raw(),
                ),
            ));
        };
        let expected_name = format!("{}/{}", fragment.module, export.item);
        if entry.canonical_name != expected_name {
            return Err(invalid_dynamic_fragment(
                fragment,
                format!(
                    "export `{}` references `{}` instead of exact canonical name `{expected_name}`",
                    export.item, entry.canonical_name,
                ),
            ));
        }
        if exports_by_item
            .insert(
                export.item.clone(),
                (export.extension, entry.canonical_name.clone()),
            )
            .is_some()
        {
            return Err(invalid_dynamic_fragment(
                fragment,
                format!("duplicate exact export item `{}`", export.item),
            ));
        }
        referenced_entries.insert(export.extension);
    }

    if referenced_entries.len() != entries_by_id.len() {
        let unexported = entries_by_id
            .iter()
            .find(|(id, _)| !referenced_entries.contains(id))
            .map(|(_, entry)| entry.canonical_name.as_str())
            .unwrap_or("<unknown>");
        return Err(invalid_dynamic_fragment(
            fragment,
            format!("extension entry `{unexported}` has no exact module export"),
        ));
    }

    Ok(exports_by_item)
}

#[cfg(test)]
mod dynamic_fragment_validation_tests {
    use super::*;

    struct TestSpecializer;

    impl CanonicalFunctionSpecializer for TestSpecializer {
        fn specialize_invocation(
            &self,
            _: &SpecializationInvocation,
            _: &mut SpecializationContext<'_>,
        ) -> MResult<SpecializedFunction> {
            unreachable!("module visibility tests do not specialize functions")
        }
    }

    struct FragmentLoader {
        fragment: DynamicFunctionModuleFragment,
    }

    impl ModuleLoader for FragmentLoader {
        fn can_load(&self, _: &str) -> bool {
            true
        }

        fn load(&self, _: &str) -> MResult<DynamicFunctionModuleFragment> {
            Ok(self.fragment.clone())
        }
    }

    fn dynamic_fragment(module: &str, items: &[&str]) -> DynamicFunctionModuleFragment {
        let mut entries = Vec::with_capacity(items.len());
        let mut exports = Vec::with_capacity(items.len());
        for item in items {
            let entry =
                FunctionExtensionEntry::new(format!("{module}/{item}"), Arc::new(TestSpecializer));
            exports.push(DynamicFunctionExport {
                item: (*item).to_string(),
                extension: entry.id,
            });
            entries.push(entry);
        }
        DynamicFunctionModuleFragment {
            module: module.to_string(),
            entries,
            exports,
            #[cfg(feature = "dynamic-modules")]
            source_declarations: BTreeMap::new(),
            #[cfg(feature = "dynamic-modules")]
            source_contracts: BTreeMap::new(),
        }
    }

    #[test]
    fn registry_rejects_a_fragment_for_a_different_module() {
        let registry = ModuleRegistry::new().with_loader(Box::new(FragmentLoader {
            fragment: dynamic_fragment("other", &["sin"]),
        }));

        let error = match registry.load("requested") {
            Ok(_) => panic!("mismatched dynamic fragment unexpectedly loaded"),
            Err(error) => error,
        };

        assert!(
            error
                .full_chain_message()
                .contains("loader returned module `other` for request `requested`")
        );
    }

    #[test]
    fn exact_dynamic_exports_are_validated_without_source_workspace() {
        let fragment = dynamic_fragment("dynamic", &["sin", "sum/column"]);
        let exports = validate_dynamic_fragment(&fragment).unwrap();
        assert_eq!(exports.len(), 2);
        assert_eq!(exports["sin"].1, "dynamic/sin");
        assert_eq!(exports["sum/column"].1, "dynamic/sum/column");
        let manifest = fragment.manifest();
        assert_eq!(manifest.items, ["sin", "sum/column"]);
    }

    #[test]
    fn malformed_dynamic_fragments_are_rejected_before_catalog_installation() {
        let valid = dynamic_fragment("dynamic", &["sin", "cos"]);
        let mut candidates = Vec::new();
        let mut duplicate_entry = valid.clone();
        duplicate_entry
            .entries
            .push(duplicate_entry.entries[0].clone());
        candidates.push((duplicate_entry, "duplicate extension entry"));
        let mut duplicate_export = valid.clone();
        duplicate_export
            .exports
            .push(duplicate_export.exports[0].clone());
        candidates.push((duplicate_export, "duplicate exact export item"));
        let mut missing = valid.clone();
        missing.exports[0].extension = ExtensionFunctionId::from_name("absent/item");
        candidates.push((missing, "references missing extension"));
        let mut wrong_name = valid.clone();
        wrong_name.exports[0].item = "different".into();
        candidates.push((wrong_name, "instead of exact canonical name"));
        let mut unexported = valid.clone();
        unexported.exports.pop();
        candidates.push((unexported, "has no exact module export"));
        for (candidate, expected) in candidates {
            let error = validate_dynamic_fragment(&candidate).unwrap_err();
            assert!(error.full_chain_message().contains(expected), "{error:?}");
        }
        assert_eq!(validate_dynamic_fragment(&valid).unwrap().len(), 2);
    }
}

#[cfg(all(test, feature = "dynamic-modules"))]
mod dynamic_binary_broadcast_tests {
    use super::*;

    fn scalar(value: f64) -> DynamicF64Arg {
        DynamicF64Arg::Scalar(ValueCell::from_exact(value).unwrap())
    }

    fn matrix(values: Vec<f64>, rows: usize, cols: usize) -> DynamicF64Arg {
        let output = dynamic_f64_matrix_output(rows, cols).unwrap();
        replace_dynamic_matrix_output(&output, rows, cols, values, "test").unwrap();
        DynamicF64Arg::Matrix(output)
    }

    #[test]
    fn broadcast_plan_matrix_scalar() {
        let lhs = matrix(vec![10.0, 20.0], 1, 2);
        let rhs = scalar(2.0);

        let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, "test").unwrap();

        assert_eq!(plan.kind, DynamicF64BinaryBroadcastKind::MatrixScalar);
        assert_eq!(plan.rows, 1);
        assert_eq!(plan.cols, 2);
        assert_eq!(plan.len, 2);
    }

    #[test]
    fn broadcast_plan_scalar_matrix() {
        let lhs = scalar(10.0);
        let rhs = matrix(vec![2.0, 3.0], 1, 2);

        let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, "test").unwrap();

        assert_eq!(plan.kind, DynamicF64BinaryBroadcastKind::ScalarMatrix);
        assert_eq!(plan.rows, 1);
        assert_eq!(plan.cols, 2);
        assert_eq!(plan.len, 2);
    }

    #[test]
    fn broadcast_plan_same_shape_matrix_matrix() {
        let lhs = matrix(vec![10.0, 20.0], 1, 2);
        let rhs = matrix(vec![2.0, 3.0], 1, 2);

        let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, "test").unwrap();

        assert_eq!(plan.kind, DynamicF64BinaryBroadcastKind::MatrixMatrix);
        assert_eq!(plan.rows, 1);
        assert_eq!(plan.cols, 2);
        assert_eq!(plan.len, 2);
    }

    #[test]
    fn broadcast_plan_rejects_different_lengths() {
        let lhs = matrix(vec![10.0, 20.0], 1, 2);
        let rhs = matrix(vec![2.0, 3.0, 4.0], 1, 3);

        assert!(dynamic_binary_broadcast_plan(&lhs, &rhs, "test").is_err());
    }

    #[test]
    fn broadcast_plan_rejects_same_len_different_shape() {
        let lhs = matrix(vec![1.0, 2.0, 3.0, 4.0], 1, 4);
        let rhs = matrix(vec![1.0, 2.0, 3.0, 4.0], 2, 2);

        assert!(dynamic_binary_broadcast_plan(&lhs, &rhs, "test").is_err());
    }

    #[test]
    fn broadcast_plan_rejects_scalar_scalar() {
        let lhs = scalar(10.0);
        let rhs = scalar(2.0);

        assert!(dynamic_binary_broadcast_plan(&lhs, &rhs, "test").is_err());
    }
}

#[cfg(all(test, feature = "dynamic-modules"))]
mod rc4_dynamic_abi_tests {
    use super::*;

    #[test]
    fn unknown_dynamic_status_is_rejected() {
        let err = DynamicModuleLoader::call_status(mech_abi::MechStatusV1(99), "test status")
            .expect_err("unknown status should be rejected");
        assert!(err.full_chain_message().contains("99"));
    }

    #[test]
    fn unknown_dynamic_kernel_kind_is_rejected() {
        let err = DynamicModuleLoader::validate_dynamic_kernel_kind(mech_abi::MechKernelKindV1(99))
            .expect_err("unknown kernel kind should be rejected");
        assert!(err.full_chain_message().contains("99"));
    }
}

#[cfg(all(test, feature = "dynamic-modules"))]
mod dynamic_live_shape_solve_tests {
    use super::*;

    extern "C" fn add_kernel(lhs: f64, rhs: f64, out: *mut f64) -> mech_abi::MechStatusV1 {
        unsafe {
            *out = lhs + rhs;
        }
        mech_abi::MechStatusV1::OK
    }

    extern "C" fn double_view_kernel(
        input: mech_abi::MechF64ViewV1,
        out: mech_abi::MechF64ViewMutV1,
    ) -> mech_abi::MechStatusV1 {
        for index in 0..input.len {
            unsafe {
                *out.ptr.add(index) = *input.ptr.add(index) * 2.0;
            }
        }
        mech_abi::MechStatusV1::OK
    }

    fn matrix(values: Vec<f64>, rows: usize, columns: usize) -> ValueCell {
        let cell = dynamic_f64_matrix_output(rows, columns).unwrap();
        set_matrix(&cell, values, rows, columns);
        cell
    }

    fn set_matrix(cell: &ValueCell, values: Vec<f64>, rows: usize, columns: usize) {
        replace_dynamic_matrix_output(cell, rows, columns, values, "test").unwrap();
    }

    fn contents(cell: &ValueCell) -> (usize, usize, Vec<f64>) {
        canonical_f64_matrix_values(cell).unwrap()
    }

    #[test]
    fn dynamic_binary_broadcast_recomputes_shape_on_solve() {
        let lhs_matrix = matrix(vec![1.0, 2.0], 1, 2);
        let rhs = DynamicF64Arg::Scalar(ValueCell::from_exact(10.0_f64).unwrap());
        let lhs = DynamicF64Arg::Matrix(lhs_matrix.clone());
        let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, "test").unwrap();
        let out = dynamic_f64_matrix_output(plan.rows, plan.cols).unwrap();
        solve_dynamic_binary_broadcast(&lhs, &rhs, &out, add_kernel, "test").unwrap();
        set_matrix(&lhs_matrix, vec![3.0, 4.0, 5.0, 6.0], 2, 2);
        solve_dynamic_binary_broadcast(&lhs, &rhs, &out, add_kernel, "test").unwrap();
        assert_eq!(contents(&out), (2, 2, vec![13.0, 14.0, 15.0, 16.0]));
    }

    #[test]
    fn dynamic_binary_broadcast_handles_growth_without_truncation() {
        let lhs_matrix = matrix(vec![1.0], 1, 1);
        let rhs = DynamicF64Arg::Scalar(ValueCell::from_exact(1.0_f64).unwrap());
        let lhs = DynamicF64Arg::Matrix(lhs_matrix.clone());
        let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, "test").unwrap();
        let out = dynamic_f64_matrix_output(plan.rows, plan.cols).unwrap();
        set_matrix(&lhs_matrix, vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        solve_dynamic_binary_broadcast(&lhs, &rhs, &out, add_kernel, "test").unwrap();
        assert_eq!(contents(&out), (2, 2, vec![2.0, 3.0, 4.0, 5.0]));
    }

    #[test]
    fn dynamic_binary_broadcast_handles_shrink_without_out_of_bounds() {
        let lhs_matrix = matrix(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let rhs = DynamicF64Arg::Scalar(ValueCell::from_exact(1.0_f64).unwrap());
        let lhs = DynamicF64Arg::Matrix(lhs_matrix.clone());
        let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, "test").unwrap();
        let out = dynamic_f64_matrix_output(plan.rows, plan.cols).unwrap();
        set_matrix(&lhs_matrix, vec![9.0], 1, 1);
        solve_dynamic_binary_broadcast(&lhs, &rhs, &out, add_kernel, "test").unwrap();
        assert_eq!(contents(&out), (1, 1, vec![10.0]));
    }

    #[test]
    fn dynamic_binary_shape_mismatch_preserves_last_successful_output() {
        let lhs_matrix = matrix(vec![1.0, 2.0], 1, 2);
        let rhs_matrix = matrix(vec![3.0, 4.0], 1, 2);
        let lhs = DynamicF64Arg::Matrix(lhs_matrix.clone());
        let rhs = DynamicF64Arg::Matrix(rhs_matrix.clone());
        let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, "test").unwrap();
        let out = dynamic_f64_matrix_output(plan.rows, plan.cols).unwrap();
        solve_dynamic_binary_broadcast(&lhs, &rhs, &out, add_kernel, "test").unwrap();
        set_matrix(&rhs_matrix, vec![1.0, 2.0, 3.0], 1, 3);
        assert!(solve_dynamic_binary_broadcast(&lhs, &rhs, &out, add_kernel, "test").is_err());
        assert_eq!(contents(&out), (1, 2, vec![4.0, 6.0]));
    }

    #[test]
    fn dynamic_unary_view_recomputes_shape_on_solve() {
        let input = matrix(vec![1.0, 2.0], 1, 2);
        let out = dynamic_f64_matrix_output(1, 2).unwrap();
        set_matrix(&input, vec![3.0, 4.0, 5.0, 6.0], 2, 2);
        solve_dynamic_unary_view(&input, &out, double_view_kernel, "test").unwrap();
        assert_eq!(contents(&out), (2, 2, vec![6.0, 8.0, 10.0, 12.0]));
    }

    #[test]
    fn dynamic_unary_view_handles_growth_without_truncation() {
        let input = matrix(vec![1.0], 1, 1);
        let out = dynamic_f64_matrix_output(1, 1).unwrap();
        set_matrix(&input, vec![1.0, 2.0, 3.0], 3, 1);
        solve_dynamic_unary_view(&input, &out, double_view_kernel, "test").unwrap();
        assert_eq!(contents(&out), (3, 1, vec![2.0, 4.0, 6.0]));
    }

    #[test]
    fn dynamic_unary_view_handles_shrink_without_out_of_bounds() {
        let input = matrix(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let out = dynamic_f64_matrix_output(2, 2).unwrap();
        set_matrix(&input, vec![7.0], 1, 1);
        solve_dynamic_unary_view(&input, &out, double_view_kernel, "test").unwrap();
        assert_eq!(contents(&out), (1, 1, vec![14.0]));
    }

    #[test]
    fn dynamic_matrix_output_retains_reference_identity() {
        let input = matrix(vec![1.0], 1, 1);
        let out = dynamic_f64_matrix_output(1, 1).unwrap();
        let alias = out.clone();
        set_matrix(&input, vec![1.0, 2.0], 2, 1);
        solve_dynamic_unary_view(&input, &out, double_view_kernel, "test").unwrap();
        assert!(out.same_cell(&alias));
        assert_eq!(contents(&out), (2, 1, vec![2.0, 4.0]));
    }

    #[test]
    fn dynamic_row_extent_remains_rank_two() {
        let input = matrix(vec![1.0, 2.0], 1, 2);
        let out = dynamic_f64_matrix_output(1, 2).unwrap();
        solve_dynamic_unary_view(&input, &out, double_view_kernel, "test").unwrap();
        assert_eq!(contents(&out), (1, 2, vec![2.0, 4.0]));
    }

    #[test]
    fn dynamic_column_extent_remains_rank_two() {
        let input = matrix(vec![1.0, 2.0], 2, 1);
        let out = dynamic_f64_matrix_output(2, 1).unwrap();
        solve_dynamic_unary_view(&input, &out, double_view_kernel, "test").unwrap();
        assert_eq!(contents(&out), (2, 1, vec![2.0, 4.0]));
    }
}
