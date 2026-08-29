use crate::*;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "dynamic-modules")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "dynamic-modules")]
use std::path::PathBuf;
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
}

impl DynamicFunctionModuleFragment {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ValidatedDynamicKernelKind {
    UnaryF64ToF64,
    BinaryF64F64ToF64,
    UnaryF64ViewToF64View,
}

#[cfg(feature = "dynamic-modules")]
#[derive(Default)]
pub struct DynamicModuleLoader;

#[cfg(feature = "dynamic-modules")]
impl DynamicModuleLoader {
    fn dynamic_error(msg: impl Into<String>) -> MechError {
        MechError::new(GenericError { msg: msg.into() }, None).with_compiler_loc()
    }

    fn find_library(module: &str) -> Option<PathBuf> {
        let module_file_part = module.replace('-', "_").replace('/', "_");
        let candidates = [
            format!("mech_module_{module_file_part}.dll"),
            format!("libmech_module_{module_file_part}.so"),
            format!("libmech_module_{module_file_part}.dylib"),
        ];

        let mut dirs: Vec<PathBuf> = std::env::var_os("MECH_MODULE_PATH")
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default();
        dirs.push(PathBuf::from("target/mech-modules"));

        for dir in dirs {
            for candidate in &candidates {
                let path = dir.join(candidate);
                if path.is_file() {
                    return Some(path);
                }
            }
        }

        None
    }

    fn call_status(status: mech_abi::MechStatusV1, context: impl Into<String>) -> MResult<()> {
        if status == mech_abi::MechStatusV1::OK {
            Ok(())
        } else {
            Err(Self::dynamic_error(format!(
                "{} returned status {}",
                context.into(),
                status.0
            )))
        }
    }

    fn validate_dynamic_kernel_kind(
        kind: mech_abi::MechKernelKindV1,
    ) -> MResult<ValidatedDynamicKernelKind> {
        match kind.0 {
            1 => Ok(ValidatedDynamicKernelKind::UnaryF64ToF64),
            2 => Ok(ValidatedDynamicKernelKind::BinaryF64F64ToF64),
            3 => Ok(ValidatedDynamicKernelKind::UnaryF64ViewToF64View),
            other => Err(Self::dynamic_error(format!(
                "dynamic module exported unsupported kernel kind {other}"
            ))),
        }
    }
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

            match Self::validate_dynamic_kernel_kind(export.kind)? {
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
            entries.push(entry);
        }

        Ok(DynamicFunctionModuleFragment {
            module: module.to_string(),
            entries,
            exports,
        })
    }
}

#[cfg(feature = "dynamic-modules")]
unsafe extern "C" fn dynamic_null_binary_f64_f64_to_f64(
    _n: f64,
    _k: f64,
    _out: *mut f64,
) -> mech_abi::MechStatusV1 {
    mech_abi::MechStatusV1::UNSUPPORTED
}

