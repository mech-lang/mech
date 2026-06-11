use crate::*;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ModuleExports {
    pub module: &'static str,
    pub exports: Vec<&'static str>,
}

const MATH_EXPORTS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "asinh", "acosh",
    "atanh", "cot", "sec", "csc", "acot", "asec", "acsc", "sqrt",
];
const STATS_EXPORTS: &[&str] = &["sum/column"];
const IO_EXPORTS: &[&str] = &["print", "println"];
const STRING_EXPORTS: &[&str] = &["concat"];
const COMBINATORICS_EXPORTS: &[&str] = &["n-choose-k"];

fn public_exports(module: &str) -> Option<ModuleExports> {
    match module {
        "math" => Some(ModuleExports {
            module: "math",
            exports: MATH_EXPORTS.to_vec(),
        }),
        "stats" => Some(ModuleExports {
            module: "stats",
            exports: STATS_EXPORTS.to_vec(),
        }),
        "io" => Some(ModuleExports {
            module: "io",
            exports: IO_EXPORTS.to_vec(),
        }),
        "string" => Some(ModuleExports {
            module: "string",
            exports: STRING_EXPORTS.to_vec(),
        }),
        "combinatorics" => Some(ModuleExports {
            module: "combinatorics",
            exports: COMBINATORICS_EXPORTS.to_vec(),
        }),
        _ => None,
    }
}

fn is_prelude_name(name: &str) -> bool {
    module_export_for_name(name).is_none()
}

fn module_export_for_name(name: &str) -> Option<(&'static str, &'static str)> {
    let exact = match name {
        "math/sin" => Some(("math", "sin")),
        "math/cos" => Some(("math", "cos")),
        "math/tan" => Some(("math", "tan")),
        "math/asin" => Some(("math", "asin")),
        "math/acos" => Some(("math", "acos")),
        "math/atan" => Some(("math", "atan")),
        "math/atan2" => Some(("math", "atan2")),
        "math/sinh" => Some(("math", "sinh")),
        "math/cosh" => Some(("math", "cosh")),
        "math/tanh" => Some(("math", "tanh")),
        "math/asinh" => Some(("math", "asinh")),
        "math/acosh" => Some(("math", "acosh")),
        "math/atanh" => Some(("math", "atanh")),
        "math/cot" => Some(("math", "cot")),
        "math/sec" => Some(("math", "sec")),
        "math/csc" => Some(("math", "csc")),
        "math/acot" => Some(("math", "acot")),
        "math/asec" => Some(("math", "asec")),
        "math/acsc" => Some(("math", "acsc")),
        "math/sqrt" => Some(("math", "sqrt")),
        "stats/sum/column" => Some(("stats", "sum/column")),
        "io/print" => Some(("io", "print")),
        "io/println" => Some(("io", "println")),
        "string/concat" => Some(("string", "concat")),
        "combinatorics/n_choose_k" | "combinatorics/n-choose-k" => {
            Some(("combinatorics", "n-choose-k"))
        }
        "MathSin" => Some(("math", "sin")),
        "MathCos" => Some(("math", "cos")),
        "MathTan" => Some(("math", "tan")),
        "MathAsin" => Some(("math", "asin")),
        "MathAcos" => Some(("math", "acos")),
        "MathAtan" => Some(("math", "atan")),
        "MathAtan2" => Some(("math", "atan2")),
        "MathSinh" => Some(("math", "sinh")),
        "MathCosh" => Some(("math", "cosh")),
        "MathTanh" => Some(("math", "tanh")),
        "MathAsinh" => Some(("math", "asinh")),
        "MathAcosh" => Some(("math", "acosh")),
        "MathAtanh" => Some(("math", "atanh")),
        "MathCot" => Some(("math", "cot")),
        "MathSec" => Some(("math", "sec")),
        "MathCsc" => Some(("math", "csc")),
        "MathAcot" => Some(("math", "acot")),
        "MathAsec" => Some(("math", "asec")),
        "MathAcsc" => Some(("math", "acsc")),
        "MathSqrt" => Some(("math", "sqrt")),
        "StatsSumColumn" => Some(("stats", "sum/column")),
        "IoPrint" => Some(("io", "print")),
        "IoPrintln" => Some(("io", "println")),
        "StringConcat" => Some(("string", "concat")),
        _ => None,
    };
    if exact.is_some() {
        return exact;
    }
    const MATH_PREFIXES: &[(&str, (&str, &str))] = &[
        ("MathAtan2", ("math", "atan2")),
        ("MathAsinh", ("math", "asinh")),
        ("MathAcosh", ("math", "acosh")),
        ("MathAtanh", ("math", "atanh")),
        ("MathSinh", ("math", "sinh")),
        ("MathCosh", ("math", "cosh")),
        ("MathTanh", ("math", "tanh")),
        ("MathAcot", ("math", "acot")),
        ("MathAsec", ("math", "asec")),
        ("MathAcsc", ("math", "acsc")),
        ("MathAsin", ("math", "asin")),
        ("MathAcos", ("math", "acos")),
        ("MathAtan", ("math", "atan")),
        ("MathSin", ("math", "sin")),
        ("MathCos", ("math", "cos")),
        ("MathTan", ("math", "tan")),
        ("MathCot", ("math", "cot")),
        ("MathSec", ("math", "sec")),
        ("MathCsc", ("math", "csc")),
        ("MathSqrt", ("math", "sqrt")),
    ];
    if let Some((_, export)) = MATH_PREFIXES
        .iter()
        .find(|(prefix, _)| name.starts_with(prefix))
    {
        return Some(*export);
    }
    if name.starts_with("StatsSumColumn") {
        return Some(("stats", "sum/column"));
    }
    if name.starts_with("NChooseK") {
        return Some(("combinatorics", "n-choose-k"));
    }
    if name.starts_with("IoPrintln") {
        return Some(("io", "println"));
    }
    if name.starts_with("IoPrint") {
        return Some(("io", "print"));
    }
    if name.starts_with("StringConcat") {
        return Some(("string", "concat"));
    }
    None
}

