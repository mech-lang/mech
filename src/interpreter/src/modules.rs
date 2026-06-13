use crate::*;
#[cfg(feature = "dynamic-modules")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "dynamic-modules")]
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ModuleManifest {
    pub module: String,
    pub items: Vec<String>,
}

fn module_items(module: &str) -> Vec<String> {
    let mut items = Vec::<String>::new();

    for item_desc in inventory::iter::<ModuleItemDescriptor> {
        if item_desc.module == module {
            let item = item_desc.item.to_string();
            if !items.iter().any(|existing| existing == &item) {
                items.push(item);
            }
        }
    }

    items
}

fn has_module_item(module: &str) -> bool {
    inventory::iter::<ModuleItemDescriptor>
        .into_iter()
        .any(|item_desc| item_desc.module == module)
}

pub trait ModuleLoader {
    fn can_load(&self, module: &str) -> bool;
    fn load(&self, fxns: &mut Functions, module: &str) -> MResult<ModuleManifest>;
}

#[derive(Default)]
pub struct LinkedModuleLoader;

impl ModuleLoader for LinkedModuleLoader {
    fn can_load(&self, module: &str) -> bool {
        has_module_item(module)
    }

    fn load(&self, fxns: &mut Functions, module: &str) -> MResult<ModuleManifest> {
        let items = module_items(module);

        if items.is_empty() {
            return Err(MechError::new(
                MissingFunctionError {
                    function_id: hash_str(module),
                },
                None,
            )
            .with_compiler_loc());
        }

        let module_prefix = format!("{module}/");
        for fxn_comp in inventory::iter::<FunctionCompilerDescriptor> {
            let name = fxn_comp.name;

            if let Some(item) = name.strip_prefix(&module_prefix) {
                if items.iter().any(|manifest_item| manifest_item == item) {
                    fxns.insert_function_compiler(
                        name,
                        Arc::new(StaticNativeFunctionCompiler::new(fxn_comp.ptr)),
                    );
                }
            }
        }

        Ok(ModuleManifest {
            module: module.to_string(),
            items,
        })
    }
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
        if status == mech_abi::MechStatusV1::Ok {
            Ok(())
        } else {
            Err(Self::dynamic_error(format!(
                "{} returned status {:?}",
                context.into(),
                status
            )))
        }
    }
}

#[cfg(feature = "dynamic-modules")]
impl ModuleLoader for DynamicModuleLoader {
    fn can_load(&self, module: &str) -> bool {
        Self::find_library(module).is_some()
    }