#[cfg(feature = "dynamic-modules")]
unsafe fn mech_str_to_string(s: mech_abi::MechStrV1) -> MResult<String> {
    if s.ptr.is_null() {
        return Err(DynamicModuleLoader::dynamic_error("null MechStrV1 pointer"));
    }

    let bytes = unsafe { std::slice::from_raw_parts(s.ptr, s.len) };
    std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .map_err(|err| {
            DynamicModuleLoader::dynamic_error(format!(
                "invalid utf8 in dynamic module string: {err}"
            ))
        })
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_trace(message: impl AsRef<str>) {
    if std::env::var_os("MECH_DYNAMIC_TRACE").is_some() {
        eprintln!("[mech-dynamic] {}", message.as_ref());
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_status_name(status: mech_abi::MechStatusV1) -> &'static str {
    match status.0 {
        0 => "Ok",
        1 => "InvalidIndex",
        2 => "NullPointer",
        3 => "WrongType",
        4 => "WrongShape",
        5 => "Unsupported",
        6 => "Panic",
        _ => "Unknown",
    }
}

#[cfg(feature = "dynamic-modules")]
fn check_dynamic_kernel_status(function: &str, status: mech_abi::MechStatusV1) -> MResult<()> {
    if status == mech_abi::MechStatusV1::OK {
        return Ok(());
    }

    Err(DynamicModuleLoader::dynamic_error(format!(
        "dynamic kernel `{}` returned {} (status {})",
        function,
        dynamic_status_name(status),
        status.0,
    )))
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
enum DynamicResidentKernelKind {
    UnaryScalar(mech_abi::MechUnaryF64ToF64KernelV1),
    BinaryScalar(mech_abi::MechBinaryF64F64ToF64KernelV1),
    UnaryView(mech_abi::MechUnaryF64ViewToF64ViewKernelV1),
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
struct DynamicResidentKernelState {
    _library: Arc<libloading::Library>,
    kernel: DynamicResidentKernelKind,
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
fn dynamic_resident_contract_matches(
    request: &ResidentKernelBindRequest<'_>,
    input_count: usize,
    shape: ShapeRule,
    change_detection: ChangeDetectionPolicy,
) -> bool {
    let ResolvedOperationContract::Declared(contract) = request.contract else {
        return false;
    };
    contract.interaction == ExternalInteraction::Pure
        && contract.inputs.len() == input_count
        && request.inputs.len() == input_count
        && contract.outputs.len() == 1
        && contract
            .inputs
            .iter()
            .zip(request.inputs)
            .all(|(port, layout)| {
                port.schema == layout.schema_id
                    && port.access == AccessMode::Read
                    && port.delivery == DeliveryMode::Signal
                    && layout.kind == ResidentValueKind::F64
                    && layout.shape.len().is_some()
            })
        && contract.outputs[0].schema == request.output.schema_id
        && contract.outputs[0].access == AccessMode::Write
        && contract.outputs[0].delivery == DeliveryMode::Signal
        && contract.outputs[0].construction == OutputConstruction::FullWrite { shape }
        && contract.outputs[0].alias == AliasPolicy::NoAlias
        && contract.outputs[0].change_detection == change_detection
        && request.output.kind == ResidentValueKind::F64
        && request.output.shape.len().is_some()
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
fn dynamic_binary_layout_matches(request: &ResidentKernelBindRequest<'_>) -> bool {
    let [lhs, rhs] = request.inputs else {
        return false;
    };
    let output = request.output.shape;
    (lhs.shape == ResidentShape::SCALAR || lhs.shape == output)
        && (rhs.shape == ResidentShape::SCALAR || rhs.shape == output)
        && (lhs.shape == output || rhs.shape == output)
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
fn dynamic_resident_kernel_kind(
    request: &ResidentKernelBindRequest<'_>,
) -> Option<ValidatedDynamicKernelKind> {
    match request.inputs {
        [input]
            if input.kind == ResidentValueKind::F64
                && request.output.kind == ResidentValueKind::F64
                && input.shape == request.output.shape =>
        {
            if input.shape == ResidentShape::SCALAR {
                Some(ValidatedDynamicKernelKind::UnaryF64ToF64)
            } else {
                Some(ValidatedDynamicKernelKind::UnaryF64ViewToF64View)
            }
        }
        [_, _] if dynamic_binary_layout_matches(request) => {
            Some(ValidatedDynamicKernelKind::BinaryF64F64ToF64)
        }
        _ => None,
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
fn load_dynamic_resident_kernel(
    module: &str,
    canonical_name: &str,
    expected: ValidatedDynamicKernelKind,
) -> MResult<Arc<DynamicResidentKernelState>> {
    let path = DynamicModuleLoader::find_library(module).ok_or_else(|| {
        DynamicModuleLoader::dynamic_error(format!("dynamic module `{module}` was not found"))
    })?;
    let library = Arc::new(unsafe { libloading::Library::new(&path) }.map_err(|error| {
        DynamicModuleLoader::dynamic_error(format!(
            "failed to open dynamic module `{module}` at {}: {error}",
            path.display()
        ))
    })?);
    let (abi_version, module_name_fn, export_count_fn, get_export_fn) = unsafe {
        (
            *library
                .get::<mech_abi::MechModuleAbiVersionFnV1>(b"mech_module_abi_version_v1\0")
                .map_err(|error| DynamicModuleLoader::dynamic_error(error.to_string()))?,
            *library
                .get::<mech_abi::MechModuleNameFnV1>(b"mech_module_name_v1\0")
                .map_err(|error| DynamicModuleLoader::dynamic_error(error.to_string()))?,
            *library
                .get::<mech_abi::MechModuleExportCountFnV1>(b"mech_module_export_count_v1\0")
                .map_err(|error| DynamicModuleLoader::dynamic_error(error.to_string()))?,
            *library
                .get::<mech_abi::MechModuleGetExportFnV1>(b"mech_module_get_export_v1\0")
                .map_err(|error| DynamicModuleLoader::dynamic_error(error.to_string()))?,
        )
    };
    let version = unsafe { abi_version() };
    if version != mech_abi::MECH_MODULE_ABI_VERSION_V1 {
        return Err(DynamicModuleLoader::dynamic_error(format!(
            "unsupported dynamic module ABI version {version}; expected {}",
            mech_abi::MECH_MODULE_ABI_VERSION_V1
        )));
    }
    let mut module_name = mech_abi::MechStrV1 {
        ptr: core::ptr::null(),
        len: 0,
    };
    DynamicModuleLoader::call_status(
        unsafe { module_name_fn(&mut module_name) },
        "mech_module_name_v1",
    )?;
    if unsafe { mech_str_to_string(module_name) }? != module {
        return Err(DynamicModuleLoader::dynamic_error(format!(
            "dynamic module name did not match requested module `{module}`"
        )));
    }

    let mut selected = None;
    for index in 0..unsafe { export_count_fn() } {
        let mut export = mech_abi::MechExportV1 {
            name: mech_abi::MechStrV1 {
                ptr: core::ptr::null(),
                len: 0,
            },
            kind: mech_abi::MechKernelKindV1::BINARY_F64_F64_TO_F64,
            function: mech_abi::MechKernelFnV1 {
                binary_f64_f64_to_f64: dynamic_null_binary_f64_f64_to_f64,
            },
        };
        DynamicModuleLoader::call_status(
            unsafe { get_export_fn(index, &mut export) },
            format!("mech_module_get_export_v1({index})"),
        )?;
        if unsafe { mech_str_to_string(export.name) }? != canonical_name
            || DynamicModuleLoader::validate_dynamic_kernel_kind(export.kind)? != expected
        {
            continue;
        }
        if selected.is_some() {
            return Err(DynamicModuleLoader::dynamic_error(format!(
                "dynamic module `{module}` exported duplicate resident kernel `{canonical_name}`"
            )));
        }
        selected = Some(match expected {
            ValidatedDynamicKernelKind::UnaryF64ToF64 => {
                DynamicResidentKernelKind::UnaryScalar(unsafe { export.function.unary_f64_to_f64 })
            }
            ValidatedDynamicKernelKind::BinaryF64F64ToF64 => {
                DynamicResidentKernelKind::BinaryScalar(unsafe {
                    export.function.binary_f64_f64_to_f64
                })
            }
            ValidatedDynamicKernelKind::UnaryF64ViewToF64View => {
                DynamicResidentKernelKind::UnaryView(unsafe {
                    export.function.unary_f64_view_to_f64_view
                })
            }
        });
    }
    let kernel = selected.ok_or_else(|| {
        DynamicModuleLoader::dynamic_error(format!(
            "dynamic module `{module}` has no compatible resident export `{canonical_name}`"
        ))
    })?;
    Ok(Arc::new(DynamicResidentKernelState {
        _library: library,
        kernel,
    }))
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
pub(crate) fn bind_dynamic_resident_operation(
    module_path: &[String],
    operation_name: &str,
    request: &ResidentKernelBindRequest<'_>,
) -> Option<Result<BoundResidentKernel, ResidentKernelBindError>> {
    let module = module_path.join("/");
    DynamicModuleLoader::find_library(&module)?;
    let expected = match dynamic_resident_kernel_kind(request) {
        Some(expected) => expected,
        None => return Some(Err(ResidentKernelBindError::UnsupportedLayout)),
    };
    let scalar = request.output.shape == ResidentShape::SCALAR;
    let (shape, change_detection) = match expected {
        ValidatedDynamicKernelKind::UnaryF64ToF64 => (
            ShapeRule::SameAsInput { input: 0 },
            ChangeDetectionPolicy::ExactScalar,
        ),
        ValidatedDynamicKernelKind::UnaryF64ViewToF64View => (
            ShapeRule::SameAsInput { input: 0 },
            ChangeDetectionPolicy::KernelReported,
        ),
        ValidatedDynamicKernelKind::BinaryF64F64ToF64 => (
            ShapeRule::Declared,
            if scalar {
                ChangeDetectionPolicy::ExactScalar
            } else {
                ChangeDetectionPolicy::KernelReported
            },
        ),
    };
    if !dynamic_resident_contract_matches(request, request.inputs.len(), shape, change_detection) {
        return Some(Err(ResidentKernelBindError::UnsupportedContract));
    }
    let canonical_name = format!("{module}/{operation_name}");
    let state = match load_dynamic_resident_kernel(&module, &canonical_name, expected) {
        Ok(state) => state,
        Err(_) => return Some(Err(ResidentKernelBindError::InvalidParameters)),
    };
    Some(Ok(BoundResidentKernel::new(
        dynamic_resident_execute,
        vec![
            u64::from(request.output.shape.rows),
            u64::from(request.output.shape.columns),
        ]
        .into_boxed_slice(),
    )
    .with_retained_state(state)))
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
fn dynamic_resident_execute(
    bound: &BoundResidentKernel,
    inputs: &dyn ResidentKernelInputs,
    output: ResidentValueMut<'_>,
) -> Result<bool, ResidentKernelError> {
    let state = bound
        .retained_state::<DynamicResidentKernelState>()
        .ok_or(ResidentKernelError::InvalidInput)?;
    let ResidentValueMut::F64(output) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    let mut next = vec![0.0; output.len()];
    match state.kernel {
        DynamicResidentKernelKind::UnaryScalar(kernel) => {
            let input = inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?;
            if input.len() != 1 || next.len() != 1 {
                return Err(ResidentKernelError::InvalidShape);
            }
            let status = unsafe { kernel(input[0], next.as_mut_ptr()) };
            if status != mech_abi::MechStatusV1::OK {
                return Err(ResidentKernelError::Arithmetic);
            }
        }
        DynamicResidentKernelKind::BinaryScalar(kernel) => {
            let lhs = inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?;
            let rhs = inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?;
            if (lhs.len() != 1 && lhs.len() != next.len())
                || (rhs.len() != 1 && rhs.len() != next.len())
            {
                return Err(ResidentKernelError::InvalidShape);
            }
            for (index, target) in next.iter_mut().enumerate() {
                let status = unsafe {
                    kernel(
                        lhs[if lhs.len() == 1 { 0 } else { index }],
                        rhs[if rhs.len() == 1 { 0 } else { index }],
                        target,
                    )
                };
                if status != mech_abi::MechStatusV1::OK {
                    return Err(ResidentKernelError::Arithmetic);
                }
            }
        }
        DynamicResidentKernelKind::UnaryView(kernel) => {
            let input = inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?;
            if input.len() != next.len() {
                return Err(ResidentKernelError::InvalidShape);
            }
            let [rows, columns] = bound.parameters() else {
                return Err(ResidentKernelError::InvalidShape);
            };
            let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
            let columns =
                usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
            if rows.checked_mul(columns) != Some(output.len()) {
                return Err(ResidentKernelError::InvalidShape);
            }
            let status = unsafe {
                kernel(
                    mech_abi::MechF64ViewV1 {
                        ptr: input.as_ptr(),
                        len: input.len(),
                        rows,
                        cols: columns,
                    },
                    mech_abi::MechF64ViewMutV1 {
                        ptr: next.as_mut_ptr(),
                        len: next.len(),
                        rows,
                        cols: columns,
                    },
                )
            };
            if status != mech_abi::MechStatusV1::OK {
                return Err(ResidentKernelError::Arithmetic);
            }
        }
    }
    let changed = output
        .iter()
        .zip(&next)
        .any(|(current, incoming)| current.to_bits() != incoming.to_bits());
    output.copy_from_slice(&next);
    Ok(changed)
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
            DynamicF64Arg::Scalar(_) => None,
            DynamicF64Arg::Matrix(matrix) => canonical_f64_matrix_values(matrix)
                .ok()
                .map(|(rows, columns, _)| (rows, columns)),
        }
    }

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
        _context: &mut SpecializationContext<'_>,
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

        let (implementation, output): (Box<dyn MechFunction>, ValueCell) = match (&lhs, &rhs) {
            (DynamicF64Arg::Scalar(n), DynamicF64Arg::Scalar(k)) => {
                let output = ValueCell::from_exact(0.0_f64)?;
                (
                    Box::new(DynamicBinaryF64F64ToF64Function {
                        name: self.name.clone(),
                        n: n.clone(),
                        k: k.clone(),
                        output: output.clone(),
                        kernel: self.kernel,
                        _library: self._library.clone(),
                    }),
                    output,
                )
            }

            _ => {
                let plan = dynamic_binary_broadcast_plan(&lhs, &rhs, &self.name)?;
                let output = dynamic_f64_matrix_output(plan.rows, plan.cols)?;

                (
                    Box::new(DynamicBinaryF64F64BroadcastFunction {
                        name: self.name.clone(),
                        lhs,
                        rhs,
                        output: output.clone(),
                        kernel: self.kernel,
                        _library: self._library.clone(),
                    }),
                    output,
                )
            }
        };
        Ok(SpecializedFunction::new(FunctionInstance::new(
            implementation,
            FunctionInvocation::binary(output, lhs_cell, rhs_cell),
        )))
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
        _context: &mut SpecializationContext<'_>,
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

        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(DynamicUnaryF64ToF64Function {
                name: self.name.clone(),
                input: input.clone(),
                output: output.clone(),
                kernel: self.kernel,
                _library: self._library.clone(),
            }),
            FunctionInvocation::unary(output, input),
        )))
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
        _context: &mut SpecializationContext<'_>,
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

        Ok(SpecializedFunction::new(FunctionInstance::new(
            Box::new(DynamicUnaryF64ViewToF64ViewFunction {
                name: self.name.clone(),
                input: input.clone(),
                output: output.clone(),
                kernel: self.kernel,
                _library: self._library.clone(),
            }),
            FunctionInvocation::unary(output, input),
        )))
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
    use mech_core::snapshot::SequenceView;

    let snapshot = value.snapshot()?;
    let ValueData::Matrix(matrix) = snapshot.data() else {
        return Err(dynamic_argument_error(
            value,
            "dynamic kernel",
            "f64 matrix",
        ));
    };
    let SequenceView::F64(elements) = matrix.elements() else {
        return Err(dynamic_argument_error(
            value,
            "dynamic kernel",
            "f64 matrix",
        ));
    };
    let shape = value.shape();
    let dimensions = shape.parameter_values();
    let [rows, columns] = dimensions else {
        return Err(dynamic_argument_error(
            value,
            "dynamic kernel",
            "rank-two f64 matrix",
        ));
    };
    Ok((
        usize::try_from(*rows)
            .map_err(|_| dynamic_argument_error(value, "dynamic kernel", "host-sized matrix"))?,
        usize::try_from(*columns)
            .map_err(|_| dynamic_argument_error(value, "dynamic kernel", "host-sized matrix"))?,
        elements.iter().map(|element| element.to_f64()).collect(),
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

#[cfg(feature = "dynamic-modules")]
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
    n: ValueCell,
    k: ValueCell,
    output: ValueCell,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
fn solve_dynamic_binary_scalar(
    n: &ValueCell,
    k: &ValueCell,
    output: &ValueCell,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    name: &str,
) -> MResult<()> {
    let mut next = canonical_f64(output)?;
    let status = unsafe { (kernel)(canonical_f64(n)?, canonical_f64(k)?, &mut next as *mut f64) };
    check_dynamic_kernel_status(name, status)?;
    output.replace(&ValueCell::from_exact(next)?.snapshot()?)
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicBinaryF64F64ToF64Function {
    fn solve_result(&self) -> MResult<()> {
        solve_dynamic_binary_scalar(&self.n, &self.k, &self.output, self.kernel, &self.name)
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
        let output = compile_value_cell_register(&self.output, ctx)?;
        let lhs = compile_value_cell_register(&self.n, ctx)?;
        let rhs = compile_value_cell_register(&self.k, ctx)?;
        let function = ctx.function_id(&self.name)?;
        ctx.emit_binop(function, output, lhs, rhs);
        Ok(output)
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicBinaryF64F64BroadcastFunction {
    name: String,
    lhs: DynamicF64Arg,
    rhs: DynamicF64Arg,
    output: ValueCell,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
fn solve_dynamic_binary_broadcast(
    lhs: &DynamicF64Arg,
    rhs: &DynamicF64Arg,
    output: &ValueCell,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    name: &str,
) -> MResult<()> {
    let plan = dynamic_binary_broadcast_plan(lhs, rhs, name)?;
    let mut out_vec = Vec::with_capacity(plan.len);
    for index in 1..=plan.len {
        let mut value = 0.0;
        let status = unsafe {
            (kernel)(
                lhs.value_at(index)?,
                rhs.value_at(index)?,
                &mut value as *mut f64,
            )
        };
        check_dynamic_kernel_status(name, status)?;
        out_vec.push(value);
    }
    replace_dynamic_matrix_output(output, plan.rows, plan.cols, out_vec, name)
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicBinaryF64F64BroadcastFunction {
    fn solve_result(&self) -> MResult<()> {
        solve_dynamic_binary_broadcast(&self.lhs, &self.rhs, &self.output, self.kernel, &self.name)
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
        let output = compile_value_cell_register(&self.output, ctx)?;
        let lhs = compile_dynamic_f64_arg(&self.lhs, ctx)?;
        let rhs = compile_dynamic_f64_arg(&self.rhs, ctx)?;
        let function = ctx.function_id(&self.name)?;
        ctx.emit_binop(function, output, lhs, rhs);
        Ok(output)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "semantic-compiler"))]
fn compile_dynamic_f64_arg(
    argument: &DynamicF64Arg,
    ctx: &mut dyn BytecodeCompilerContext,
) -> MResult<Register> {
    Ok(match argument {
        DynamicF64Arg::Scalar(value) | DynamicF64Arg::Matrix(value) => {
            compile_value_cell_register(value, ctx)?
        }
    })
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ToF64Function {
    name: String,
    input: ValueCell,
    output: ValueCell,
    kernel: mech_abi::MechUnaryF64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
fn solve_dynamic_unary_scalar(
    input: &ValueCell,
    output: &ValueCell,
    kernel: mech_abi::MechUnaryF64ToF64KernelV1,
    name: &str,
) -> MResult<()> {
    let mut next = canonical_f64(output)?;
    let status = unsafe { (kernel)(canonical_f64(input)?, &mut next as *mut f64) };
    check_dynamic_kernel_status(name, status)?;
    output.replace(&ValueCell::from_exact(next)?.snapshot()?)
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicUnaryF64ToF64Function {
    fn solve_result(&self) -> MResult<()> {
        solve_dynamic_unary_scalar(&self.input, &self.output, self.kernel, &self.name)
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
        let output = compile_value_cell_register(&self.output, ctx)?;
        let input = compile_value_cell_register(&self.input, ctx)?;
        let function = ctx.function_id(&self.name)?;
        ctx.emit_unop(function, output, input);
        Ok(output)
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ViewToF64ViewFunction {
    name: String,
    input: ValueCell,
    output: ValueCell,
    kernel: mech_abi::MechUnaryF64ViewToF64ViewKernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
fn solve_dynamic_unary_view(
    input: &ValueCell,
    output: &ValueCell,
    kernel: mech_abi::MechUnaryF64ViewToF64ViewKernelV1,
    name: &str,
) -> MResult<()> {
    let (rows, cols, input_vec) = canonical_f64_matrix_values(input)?;
    let len = rows.checked_mul(cols).ok_or_else(|| {
        MechError::new(
            GenericError {
                msg: format!("dynamic function `{}` matrix shape overflows", name),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    let mut out_vec = vec![0.0; len];
    let status = unsafe {
        (kernel)(
            mech_abi::MechF64ViewV1 {
                ptr: input_vec.as_ptr(),
                len,
                rows,
                cols,
            },
            mech_abi::MechF64ViewMutV1 {
                ptr: out_vec.as_mut_ptr(),
                len,
                rows,
                cols,
            },
        )
    };
    check_dynamic_kernel_status(name, status)?;
    replace_dynamic_matrix_output(output, rows, cols, out_vec, name)
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicUnaryF64ViewToF64ViewFunction {
    fn solve_result(&self) -> MResult<()> {
        solve_dynamic_unary_view(&self.input, &self.output, self.kernel, &self.name)
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
        let output = compile_value_cell_register(&self.output, ctx)?;
        let input = compile_value_cell_register(&self.input, ctx)?;
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

fn missing_module(module: &str) -> MechError {
    MechError::new(MissingFunctionError::named(module), None).with_compiler_loc()
}

fn missing_module_item(module: &str, item: &str) -> MechError {
    MechError::new(
        MissingFunctionError::named(format!("{module}/{item}")),
        None,
    )
    .with_compiler_loc()
}

fn static_module_manifest(catalog: &FunctionCatalog, module: &str) -> Option<ModuleManifest> {
    if !catalog.has_module(module) {
        return None;
    }

    Some(ModuleManifest {
        module: module.to_string(),
        items: catalog
            .module_exports(module)
            .map(|export| {
                export
                    .item
                    .clone()
                    .expect("catalog module exports always have an item")
            })
            .collect(),
    })
}

fn bind_static_exports<'a>(
    interpreter: &Interpreter,
    bindings: impl IntoIterator<Item = (&'a FunctionExport, String)>,
) -> MResult<()> {
    let mut state = interpreter.state.borrow_mut();
    let checkpoint = state.function_environment.clone();

    for (export, visible_name) in bindings {
        if let Err(error) = state
            .function_environment
            .bind_catalog_export(export, &visible_name)
        {
            state.function_environment = checkpoint;
            return Err(error);
        }
    }

    Ok(())
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

fn install_dynamic_fragment(
    interpreter: &Interpreter,
    fragment: DynamicFunctionModuleFragment,
    binding_requests: Vec<(String, String)>,
) -> MResult<ModuleManifest> {
    let exports_by_item = validate_dynamic_fragment(&fragment)?;
    let mut bindings = Vec::with_capacity(binding_requests.len());
    for (item, visible_name) in binding_requests {
        let Some((extension, canonical_name)) = exports_by_item.get(&item) else {
            return Err(missing_module_item(&fragment.module, &item));
        };
        bindings.push((*extension, canonical_name.clone(), visible_name));
    }

    let manifest = fragment.manifest();
    let mut state = interpreter.state.borrow_mut();
    let mut next_extensions = state.function_extensions.clone();
    for entry in fragment.entries {
        next_extensions.insert_or_replace(entry)?;
    }
    for export in fragment.exports {
        next_extensions.insert_module_export_or_replace(
            fragment.module.clone(),
            export.item,
            export.extension,
        )?;
    }

    let mut next_environment = state.function_environment.clone();
    for (extension, canonical_name, visible_name) in bindings {
        next_environment.bind_extension(&canonical_name, &visible_name, extension)?;
    }

    state.function_extensions = next_extensions;
    state.function_environment = next_environment;
    Ok(manifest)
}

fn load_module_with_registry(
    interpreter: &Interpreter,
    module: &str,
    registry: &ModuleRegistry,
) -> MResult<ModuleManifest> {
    let catalog = interpreter.function_catalog();
    if let Some(manifest) = static_module_manifest(catalog, module) {
        let bindings = catalog.module_exports(module).map(|export| {
            let item = export
                .item
                .as_deref()
                .expect("catalog module exports always have an item");
            (export, format!("{module}/{item}"))
        });
        bind_static_exports(interpreter, bindings)?;
        return Ok(manifest);
    }

    let fragment = registry.load(module)?;
    let binding_requests = fragment
        .exports
        .iter()
        .map(|export| (export.item.clone(), format!("{module}/{}", export.item)))
        .collect();
    install_dynamic_fragment(interpreter, fragment, binding_requests)
}

pub fn load_module(interpreter: &Interpreter, module: &str) -> MResult<ModuleManifest> {
    load_module_with_registry(interpreter, module, &ModuleRegistry::available())
}

pub fn import_module_qualified(interpreter: &Interpreter, module: &str) -> MResult<ModuleManifest> {
    load_module(interpreter, module)
}

pub fn import_module_item(interpreter: &Interpreter, module: &str, item: &str) -> MResult<()> {
    let catalog = interpreter.function_catalog();
    if catalog.has_module(module) {
        let export = catalog
            .module_export(module, item)
            .ok_or_else(|| missing_module_item(module, item))?;
        let local_name = item.rsplit('/').next().unwrap_or(item);
        return bind_static_exports(interpreter, [(export, local_name.to_string())]);
    }

    let fragment = ModuleRegistry::available().load(module)?;
    let local_name = item.rsplit('/').next().unwrap_or(item);
    install_dynamic_fragment(
        interpreter,
        fragment,
        vec![(item.to_string(), local_name.to_string())],
    )?;
    Ok(())
}

pub fn import_module_item_as(
    interpreter: &Interpreter,
    module: &str,
    item: &str,
    alias: &str,
) -> MResult<()> {
    let catalog = interpreter.function_catalog();
    if catalog.has_module(module) {
        let export = catalog
            .module_export(module, item)
            .ok_or_else(|| missing_module_item(module, item))?;
        return bind_static_exports(interpreter, [(export, alias.to_string())]);
    }

    let fragment = ModuleRegistry::available().load(module)?;
    install_dynamic_fragment(
        interpreter,
        fragment,
        vec![(item.to_string(), alias.to_string())],
    )?;
    Ok(())
}

pub fn import_module_group(
    interpreter: &Interpreter,
    module: &str,
    items: &[String],
) -> MResult<()> {
    let catalog = interpreter.function_catalog();
    if catalog.has_module(module) {
        let mut bindings = Vec::with_capacity(items.len());
        for item in items {
            let export = catalog
                .module_export(module, item)
                .ok_or_else(|| missing_module_item(module, item))?;
            let local_name = item.rsplit('/').next().unwrap_or(item);
            bindings.push((export, local_name.to_string()));
        }
        return bind_static_exports(interpreter, bindings);
    }

    let fragment = ModuleRegistry::available().load(module)?;
    let binding_requests = items
        .iter()
        .map(|item| {
            let local_name = item.rsplit('/').next().unwrap_or(item);
            (item.clone(), local_name.to_string())
        })
        .collect();
    install_dynamic_fragment(interpreter, fragment, binding_requests)?;
    Ok(())
}

pub fn import_module_glob(interpreter: &Interpreter, module: &str) -> MResult<()> {
    let catalog = interpreter.function_catalog();
    if catalog.has_module(module) {
        let bindings = catalog.module_exports(module).map(|export| {
            let item = export
                .item
                .as_deref()
                .expect("catalog module exports always have an item");
            let local_name = item.rsplit('/').next().unwrap_or(item);
            (export, local_name.to_string())
        });
        return bind_static_exports(interpreter, bindings);
    }

    let fragment = ModuleRegistry::available().load(module)?;
    let binding_requests = fragment
        .exports
        .iter()
        .map(|export| {
            let local_name = export.item.rsplit('/').next().unwrap_or(&export.item);
            (export.item.clone(), local_name.to_string())
        })
        .collect();
    install_dynamic_fragment(interpreter, fragment, binding_requests)?;
    Ok(())
}

#[cfg(test)]
mod static_catalog_module_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct TestSpecializer;

    impl FunctionSpecializer for TestSpecializer {
        fn specialize(&self, _: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
            unreachable!("module visibility tests do not specialize functions")
        }
    }

    fn static_catalog() -> Arc<FunctionCatalog> {
        let mut builder = FunctionCatalogBuilder::new();
        for (canonical_name, module, item) in [
            ("math/sin", "math", "sin"),
            ("math/cos", "math", "cos"),
            ("stats/sum/column", "stats", "sum/column"),
        ] {
            let operation = builder
                .insert_specializer(canonical_name, Arc::new(TestSpecializer))
                .unwrap();
            builder
                .insert_export(FunctionExport {
                    operation,
                    canonical_name: canonical_name.to_string(),
                    module: Some(module.to_string()),
                    item: Some(item.to_string()),
                    exposure: FunctionExposure::ModuleOnly,
                })
                .unwrap();
        }
        Arc::new(builder.build().unwrap())
    }

    fn interpreter() -> Interpreter {
        Interpreter::with_function_catalog(0, 100, static_catalog())
    }

    fn binding(interpreter: &Interpreter, name: &str) -> Option<FunctionBinding> {
        interpreter
            .state
            .borrow()
            .function_environment
            .resolve_name(name)
    }

    #[test]
    fn qualified_item_alias_glob_and_nested_imports_bind_exact_catalog_exports() {
        let qualified = interpreter();
        let manifest = load_module(&qualified, "math").unwrap();
        assert_eq!(manifest.items, ["cos", "sin"]);
        assert_eq!(
            binding(&qualified, "math/cos"),
            Some(FunctionBinding::CatalogOperation(OperationId::from_name(
                "math/cos",
            ))),
        );
        assert_eq!(binding(&qualified, "cos"), None);

        let item = interpreter();
        import_module_item(&item, "stats", "sum/column").unwrap();
        assert_eq!(
            binding(&item, "column"),
            Some(FunctionBinding::CatalogOperation(OperationId::from_name(
                "stats/sum/column",
            ))),
        );
        assert_eq!(binding(&item, "stats/sum/column"), None);

        let alias = interpreter();
        import_module_item_as(&alias, "math", "cos", "trig").unwrap();
        assert_eq!(
            binding(&alias, "trig"),
            Some(FunctionBinding::CatalogOperation(OperationId::from_name(
                "math/cos",
            ))),
        );
        assert_eq!(binding(&alias, "cos"), None);

        let glob = interpreter();
        import_module_glob(&glob, "math").unwrap();
        assert!(binding(&glob, "cos").is_some());
        assert!(binding(&glob, "sin").is_some());
        assert_eq!(binding(&glob, "math/cos"), None);
    }

    struct RecordingLoader {
        probed: Arc<AtomicBool>,
    }

    impl ModuleLoader for RecordingLoader {
        fn can_load(&self, _: &str) -> bool {
            self.probed.store(true, Ordering::SeqCst);
            true
        }

        fn load(&self, _: &str) -> MResult<DynamicFunctionModuleFragment> {
            unreachable!("an exact static module must take precedence")
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
            let entry = FunctionExtensionEntry::new(
                format!("{module}/{item}"),
                canonical_function_specializer(Arc::new(TestSpecializer)),
            );
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
        }
    }

    #[test]
    fn exact_static_module_precedes_every_dynamic_loader() {
        let interpreter = interpreter();
        let probed = Arc::new(AtomicBool::new(false));
        let registry = ModuleRegistry::new().with_loader(Box::new(RecordingLoader {
            probed: probed.clone(),
        }));

        load_module_with_registry(&interpreter, "math", &registry).unwrap();

        assert!(!probed.load(Ordering::SeqCst));
    }

    #[test]
    fn dynamic_fragment_installs_exact_exports_and_qualified_bindings() {
        let interpreter = interpreter();
        let registry = ModuleRegistry::new().with_loader(Box::new(FragmentLoader {
            fragment: dynamic_fragment("dynamic", &["sin", "sum/column"]),
        }));

        let manifest = load_module_with_registry(&interpreter, "dynamic", &registry).unwrap();

        assert_eq!(manifest.module, "dynamic");
        assert_eq!(manifest.items, ["sin", "sum/column"]);
        let sin = ExtensionFunctionId::from_name("dynamic/sin");
        let column = ExtensionFunctionId::from_name("dynamic/sum/column");
        assert_eq!(
            binding(&interpreter, "dynamic/sin"),
            Some(FunctionBinding::Extension(sin)),
        );
        assert_eq!(
            binding(&interpreter, "dynamic/sum/column"),
            Some(FunctionBinding::Extension(column)),
        );
        assert_eq!(binding(&interpreter, "sin"), None);

        let state = interpreter.state.borrow();
        assert!(state.function_extensions.entry(sin).is_some());
        assert!(state.function_extensions.entry(column).is_some());
        assert_eq!(
            state.function_extensions.module_export("dynamic", "sin"),
            Some(sin),
        );
        assert_eq!(
            state
                .function_extensions
                .module_export("dynamic", "sum/column"),
            Some(column),
        );
    }

    #[test]
    fn failed_dynamic_binding_does_not_install_entries_exports_or_names() {
        let interpreter = interpreter();
        let extension = ExtensionFunctionId::from_name("dynamic/sin");

        let error = install_dynamic_fragment(
            &interpreter,
            dynamic_fragment("dynamic", &["sin"]),
            vec![(String::from("sin"), String::new())],
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "FunctionEnvironmentInvalidBinding");
        let state = interpreter.state.borrow();
        assert!(state.function_extensions.entry(extension).is_none());
        assert_eq!(
            state.function_extensions.module_export("dynamic", "sin"),
            None,
        );
        assert_eq!(state.function_environment.resolve_name("dynamic/sin"), None);
        assert_eq!(state.function_environment.resolve_name("sin"), None);
    }

    #[test]
    fn checkpoint_restore_removes_a_loaded_dynamic_fragment_without_changing_the_catalog() {
        let mut interpreter = interpreter();
        let catalog = Arc::clone(interpreter.function_catalog());
        let extension = ExtensionFunctionId::from_name("dynamic/sin");
        let checkpoint = interpreter.checkpoint().unwrap();

        install_dynamic_fragment(
            &interpreter,
            dynamic_fragment("dynamic", &["sin"]),
            vec![(String::from("sin"), String::from("s"))],
        )
        .unwrap();
        assert_eq!(
            binding(&interpreter, "s"),
            Some(FunctionBinding::Extension(extension)),
        );

        interpreter.restore(checkpoint).unwrap();

        assert!(Arc::ptr_eq(interpreter.function_catalog(), &catalog));
        let state = interpreter.state.borrow();
        assert!(state.function_extensions.entry(extension).is_none());
        assert_eq!(
            state.function_extensions.module_export("dynamic", "sin"),
            None,
        );
        assert_eq!(state.function_environment.resolve_name("s"), None);
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
    fn missing_item_in_static_module_does_not_mutate_visibility() {
        let interpreter = interpreter();

        let error = import_module_item(&interpreter, "math", "missing").unwrap_err();

        assert_eq!(error.kind_name(), "MissingFunction");
        assert_eq!(binding(&interpreter, "missing"), None);
        assert_eq!(binding(&interpreter, "math/missing"), None);
    }

    #[test]
    fn failed_group_import_rolls_back_every_earlier_binding() {
        let interpreter = interpreter();

        let error = import_module_group(
            &interpreter,
            "math",
            &[String::from("sin"), String::from("missing")],
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "MissingFunction");
        assert_eq!(binding(&interpreter, "sin"), None);
        assert_eq!(binding(&interpreter, "missing"), None);
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