fn canonical_qualified_name(module: &str, export: &str) -> String {
    format!("{module}/{export}")
}

pub fn load_prelude(fxns: &mut Functions) {
    for fxn_desc in inventory::iter::<FunctionDescriptor> {
        if is_prelude_name(fxn_desc.name) {
            fxns.insert_function(fxn_desc.clone());
        }
    }
    for fxn_comp in inventory::iter::<FunctionCompilerDescriptor> {
        if is_prelude_name(fxn_comp.name) {
            fxns.insert_function_compiler(
                fxn_comp.name,
                Arc::new(StaticNativeFunctionCompiler::new(fxn_comp.ptr)),
            );
        }
    }
}

pub fn load_module(fxns: &mut Functions, module: &str) -> MResult<ModuleExports> {
    let exports = public_exports(module).ok_or_else(|| {
        MechError::new(
            MissingFunctionError {
                function_id: hash_str(module),
            },
            None,
        )
        .with_compiler_loc()
    })?;
    for fxn_desc in inventory::iter::<FunctionDescriptor> {
        if let Some((desc_module, desc_export)) = module_export_for_name(fxn_desc.name) {
            if desc_module == exports.module {
                fxns.functions.insert(hash_str(fxn_desc.name), fxn_desc.ptr);
                fxns.dictionary
                    .borrow_mut()
                    .insert(hash_str(fxn_desc.name), fxn_desc.name.to_string());
            }
        }
    }
    for fxn_comp in inventory::iter::<FunctionCompilerDescriptor> {
        if let Some((desc_module, desc_export)) = module_export_for_name(fxn_comp.name) {
            if desc_module == exports.module {
                let qualified = canonical_qualified_name(desc_module, desc_export);
                fxns.insert_function_compiler(
                    qualified,
                    Arc::new(StaticNativeFunctionCompiler::new(fxn_comp.ptr)),
                );
            }
        }
    }
    Ok(exports)
}

pub fn import_module_qualified(fxns: &mut Functions, module: &str) -> MResult<ModuleExports> {
    load_module(fxns, module)
}

pub fn import_module_item(fxns: &mut Functions, module: &str, item: &str) -> MResult<()> {
    let exports = load_module(fxns, module)?;
    if !exports.exports.contains(&item) {
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
    let exports = load_module(fxns, module)?;
    for item in exports.exports.iter() {
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

    if !found {
        if let Some(ptr) = fxns.functions.get(&qualified_id).copied() {
            fxns.functions.insert(local_id, ptr);
            fxns.dictionary
                .borrow_mut()
                .insert(local_id, local_name.to_string());
            found = true;
        }
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