    fn load(&self, fxns: &mut Functions, module: &str) -> MResult<ModuleManifest> {
        let path = Self::find_library(module).ok_or_else(|| {
            MechError::new(
                MissingFunctionError {
                    function_id: hash_str(module),
                },
                None,
            )
            .with_compiler_loc()
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
        let mut items = Vec::new();
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
        let mut dynamic_compilers = HashMap::<String, Vec<Arc<dyn NativeFunctionCompiler>>>::new();

        for index in 0..export_count {
            let mut export = mech_abi::MechExportV1 {
                name: mech_abi::MechStrV1 {
                    ptr: std::ptr::null(),
                    len: 0,
                },
                kind: mech_abi::MechKernelKindV1::BinaryF64F64ToF64,
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

            match export.kind {
                mech_abi::MechKernelKindV1::BinaryF64F64ToF64 => {
                    let kernel = unsafe { export.function.binary_f64_f64_to_f64 };
                    let compiler_name = export_name.clone();

                    dynamic_compilers
                        .entry(compiler_name.clone())
                        .or_default()
                        .push(Arc::new(DynamicBinaryF64F64ToF64Compiler {
                            name: compiler_name.clone(),
                            kernel,
                            _library: library.clone(),
                        }));

                    dynamic_trace(format!(
                        "registered dynamic export `{}` as item `{}`",
                        compiler_name, item
                    ));

                    if !items.iter().any(|existing| existing == &item) {
                        items.push(item);
                    }
                }
                mech_abi::MechKernelKindV1::UnaryF64ToF64 => {
                    let kernel = unsafe { export.function.unary_f64_to_f64 };
                    let compiler_name = export_name.clone();

                    dynamic_compilers
                        .entry(compiler_name.clone())
                        .or_default()
                        .push(Arc::new(DynamicUnaryF64ToF64Compiler {
                            name: compiler_name.clone(),
                            kernel,
                            _library: library.clone(),
                        }));

                    dynamic_trace(format!(
                        "registered dynamic export `{}` as item `{}`",
                        compiler_name, item
                    ));

                    if !items.iter().any(|existing| existing == &item) {
                        items.push(item);
                    }
                }
                mech_abi::MechKernelKindV1::UnaryF64ViewToF64View => {
                    let kernel = unsafe { export.function.unary_f64_view_to_f64_view };
                    let compiler_name = export_name.clone();

                    dynamic_compilers
                        .entry(compiler_name.clone())
                        .or_default()
                        .push(Arc::new(DynamicUnaryF64ViewToF64ViewCompiler {
                            name: compiler_name.clone(),
                            kernel,
                            _library: library.clone(),
                        }));

                    dynamic_trace(format!(
                        "registered dynamic export `{}` as item `{}`",
                        compiler_name, item
                    ));

                    if !items.iter().any(|existing| existing == &item) {
                        items.push(item);
                    }
                }
            }
        }

        for (compiler_name, compilers) in dynamic_compilers {
            if compilers.len() == 1 {
                let compiler = compilers.into_iter().next().expect("one compiler");
                fxns.insert_function_compiler(compiler_name, compiler);
            } else {
                fxns.insert_function_compiler(
                    compiler_name.clone(),
                    Arc::new(DynamicOverloadedCompiler {
                        name: compiler_name,
                        compilers,
                    }),
                );
            }
        }

        Ok(ModuleManifest {
            module: module.to_string(),
            items,
        })
    }
}

#[cfg(feature = "dynamic-modules")]
unsafe extern "C" fn dynamic_null_binary_f64_f64_to_f64(
    _n: f64,
    _k: f64,
    _out: *mut f64,
) -> mech_abi::MechStatusV1 {
    mech_abi::MechStatusV1::Unsupported
}

#[cfg(feature = "dynamic-modules")]
unsafe extern "C" fn dynamic_null_unary_f64_to_f64(
    _input: f64,
    _out: *mut f64,
) -> mech_abi::MechStatusV1 {
    mech_abi::MechStatusV1::Unsupported
}

#[cfg(feature = "dynamic-modules")]
unsafe extern "C" fn dynamic_null_unary_f64_view_to_f64_view(
    _input: mech_abi::MechF64ViewV1,
    _out: mech_abi::MechF64ViewMutV1,
) -> mech_abi::MechStatusV1 {
    mech_abi::MechStatusV1::Unsupported
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
struct DynamicOverloadedCompiler {
    name: String,
    compilers: Vec<Arc<dyn NativeFunctionCompiler>>,
}

#[cfg(feature = "dynamic-modules")]
impl NativeFunctionCompiler for DynamicOverloadedCompiler {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        let mut last_error = None;

        for compiler in &self.compilers {
            match compiler.compile(arguments) {
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
    Scalar(Ref<f64>),
    Matrix(Matrix<f64>),
}

#[cfg(feature = "dynamic-modules")]
impl DynamicF64Arg {
    fn matrix_shape(&self) -> Option<(usize, usize)> {
        match self {
            DynamicF64Arg::Scalar(_) => None,
            DynamicF64Arg::Matrix(matrix) => Some((matrix.rows(), matrix.cols())),
        }
    }

    fn value_at(&self, index: usize) -> f64 {
        match self {
            DynamicF64Arg::Scalar(value) => unsafe { *value.as_ptr() },
            DynamicF64Arg::Matrix(matrix) => matrix.index1d(index),
        }
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicBinaryF64F64ToF64Compiler {
    name: String,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl NativeFunctionCompiler for DynamicBinaryF64F64ToF64Compiler {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 2 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let lhs = dynamic_arg_as_f64_scalar_or_matrix(&arguments[0], &self.name)?;
        let rhs = dynamic_arg_as_f64_scalar_or_matrix(&arguments[1], &self.name)?;

        match (&lhs, &rhs) {
            (DynamicF64Arg::Scalar(n), DynamicF64Arg::Scalar(k)) => {
                Ok(Box::new(DynamicBinaryF64F64ToF64Function {
                    name: self.name.clone(),
                    n: n.clone(),
                    k: k.clone(),
                    out: Ref::new(0.0),
                    kernel: self.kernel,
                    _library: self._library.clone(),
                }))
            }

            _ => {
                let (rows, cols) = dynamic_binary_broadcast_shape(&lhs, &rhs, &self.name)?;
                let Some(len) = rows.checked_mul(cols) else {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "dynamic function `{}` broadcast shape overflowed: {} x {}",
                                self.name, rows, cols
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                };

                let out = Matrix::from_vec(vec![0.0; len], rows, cols);

                Ok(Box::new(DynamicBinaryF64F64BroadcastFunction {
                    name: self.name.clone(),
                    lhs,
                    rhs,
                    out,
                    len,
                    kernel: self.kernel,
                    _library: self._library.clone(),
                }))
            }
        }
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ToF64Compiler {
    name: String,
    kernel: mech_abi::MechUnaryF64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl NativeFunctionCompiler for DynamicUnaryF64ToF64Compiler {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let input = dynamic_arg_as_f64_ref(&arguments[0], &self.name)?;

        Ok(Box::new(DynamicUnaryF64ToF64Function {
            name: self.name.clone(),
            input,
            out: Ref::new(0.0),
            kernel: self.kernel,
            _library: self._library.clone(),
        }))
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ViewToF64ViewCompiler {
    name: String,
    kernel: mech_abi::MechUnaryF64ViewToF64ViewKernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl NativeFunctionCompiler for DynamicUnaryF64ViewToF64ViewCompiler {
    fn compile(&self, arguments: &Vec<Value>) -> MResult<Box<dyn MechFunction>> {
        if arguments.len() != 1 {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 1,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        }

        let input = dynamic_arg_as_f64_matrix(&arguments[0], &self.name)?;
        let rows = input.rows();
        let cols = input.cols();
        let len = rows * cols;
        let out = Matrix::from_vec(vec![0.0; len], rows, cols);

        Ok(Box::new(DynamicUnaryF64ViewToF64ViewFunction {
            name: self.name.clone(),
            input,
            out,
            len,
            rows,
            cols,
            kernel: self.kernel,
            _library: self._library.clone(),
        }))
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_arg_as_f64_ref(value: &Value, fxn_name: &str) -> MResult<Ref<f64>> {
    match value {
        Value::F64(v) => Ok(v.clone()),
        Value::MutableReference(v) => {
            let borrowed = v.borrow();
            match &*borrowed {
                Value::F64(inner) => Ok(inner.clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: fxn_name.to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            }
        }
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind1 {
                arg: x.kind(),
                fxn_name: fxn_name.to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_arg_as_f64_scalar_or_matrix(value: &Value, fxn_name: &str) -> MResult<DynamicF64Arg> {
    match value {
        #[cfg(feature = "f64")]
        Value::F64(v) => Ok(DynamicF64Arg::Scalar(v.clone())),

        #[cfg(all(feature = "matrix", feature = "f64"))]
        Value::MatrixF64(matrix) => Ok(DynamicF64Arg::Matrix(matrix.clone())),

        Value::MutableReference(v) => {
            let borrowed = v.borrow();
            match &*borrowed {
                #[cfg(feature = "f64")]
                Value::F64(inner) => Ok(DynamicF64Arg::Scalar(inner.clone())),

                #[cfg(all(feature = "matrix", feature = "f64"))]
                Value::MatrixF64(matrix) => Ok(DynamicF64Arg::Matrix(matrix.clone())),

                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: fxn_name.to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            }
        }

        x => Err(MechError::new(
            UnhandledFunctionArgumentKind1 {
                arg: x.kind(),
                fxn_name: fxn_name.to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_binary_broadcast_shape(
    lhs: &DynamicF64Arg,
    rhs: &DynamicF64Arg,
    fxn_name: &str,
) -> MResult<(usize, usize)> {
    match (lhs.matrix_shape(), rhs.matrix_shape()) {
        (Some((lhs_rows, lhs_cols)), Some((rhs_rows, rhs_cols))) => {
            if lhs_rows == rhs_rows && lhs_cols == rhs_cols {
                Ok((lhs_rows, lhs_cols))
            } else {
                Err(MechError::new(
                    GenericError {
                        msg: format!(
                            "dynamic function `{}` cannot broadcast matrix shapes {}x{} and {}x{}",
                            fxn_name, lhs_rows, lhs_cols, rhs_rows, rhs_cols
                        ),
                    },
                    None,
                )
                .with_compiler_loc())
            }
        }
        (Some(shape), None) => Ok(shape),
        (None, Some(shape)) => Ok(shape),
        (None, None) => Err(MechError::new(
            GenericError {
                msg: format!(
                    "dynamic function `{}` expected at least one matrix argument for broadcast",
                    fxn_name
                ),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_arg_as_f64_matrix(value: &Value, fxn_name: &str) -> MResult<Matrix<f64>> {
    match value {
        #[cfg(all(feature = "matrix", feature = "f64"))]
        Value::MatrixF64(matrix) => Ok(matrix.clone()),
        Value::MutableReference(v) => {
            let borrowed = v.borrow();
            match &*borrowed {
                #[cfg(all(feature = "matrix", feature = "f64"))]
                Value::MatrixF64(matrix) => Ok(matrix.clone()),
                x => Err(MechError::new(
                    UnhandledFunctionArgumentKind1 {
                        arg: x.kind(),
                        fxn_name: fxn_name.to_string(),
                    },
                    None,
                )
                .with_compiler_loc()),
            }
        }
        x => Err(MechError::new(
            UnhandledFunctionArgumentKind1 {
                arg: x.kind(),
                fxn_name: fxn_name.to_string(),
            },
            None,
        )
        .with_compiler_loc()),
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicBinaryF64F64ToF64Function {
    name: String,
    n: Ref<f64>,
    k: Ref<f64>,
    out: Ref<f64>,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicBinaryF64F64ToF64Function {
    fn solve(&self) {
        let status =
            unsafe { (self.kernel)(*self.n.as_ptr(), *self.k.as_ptr(), self.out.as_mut_ptr()) };

        if status != mech_abi::MechStatusV1::Ok {
            dynamic_trace(format!(
                "dynamic kernel `{}` returned status {:?}",
                self.name, status
            ));
        }
    }

    fn out(&self) -> Value {
        self.out.to_value()
    }

    fn to_string(&self) -> String {
        format!("dynamic {}", self.name)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "compiler"))]
impl MechFunctionCompiler for DynamicBinaryF64F64ToF64Function {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: format!(
                    "bytecode compilation is not implemented for dynamic function `{}`",
                    self.name
                ),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicBinaryF64F64BroadcastFunction {
    name: String,
    lhs: DynamicF64Arg,
    rhs: DynamicF64Arg,
    out: Matrix<f64>,
    len: usize,
    kernel: mech_abi::MechBinaryF64F64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicBinaryF64F64BroadcastFunction {
    fn solve(&self) {
        let mut out_vec = Vec::with_capacity(self.len);

        for index in 1..=self.len {
            let lhs = self.lhs.value_at(index);
            let rhs = self.rhs.value_at(index);
            let mut out = 0.0;

            let status = unsafe { (self.kernel)(lhs, rhs, &mut out as *mut f64) };

            if status != mech_abi::MechStatusV1::Ok {
                dynamic_trace(format!(
                    "dynamic kernel `{}` returned status {:?}",
                    self.name, status
                ));
                return;
            }

            out_vec.push(out);
        }

        self.out.set(out_vec);
    }

    fn out(&self) -> Value {
        Value::MatrixF64(self.out.clone())
    }

    fn to_string(&self) -> String {
        format!("dynamic {}", self.name)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "compiler"))]
impl MechFunctionCompiler for DynamicBinaryF64F64BroadcastFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: format!(
                    "bytecode compilation is not implemented for dynamic function `{}`",
                    self.name
                ),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ToF64Function {
    name: String,
    input: Ref<f64>,
    out: Ref<f64>,
    kernel: mech_abi::MechUnaryF64ToF64KernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicUnaryF64ToF64Function {
    fn solve(&self) {
        let status = unsafe { (self.kernel)(*self.input.as_ptr(), self.out.as_mut_ptr()) };

        if status != mech_abi::MechStatusV1::Ok {
            dynamic_trace(format!(
                "dynamic kernel `{}` returned status {:?}",
                self.name, status
            ));
        }
    }

    fn out(&self) -> Value {
        self.out.to_value()
    }

    fn to_string(&self) -> String {
        format!("dynamic {}", self.name)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "compiler"))]
impl MechFunctionCompiler for DynamicUnaryF64ToF64Function {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: format!(
                    "bytecode compilation is not implemented for dynamic function `{}`",
                    self.name
                ),
            },
            None,
        )
        .with_compiler_loc())
    }
}

#[cfg(feature = "dynamic-modules")]
struct DynamicUnaryF64ViewToF64ViewFunction {
    name: String,
    input: Matrix<f64>,
    out: Matrix<f64>,
    len: usize,
    rows: usize,
    cols: usize,
    kernel: mech_abi::MechUnaryF64ViewToF64ViewKernelV1,
    _library: Arc<libloading::Library>,
}

#[cfg(feature = "dynamic-modules")]
impl MechFunctionImpl for DynamicUnaryF64ViewToF64ViewFunction {
    fn solve(&self) {
        let mut input_vec = Vec::with_capacity(self.len);
        for index in 1..=self.len {
            input_vec.push(self.input.index1d(index));
        }

        let mut out_vec = vec![0.0; self.len];

        let status = unsafe {
            (self.kernel)(
                mech_abi::MechF64ViewV1 {
                    ptr: input_vec.as_ptr(),
                    len: input_vec.len(),
                    rows: self.rows,
                    cols: self.cols,
                },
                mech_abi::MechF64ViewMutV1 {
                    ptr: out_vec.as_mut_ptr(),
                    len: out_vec.len(),
                    rows: self.rows,
                    cols: self.cols,
                },
            )
        };

        if status == mech_abi::MechStatusV1::Ok {
            self.out.set(out_vec);
        } else {
            dynamic_trace(format!(
                "dynamic kernel `{}` returned status {:?}",
                self.name, status
            ));
        }
    }

    fn out(&self) -> Value {
        Value::MatrixF64(self.out.clone())
    }

    fn to_string(&self) -> String {
        format!("dynamic {}", self.name)
    }
}

#[cfg(all(feature = "dynamic-modules", feature = "compiler"))]
impl MechFunctionCompiler for DynamicUnaryF64ViewToF64ViewFunction {
    fn compile(&self, _ctx: &mut CompileCtx) -> MResult<Register> {
        Err(MechError::new(
            GenericError {
                msg: format!(
                    "bytecode compilation is not implemented for dynamic function `{}`",
                    self.name
                ),
            },
            None,
        )
        .with_compiler_loc())
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

    pub fn linked_stdlib() -> Self {
        let registry = Self::new().with_loader(Box::new(LinkedModuleLoader::default()));
        #[cfg(feature = "dynamic-modules")]
        let registry = registry.with_loader(Box::new(DynamicModuleLoader::default()));
        registry
    }

    pub fn load(&self, fxns: &mut Functions, module: &str) -> MResult<ModuleManifest> {
        for loader in &self.loaders {
            if loader.can_load(module) {
                return loader.load(fxns, module);
            }
        }

        Err(MechError::new(
            MissingFunctionError {
                function_id: hash_str(module),
            },
            None,
        )
        .with_compiler_loc())
    }
}

pub fn load_module(fxns: &mut Functions, module: &str) -> MResult<ModuleManifest> {
    ModuleRegistry::linked_stdlib().load(fxns, module)
}

pub fn import_module_qualified(fxns: &mut Functions, module: &str) -> MResult<ModuleManifest> {
    load_module(fxns, module)
}

pub fn import_module_item(fxns: &mut Functions, module: &str, item: &str) -> MResult<()> {
    let manifest = load_module(fxns, module)?;
    if !manifest
        .items
        .iter()
        .any(|manifest_item| manifest_item == item)
    {
        return Err(MechError::new(
            MissingFunctionError {
                function_id: hash_str(&format!("{module}/{item}")),
            },
            None,
        )
        .with_compiler_loc());
    }
    alias_module_item(fxns, module, item)
}

pub fn import_module_glob(fxns: &mut Functions, module: &str) -> MResult<()> {
    let manifest = load_module(fxns, module)?;
    for item in manifest.items.iter() {
        alias_module_item(fxns, module, item)?;
    }
    Ok(())
}

fn alias_module_item(fxns: &mut Functions, module: &str, item: &str) -> MResult<()> {
    let qualified_name = format!("{module}/{item}");
    let qualified_id = hash_str(&qualified_name);
    let local_name = item.rsplit('/').next().unwrap_or(item);
    let local_id = hash_str(local_name);
    let mut found = false;

    if let Some(ptr) = fxns.function_compilers.get(&qualified_id).cloned() {
        fxns.function_compilers.insert(local_id, ptr);
        fxns.dictionary
            .borrow_mut()
            .insert(local_id, local_name.to_string());
        found = true;
    }

    if let Some(ptr) = fxns.functions.get(&qualified_id).copied() {
        fxns.functions.insert(local_id, ptr);
        fxns.dictionary
            .borrow_mut()
            .insert(local_id, local_name.to_string());
        found = true;
    }

    if found {
        Ok(())
    } else {
        Err(MechError::new(
            MissingFunctionError {
                function_id: qualified_id,
            },
            None,
        )
        .with_compiler_loc())
    }
}
