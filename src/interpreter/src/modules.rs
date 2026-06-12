use crate::*;
#[cfg(feature = "dynamic-modules")]
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(feature = "dynamic-modules")]
use std::sync::{Mutex, OnceLock};

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

#[track_caller]
fn missing_module_error(module: &str) -> MechError {
    let function_id = hash_str(module);
    MechError::new(
        MissingFunctionError { function_id },
        Some(format!(
            "Missing module `{module}` (function id {function_id})"
        )),
    )
    .with_compiler_loc()
}

#[track_caller]
fn missing_module_item_error(module: &str, item: &str) -> MechError {
    let name = format!("{module}/{item}");
    let function_id = hash_str(&name);
    MechError::new(
        MissingFunctionError { function_id },
        Some(format!(
            "Missing module item `{name}` (function id {function_id})"
        )),
    )
    .with_compiler_loc()
}

#[track_caller]
fn missing_alias_error(qualified_name: &str, local_name: &str) -> MechError {
    let function_id = hash_str(qualified_name);
    MechError::new(
        MissingFunctionError { function_id },
        Some(format!(
            "Cannot alias `{qualified_name}` as `{local_name}`: function compiler not found (function id {function_id})"
        )),
    )
    .with_compiler_loc()
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
            return Err(missing_module_error(module));
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
fn dynamic_trace_enabled() -> bool {
    std::env::var_os("MECH_DYNAMIC_TRACE").is_some()
}

#[cfg(feature = "dynamic-modules")]
fn dynamic_trace(message: impl std::fmt::Display) {
    if dynamic_trace_enabled() {
        eprintln!("[dynamic-module] {message}");
    }
}

#[cfg(feature = "dynamic-modules")]
static DYNAMIC_MODULE_LIBRARIES: OnceLock<Mutex<Vec<libloading::Library>>> = OnceLock::new();

#[cfg(feature = "dynamic-modules")]
pub struct DynamicModuleLoader {
    search_paths: Vec<PathBuf>,
}

#[cfg(feature = "dynamic-modules")]
impl DynamicModuleLoader {
    pub fn from_env() -> Self {
        let mut search_paths = Vec::new();

        if let Some(paths) = std::env::var_os("MECH_MODULE_PATH") {
            search_paths.extend(std::env::split_paths(&paths));
        }

        search_paths.push(PathBuf::from("mech-modules"));
        search_paths.push(PathBuf::from("target/mech-modules"));

        dynamic_trace(format!("search paths: {:?}", search_paths));

        Self { search_paths }
    }

    fn candidate_paths(&self, module: &str) -> Vec<PathBuf> {
        let filename_module = module.replace('-', "_");
        let filename = format!(
            "{}mech_module_{}{}",
            std::env::consts::DLL_PREFIX,
            filename_module,
            std::env::consts::DLL_SUFFIX
        );

        let candidates: Vec<PathBuf> = self
            .search_paths
            .iter()
            .map(|search_path| search_path.join(&filename))
            .collect();
        dynamic_trace(format!("module `{module}` candidates: {:?}", candidates));
        candidates
    }

    fn dynamic_error(msg: impl Into<String>) -> MechError {
        MechError::new(GenericError { msg: msg.into() }, None).with_compiler_loc()
    }
}

#[cfg(feature = "dynamic-modules")]
fn module_symbol_name(module: &str) -> Vec<u8> {
    // Mech-facing module names keep hyphens. Rust/OS symbol names may not.
    //
    // Rule:
    //   "/" is not expected in module names
    //   "-" becomes "__"
    //   append NUL for libloading
    let escaped = module.replace('-', "__");
    format!("mech_module_{escaped}\0").into_bytes()
}

#[cfg(feature = "dynamic-modules")]
fn commit_dynamic_registrar(
    fxns: &mut Functions,
    registrar: &DynamicModuleRegistrar,
) -> MResult<ModuleManifest> {
    for compiler in registrar.compilers() {
        fxns.insert_function_compiler(
            compiler.name,
            Arc::new(StaticNativeFunctionCompiler::new(compiler.ptr)),
        );
    }

    let items = registrar.items().to_vec();

    if items.is_empty() {
        return Err(DynamicModuleLoader::dynamic_error(format!(
            "dynamic module `{}` registered no items",
            registrar.module()
        )));
    }

    Ok(ModuleManifest {
        module: registrar.module().to_string(),
        items,
    })
}

#[cfg(feature = "dynamic-modules")]
impl ModuleLoader for DynamicModuleLoader {
    fn can_load(&self, module: &str) -> bool {
        let candidates = self.candidate_paths(module);
        let can_load = candidates.iter().any(|candidate| candidate.exists());
        dynamic_trace(format!(
            "can_load `{module}` = {can_load}; candidates = {:?}",
            candidates
        ));
        can_load
    }

    fn load(&self, fxns: &mut Functions, module: &str) -> MResult<ModuleManifest> {
        dynamic_trace(format!("loading module `{module}`"));
        let candidates = self.candidate_paths(module);
        let library_path = candidates
            .iter()
            .find(|candidate| candidate.exists())
            .cloned()
            .ok_or_else(|| {
                Self::dynamic_error(format!(
                    "dynamic module library not found for module `{module}`; searched: {}",
                    candidates
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;

        dynamic_trace(format!("opening library `{}`", library_path.display()));
        let library = unsafe { libloading::Library::new(&library_path) }.map_err(|err| {
            Self::dynamic_error(format!(
                "dynamic module library failed to open for module `{module}` at `{}`: {err}",
                library_path.display()
            ))
        })?;

        let symbol_name = module_symbol_name(module);
        let symbol_label =
            String::from_utf8_lossy(symbol_name.strip_suffix(&[0]).unwrap_or(&symbol_name));
        dynamic_trace(format!("loading declaration symbol `{symbol_label}`"));
        let declaration_symbol = unsafe {
            library.get::<*const DynamicModuleDeclaration>(&symbol_name)
        }
        .map_err(|err| {
            Self::dynamic_error(format!(
                "dynamic module declaration symbol missing for module `{module}` in `{}`: {err}",
                library_path.display()
            ))
        })?;

        let declaration = unsafe { &**declaration_symbol };
        dynamic_trace(format!(
            "declaration loaded: module=`{}`, abi={}",
            declaration.module, declaration.abi_version
        ));

        if declaration.abi_version != MECH_DYNAMIC_MODULE_ABI_VERSION {
            return Err(Self::dynamic_error(format!(
                "dynamic module ABI version mismatch for module `{module}` in `{}`: host expected {}, module declared {}",
                library_path.display(),
                MECH_DYNAMIC_MODULE_ABI_VERSION,
                declaration.abi_version
            )));
        }

        if declaration.module != module {
            return Err(Self::dynamic_error(format!(
                "dynamic module declaration mismatch for requested module `{module}` in `{}`: declaration is for `{}`",
                library_path.display(),
                declaration.module
            )));
        }

        dynamic_trace(format!("registering module `{module}`"));
        let mut registrar = DynamicModuleRegistrar::new(module);
        unsafe { (declaration.register)(&mut registrar) }?;
        let manifest = commit_dynamic_registrar(fxns, &registrar)?;
        dynamic_trace(format!(
            "registered module `{module}` items: {:?}",
            manifest.items
        ));

        dynamic_trace(format!("storing library handle for module `{module}`"));
        DYNAMIC_MODULE_LIBRARIES
            .get_or_init(|| Mutex::new(Vec::new()))
            .lock()
            .map_err(|err| {
                Self::dynamic_error(format!(
                    "dynamic module library handle store lock failed for module `{module}`: {err}"
                ))
            })?
            .push(library);

        dynamic_trace(format!("returning manifest for module `{module}`"));
        Ok(manifest)
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
        Self::new().with_loader(Box::new(LinkedModuleLoader::default()))
    }

    pub fn default_loaders() -> Self {
        let registry = Self::new().with_loader(Box::new(LinkedModuleLoader::default()));

        #[cfg(feature = "dynamic-modules")]
        let registry = {
            dynamic_trace("dynamic module loader enabled");
            registry.with_loader(Box::new(DynamicModuleLoader::from_env()))
        };

        registry
    }

    pub fn load(&self, fxns: &mut Functions, module: &str) -> MResult<ModuleManifest> {
        for loader in &self.loaders {
            if loader.can_load(module) {
                return loader.load(fxns, module);
            }
        }

        #[cfg(feature = "dynamic-modules")]
        dynamic_trace(format!("no loader claimed module `{module}`"));

        Err(missing_module_error(module))
    }
}

pub fn load_module(fxns: &mut Functions, module: &str) -> MResult<ModuleManifest> {
    ModuleRegistry::default_loaders().load(fxns, module)
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
        return Err(missing_module_item_error(module, item));
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
        Err(missing_alias_error(&qualified_name, local_name))
    }
}
