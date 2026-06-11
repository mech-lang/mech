use crate::*;
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
