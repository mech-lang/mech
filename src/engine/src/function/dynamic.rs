//! ABI module loading shared by source compilation and resident-only execution.
use crate::*;
use std::{path::PathBuf, sync::Arc};

#[cfg(feature = "dynamic-modules")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ValidatedDynamicKernelKind {
    UnaryF64ToF64,
    BinaryF64F64ToF64,
    UnaryF64ViewToF64View,
}

#[cfg(feature = "dynamic-modules")]
#[derive(Default)]
pub struct DynamicModuleLoader;

#[cfg(feature = "dynamic-modules")]
impl DynamicModuleLoader {
    pub(super) fn dynamic_error(msg: impl Into<String>) -> MechError {
        MechError::new(GenericError { msg: msg.into() }, None).with_compiler_loc()
    }

    pub(super) fn find_library(module: &str) -> Option<PathBuf> {
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

    pub(super) fn call_status(
        status: mech_abi::MechStatusV1,
        context: impl Into<String>,
    ) -> MResult<()> {
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

    pub(super) fn validate_dynamic_kernel_kind(
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
pub(super) unsafe extern "C" fn dynamic_null_binary_f64_f64_to_f64(
    _n: f64,
    _k: f64,
    _out: *mut f64,
) -> mech_abi::MechStatusV1 {
    mech_abi::MechStatusV1::UNSUPPORTED
}

#[cfg(feature = "dynamic-modules")]
pub(super) unsafe fn mech_str_to_string(s: mech_abi::MechStrV1) -> MResult<String> {
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

#[cfg(feature = "semantic-compiler")]
pub(super) fn dynamic_trace(message: impl AsRef<str>) {
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

#[cfg(feature = "semantic-compiler")]
pub(super) fn check_dynamic_kernel_status(
    function: &str,
    status: mech_abi::MechStatusV1,
) -> MResult<()> {
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
    operation: Box<str>,
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
        operation: canonical_name.into(),
        kernel,
    }))
}

#[cfg(all(feature = "dynamic-modules", feature = "resident-artifact"))]
fn dynamic_resident_status_error(
    operation: &str,
    status: mech_abi::MechStatusV1,
) -> ResidentKernelError {
    ResidentKernelError::ProviderStatus {
        operation: operation.into(),
        status: dynamic_status_name(status).into(),
        code: status.0,
    }
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
    let ResidentValueMut::F64(candidate) = output else {
        return Err(ResidentKernelError::InvalidOutput);
    };
    // Resident supplies the non-published transaction stage as `candidate`.
    // The module writes that admitted storage directly: a partial ABI write
    // is discarded with the turn on error, so no output-sized bridge Vec is
    // allocated and published state remains failure-atomic.
    let changed = match state.kernel {
        DynamicResidentKernelKind::UnaryScalar(kernel) => {
            let input = inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?;
            if input.len() != 1 || candidate.len() != 1 {
                return Err(ResidentKernelError::InvalidShape);
            }
            let previous = candidate[0].to_bits();
            let status = unsafe { kernel(input[0], candidate.as_mut_ptr()) };
            if status != mech_abi::MechStatusV1::OK {
                return Err(dynamic_resident_status_error(&state.operation, status));
            }
            candidate[0].to_bits() != previous
        }
        DynamicResidentKernelKind::BinaryScalar(kernel) => {
            let lhs = inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?;
            let rhs = inputs.f64(1).ok_or(ResidentKernelError::InvalidInput)?;
            if (lhs.len() != 1 && lhs.len() != candidate.len())
                || (rhs.len() != 1 && rhs.len() != candidate.len())
            {
                return Err(ResidentKernelError::InvalidShape);
            }
            let mut changed = false;
            for (index, target) in candidate.iter_mut().enumerate() {
                let previous = target.to_bits();
                let status = unsafe {
                    kernel(
                        lhs[if lhs.len() == 1 { 0 } else { index }],
                        rhs[if rhs.len() == 1 { 0 } else { index }],
                        target,
                    )
                };
                if status != mech_abi::MechStatusV1::OK {
                    return Err(dynamic_resident_status_error(&state.operation, status));
                }
                changed |= target.to_bits() != previous;
            }
            changed
        }
        DynamicResidentKernelKind::UnaryView(kernel) => {
            let input = inputs.f64(0).ok_or(ResidentKernelError::InvalidInput)?;
            if input.len() != candidate.len() {
                return Err(ResidentKernelError::InvalidShape);
            }
            let [rows, columns] = bound.parameters() else {
                return Err(ResidentKernelError::InvalidShape);
            };
            let rows = usize::try_from(*rows).map_err(|_| ResidentKernelError::InvalidShape)?;
            let columns =
                usize::try_from(*columns).map_err(|_| ResidentKernelError::InvalidShape)?;
            if rows.checked_mul(columns) != Some(candidate.len()) {
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
                        ptr: candidate.as_mut_ptr(),
                        len: candidate.len(),
                        rows,
                        cols: columns,
                    },
                )
            };
            if status != mech_abi::MechStatusV1::OK {
                return Err(dynamic_resident_status_error(&state.operation, status));
            }
            // The ABI owns the whole candidate view and does not expose a
            // per-element write callback. Conservatively report a successful
            // non-empty call as changed without allocating a comparison copy.
            !candidate.is_empty()
        }
    };
    Ok(changed)
}
