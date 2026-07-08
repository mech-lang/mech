use std::fs;
use std::path::{Path, PathBuf};

use colored::*;
use mech_core::*;
use mech_syntax::formatter::*;
use mech_syntax::parser;

use crate::cli::format_targets::{
    CollectedSourceTarget, FORMAT_EXTENSIONS, FormatTargetPlan, format_output_file_for_target,
};
use crate::fs_paths::{source_extension, unsupported_source_path_error};
use crate::{GenericError, MechError, save_to_file};

pub(crate) fn read_format_source(path: &Path) -> MResult<MechSourceCode> {
    let extension = source_extension(path)
        .ok_or_else(|| unsupported_source_path_error(path, FORMAT_EXTENSIONS))?;
    match extension.as_str() {
        "mec" | "🤖" | "mdoc" => Ok(MechSourceCode::String(std::fs::read_to_string(path)?)),
        "html" | "htm" => Ok(MechSourceCode::Html(std::fs::read_to_string(path)?)),
        _ => Err(unsupported_source_path_error(path, FORMAT_EXTENSIONS)),
    }
}

pub(crate) struct LoadedFormatSource {
    pub(crate) target: CollectedSourceTarget,
    pub(crate) source: MechSourceCode,
}

pub(crate) fn load_format_sources(
    targets: &[CollectedSourceTarget],
) -> MResult<Vec<LoadedFormatSource>> {
    targets
        .iter()
        .map(|target| {
            Ok(LoadedFormatSource {
                target: target.clone(),
                source: read_format_source(&target.path)?,
            })
        })
        .collect()
}

pub(crate) fn write_format_outputs(
    plan: &FormatTargetPlan,
    loaded_sources: Vec<LoadedFormatSource>,
    stylesheet_str: String,
    shim_str: String,
) -> MResult<()> {
    if !plan.is_output_file && plan.output_path != PathBuf::from(".") {
        fs::create_dir_all(&plan.output_path)?;
        println!(
            "{} Directory created: {}",
            "[Created]".truecolor(153, 221, 85),
            plan.output_path.display()
        );
    }

    if plan.html {
        let mut html_items: Vec<(CollectedSourceTarget, String)> = Vec::new();
        for LoadedFormatSource { target, source } in &loaded_sources {
            let html = match source {
                MechSourceCode::Html(content) => content.clone(),
                MechSourceCode::String(source) => {
                    let tree = parser::parse(source.trim())?;
                    let mut formatter = Formatter::new();
                    formatter.format_html(&tree, stylesheet_str.clone(), shim_str.clone())
                }
                other => {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "Unsupported source kind for HTML formatting `{}`: {:?}",
                                target.path.display(),
                                other
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            };
            html_items.push((target.clone(), html));
        }
        if plan.is_output_file && html_items.len() == 1 {
            let (_, content) = html_items.remove(0);
            save_to_file(plan.output_path.clone(), &content)?;
        } else {
            for (target, content) in html_items {
                let output_file = format_output_file_for_target(
                    &target,
                    &plan.output_path,
                    plan.is_output_file,
                    plan.writes_in_place,
                    true,
                );
                save_to_file(output_file, &content)?;
            }
        }
    } else {
        for LoadedFormatSource { target, source } in loaded_sources {
            let content = match source {
                MechSourceCode::String(source) => {
                    let tree = parser::parse(source.trim())?;
                    let mut formatter = Formatter::new();
                    formatter.format(&tree)
                }
                MechSourceCode::Html(content) => content,
                other => {
                    return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "Unsupported source kind for raw formatting `{}`: {:?}",
                                target.path.display(),
                                other
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc());
                }
            };
            let output_file = format_output_file_for_target(
                &target,
                &plan.output_path,
                plan.is_output_file,
                plan.writes_in_place,
                false,
            );
            save_to_file(output_file, &content)?;
        }
    }

    Ok(())
}
