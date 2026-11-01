//! Canonical-source compilation for general Boolean grid maps. Kernel bodies
//! are subsequently compiled by the maintained canonical artifact compiler.
use super::{text, visit};
use mech_syntax::document::{
    AnyCallArgumentSyntax, AstNode, FunctionCallSyntax, SyntaxKind, SyntaxNode,
    VariableDefineSyntax,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub enum DimensionSource {
    Argument(usize),
    Constant(u64),
}
pub struct GridProgram {
    pub kernel_source: String,
    pub width: DimensionSource,
    pub height: DimensionSource,
    pub lanes: usize,
    pub inputs: [String; 3],
}
fn arguments(node: &SyntaxNode, name: &str) -> Result<Vec<SyntaxNode>, String> {
    let mut calls = Vec::new();
    visit(node, SyntaxKind::FunctionCall, &mut calls);
    let call = calls
        .into_iter()
        .find(|n| {
            FunctionCallSyntax::cast(n.clone())
                .and_then(|c| c.function())
                .is_some_and(|f| text(f.syntax()).ok().as_deref() == Some(name))
        })
        .ok_or_else(|| format!("expected {name}"))?;
    if text(node)?.trim() != text(&call)?.trim() {
        return Err(format!("expected a direct {name} call"));
    }
    let call = FunctionCallSyntax::cast(call).unwrap();
    call.arguments()
        .ok_or("missing arguments")?
        .arguments()
        .into_iter()
        .map(|a| match a {
            AnyCallArgumentSyntax::Positional(a) => {
                Ok(a.value().ok_or("missing value")?.syntax().clone())
            }
            _ => Err("grid calls require positional arguments".into()),
        })
        .collect()
}
fn integer(node: &SyntaxNode) -> Result<u64, String> {
    text(node)?
        .trim()
        .parse()
        .map_err(|_| "expected u64 constant".into())
}
fn dimension(
    node: &SyntaxNode,
    vars: &BTreeMap<String, DimensionSource>,
) -> Result<DimensionSource, String> {
    let source = text(node)?;
    let source = source.trim();
    if let Some(d) = vars.get(source) {
        return Ok(*d);
    }
    Ok(DimensionSource::Constant(source.parse().map_err(
        |_| "grid dimensions require a constant or argument binding",
    )?))
}
fn parameter(node: &SyntaxNode) -> Result<(String, String), String> {
    let name = node
        .children()
        .find(|n| n.kind() == SyntaxKind::Identifier)
        .ok_or("missing parameter name")?;
    let annotation = node
        .children()
        .find(|n| n.kind() == SyntaxKind::KindAnnotation)
        .ok_or("missing parameter kind")?;
    Ok((text(&name)?, text(&annotation)?.replace(' ', "")))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputValue {
    Grid,
    Encoded,
}
/// Coordinator bindings have the same meaning whether written as nested calls,
/// explicit aliases, or application chains. Only the general grid/image values
/// supported by this frontend are admitted; kernel selection remains unchanged.
fn output_value(
    node: &SyntaxNode,
    bindings: &BTreeMap<String, OutputValue>,
) -> Result<OutputValue, String> {
    let source = text(node)?;
    if let Some(value) = bindings.get(source.trim()) {
        return Ok(*value);
    }
    let encoded = arguments(node, "image/pbm")?;
    if encoded.len() != 1 || output_value(&encoded[0], bindings)? != OutputValue::Grid {
        return Err("image/pbm requires the grid image".into());
    }
    Ok(OutputValue::Encoded)
}
pub fn compile(source: &str, operations: &[&str]) -> Result<GridProgram, String> {
    compile_parsed(&super::ParsedSource::parse(source)?, operations)
}
pub fn compile_parsed(
    parsed: &super::ParsedSource,
    operations: &[&str],
) -> Result<GridProgram, String> {
    let syntax = &parsed.syntax;
    let mut roots = Vec::new();
    visit(syntax, SyntaxKind::MechCodeAlt, &mut roots);
    let mut variables = BTreeMap::new();
    let mut output_bindings = BTreeMap::new();
    let mut map = None;
    let mut write = false;
    let mut functions = BTreeMap::new();
    let mut definitions = Vec::new();
    visit(syntax, SyntaxKind::FunctionDefine, &mut definitions);
    for node in definitions {
        let name = node
            .children()
            .find(|n| n.kind() == SyntaxKind::Identifier)
            .ok_or("missing function name")?;
        if functions.insert(text(&name)?, node).is_some() {
            return Err("duplicate grid function".into());
        }
    }
    for root in roots {
        if root
            .children()
            .all(|child| child.kind() == SyntaxKind::Comment)
        {
            continue;
        }
        let mut defs = Vec::new();
        visit(&root, SyntaxKind::FunctionDefine, &mut defs);
        if !defs.is_empty() {
            continue;
        }
        let mut calls = Vec::new();
        visit(&root, SyntaxKind::FunctionCall, &mut calls);
        for call in &calls {
            let c = FunctionCallSyntax::cast(call.clone()).unwrap();
            let name = text(c.function().ok_or("missing function")?.syntax())?;
            if !operations.contains(&name.as_str()) {
                return Err(format!("unsupported grid coordinator operation {name}"));
            }
        }
        let mut defs = Vec::new();
        visit(&root, SyntaxKind::VariableDefine, &mut defs);
        if let Some(def) = defs.first().cloned().and_then(VariableDefineSyntax::cast) {
            if defs.len() != 1 || def.mutability_marker().is_some() {
                return Err("grid coordinator requires immutable definitions".into());
            }
            let name = text(def.name().ok_or("missing name")?.syntax())?;
            let value = def.value().ok_or("missing value")?;
            if variables.contains_key(&name) || output_bindings.contains_key(&name) {
                return Err("duplicate coordinator binding".into());
            }
            if calls
                .iter()
                .any(|c| text(c).is_ok_and(|s| s.starts_with("io/argument-u64(")))
            {
                let a = arguments(value.syntax(), "io/argument-u64")?;
                if a.len() != 1 {
                    return Err("argument accessor arity".into());
                }
                let index = integer(&a[0])?;
                if index == 0 || index > 32 {
                    return Err("argument index must be 1..32".into());
                }
                if variables
                    .insert(name, DimensionSource::Argument(index as usize))
                    .is_some()
                {
                    return Err("duplicate dimension binding".into());
                }
            } else if calls
                .iter()
                .any(|c| text(c).is_ok_and(|s| s.starts_with("grid/map-until(")))
            {
                if map.is_some() {
                    return Err("one grid map admitted per executable".into());
                }
                let a = arguments(value.syntax(), "grid/map-until")?;
                if a.len() != 3 {
                    return Err("grid/map-until arity".into());
                }
                output_bindings.insert(name.clone(), OutputValue::Grid);
                map = Some((
                    name,
                    dimension(&a[0], &variables)?,
                    dimension(&a[1], &variables)?,
                    text(&a[2])?.trim().to_string(),
                ));
            } else {
                if let Ok(d) = dimension(value.syntax(), &variables) {
                    variables.insert(name, d);
                } else {
                    let value = output_value(value.syntax(), &output_bindings)?;
                    output_bindings.insert(name, value);
                }
            }
        } else {
            let args = arguments(&root, "io/write")?;
            if args.len() != 1 || write {
                return Err("one grid output write required".into());
            }
            if output_value(&args[0], &output_bindings)? != OutputValue::Encoded {
                return Err("io/write requires an encoded grid image".into());
            }
            write = true;
        }
    }
    if !write {
        return Err("grid program requires io/write(image/pbm(image))".into());
    }
    let (_, width, height, kernel) = map.ok_or("missing grid map")?;
    let function = functions
        .get(&kernel)
        .ok_or("grid map requires a Mech-defined kernel")?;
    let inputs = function
        .children()
        .filter(|n| n.kind() == SyntaxKind::FunctionArg)
        .map(|n| parameter(&n))
        .collect::<Result<Vec<_>, _>>()?;
    if inputs.len() != 3 || inputs[0].1 != "<f64>" || inputs[2].1 != "<f64>" {
        return Err("grid kernel requires width, columns, row f64 inputs".into());
    }
    let lanes = if inputs[1].1 == "<f64>" {
        1
    } else {
        inputs[1]
            .1
            .strip_prefix("<[f64]:")
            .and_then(|s| s.strip_suffix(",1>"))
            .and_then(|s| s.parse::<usize>().ok())
            .ok_or("grid column parameter must be scalar or fixed column vector")?
    };
    if !(1..=64).contains(&lanes) {
        return Err("grid maps support 1..64 lanes".into());
    }
    let body = function
        .children()
        .find(|n| n.kind() == SyntaxKind::FunctionDefineStatements)
        .ok_or("grid kernel requires a statement body")?;
    let mut out = Vec::new();
    visit(
        &body
            .children()
            .find(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::FunctionOutArg | SyntaxKind::FunctionOutArgs
                )
            })
            .ok_or("missing output")?,
        SyntaxKind::FunctionArg,
        &mut out,
    );
    if out.len() != 1 || parameter(&out[0])?.1 != format!("<[bool]:1,{}>", lanes + 1) {
        return Err("grid kernel must return lane results followed by a stopping Boolean".into());
    }
    let column_kind = if lanes == 1 {
        "<f64>".to_string()
    } else {
        format!("<[f64]:{lanes},1>")
    };
    let all_functions = functions
        .values()
        .map(text)
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    let lane_outputs = (1..=lanes)
        .map(|i| format!("result[{i}]"))
        .collect::<Vec<_>>()
        .join(", ");
    let kernel_source = format!(
        "{all_functions}\nresult := {kernel}({}<f64>, {}{column_kind}, {}<f64>)\ninside := [{lane_outputs}]\ndone := result[{}]\n<+ inside\n<+ done\n",
        inputs[0].0,
        inputs[1].0,
        inputs[2].0,
        lanes + 1
    );
    Ok(GridProgram {
        kernel_source,
        width,
        height,
        lanes,
        inputs: [
            inputs[0].0.clone(),
            inputs[1].0.clone(),
            inputs[2].0.clone(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const OPERATIONS: &[&str] = &["grid/map-until", "image/pbm", "io/write", "io/argument-u64"];
    const KERNEL: &str = "pixels(width<f64>, column<f64>, row<f64>) = (result<[bool]:1,2>) := result := [true, true].\nsize := io/argument-u64(1)\nimage := grid/map-until(size, size, pixels)\n";
    #[test]
    fn nested_output_explicit_aliases_and_application_have_identical_kernel() {
        let nested = compile(&format!("{KERNEL}io/write(image/pbm(image))\n"), OPERATIONS).unwrap();
        for output in [
            "alias := image\nencoded := image/pbm(alias)\nencoded-alias := encoded\nio/write(encoded-alias)\n",
            "image |> image/pbm |> io/write\n",
        ] {
            let aliases = compile(&format!("{KERNEL}{output}"), OPERATIONS).unwrap();
            assert_eq!(aliases.kernel_source, nested.kernel_source);
            assert_eq!(aliases.lanes, nested.lanes);
            assert!(matches!(aliases.width, DimensionSource::Argument(1)));
            assert!(matches!(aliases.height, DimensionSource::Argument(1)));
        }
    }
    #[test]
    fn expression_function_and_dimension_aliases_use_existing_grid_graph() {
        let source = "pixels(width<f64>, column<f64>, row<f64>) => <[bool]:1,2> := [true, true]\nsize := io/argument-u64(1)\nwidth := size\nimage := grid/map-until(width, 12, pixels)\nimage |> image/pbm |> io/write\n";
        let program = compile(source, OPERATIONS).unwrap();
        assert!(matches!(program.width, DimensionSource::Argument(1)));
        assert!(matches!(program.height, DimensionSource::Constant(12)));
        assert_eq!(program.lanes, 1);
    }
    #[test]
    fn image_aliases_cannot_change_output_kind_or_reference_unknown_images() {
        for output in [
            "io/write(image)\n",
            "encoded := image/pbm(missing)\nio/write(encoded)\n",
            "encoded := image/pbm(image)\nagain := image/pbm(encoded)\nio/write(again)\n",
            "image := 3\nio/write(image/pbm(image))\n",
        ] {
            assert!(compile(&format!("{KERNEL}{output}"), OPERATIONS).is_err());
        }
    }
}
