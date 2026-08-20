use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};
use mech_runtime::RuntimeValueSnapshot;
use mech_runtime::{
    OutputContent, TableOutput, TextOutput, ValueOutput, terminal_display_capabilities,
};

use crate::cli::host_grants::EffectiveCliHostGrants;

static DOCS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/docs");

pub(super) const MECH_AMBER: (u8, u8, u8) = (246, 192, 78);

pub(super) fn value(value: &RuntimeValueSnapshot) -> Option<OutputContent> {
    if !value.is_empty() {
        return Some(OutputContent::Value(ValueOutput::new(
            value.kind().to_string(),
            value.format_canonical_inline(),
        )));
    }
    None
}

pub(super) fn capabilities(grants: &EffectiveCliHostGrants) -> OutputContent {
    let mut rows = vec![
        capability_row("cli/env", "read", &grants.env_read_paths),
        capability_row("cli/stdout", "write", &grants.stdout_write_paths),
        capability_row("cli/stderr", "write", &grants.stderr_write_paths),
    ];
    rows.extend(
        terminal_display_capabilities()
            .into_iter()
            .map(|capability| {
                vec![
                    capability.name,
                    "render".to_string(),
                    capability.support.to_string(),
                    capability.fallback.unwrap_or_else(|| "—".to_string()),
                ]
            }),
    );
    OutputContent::Table(TableOutput::new(
        vec![
            "Context".to_string(),
            "Operation".to_string(),
            "Status".to_string(),
            "Paths / fallback".to_string(),
        ],
        rows,
    ))
}

fn capability_row(context: &str, operation: &str, paths: &[String]) -> Vec<String> {
    let granted = !paths.is_empty();
    vec![
        context.to_string(),
        operation.to_string(),
        if granted { "granted" } else { "denied" }.to_string(),
        if granted {
            paths.join(", ")
        } else {
            "—".to_string()
        },
    ]
}

pub(super) fn profiling(requested: Option<bool>) -> OutputContent {
    let requested = match requested {
        Some(true) => "on",
        Some(false) => "off",
        None => "status only",
    };
    let rows = vec![
        vec!["Status".to_string(), "unavailable".to_string()],
        vec!["Requested".to_string(), requested.to_string()],
        vec![
            "Reason".to_string(),
            "the resident runtime does not expose a profiling control or report API".to_string(),
        ],
    ];
    OutputContent::Table(TableOutput::new(
        vec!["Property".to_string(), "Value".to_string()],
        rows,
    ))
}

pub(super) fn list_directory(path: Option<&str>) -> io::Result<(PathBuf, OutputContent)> {
    let requested = match path {
        Some(path) => PathBuf::from(path),
        None => env::current_dir()?,
    };
    let display_path = requested
        .canonicalize()
        .unwrap_or_else(|_| requested.clone());
    let mut entries = Vec::new();

    for entry in fs::read_dir(&requested)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let metadata = entry.metadata().ok();
        let kind = if file_type.is_dir() {
            "directory"
        } else if file_type.is_symlink() {
            "symlink"
        } else if file_type.is_file() {
            "file"
        } else {
            "other"
        };
        let size = if file_type.is_file() {
            metadata
                .as_ref()
                .map(|metadata| format_bytes(metadata.len()))
                .unwrap_or_else(|| "?".to_string())
        } else {
            "—".to_string()
        };
        entries.push((
            !file_type.is_dir(),
            entry.file_name().to_string_lossy().into_owned(),
            kind.to_string(),
            size,
        ));
    }

    entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let rows = entries
        .into_iter()
        .map(|(_, name, kind, size)| vec![name, kind, size])
        .collect::<Vec<_>>();
    let content = if rows.is_empty() {
        OutputContent::Text(TextOutput::new("(empty directory)"))
    } else {
        OutputContent::Table(TableOutput::new(
            vec!["Name".to_string(), "Type".to_string(), "Size".to_string()],
            rows,
        ))
    };
    Ok((display_path, content))
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub(super) fn save_session_source(path: &Path, source: &str) -> io::Result<()> {
    if path.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "save path cannot be empty",
        ));
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if !parent.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("parent directory `{}` does not exist", parent.display()),
            ));
        }
    }
    fs::write(path, source)
}

#[derive(Clone)]
struct EmbeddedDoc {
    path: String,
    topic: String,
    content: String,
}

fn embedded_docs() -> Result<Vec<EmbeddedDoc>, String> {
    let entries = DOCS_DIR
        .find("**/*.mec")
        .map_err(|error| format!("Unable to index documentation: {error}"))?;
    let mut docs = entries
        .filter_map(|entry| entry.as_file())
        .filter_map(|file| {
            let content = file.contents_utf8()?;
            let path = file.path().to_string_lossy().replace('\\', "/");
            let topic = path.strip_suffix(".mec").unwrap_or(&path).to_string();
            Some(EmbeddedDoc {
                path,
                topic,
                content: content.to_string(),
            })
        })
        .collect::<Vec<_>>();
    docs.sort_by(|left, right| left.topic.cmp(&right.topic));
    Ok(docs)
}

pub(super) fn docs(name: Option<String>) -> String {
    let docs = match embedded_docs() {
        Ok(docs) => docs,
        Err(error) => return error,
    };
    if docs.is_empty() {
        return "No embedded documentation is available in this build.".to_string();
    }

    let Some(name) = name else {
        let mut categories = BTreeMap::<String, usize>::new();
        for doc in &docs {
            let category = doc
                .topic
                .split('/')
                .next()
                .unwrap_or("uncategorized")
                .to_string();
            *categories.entry(category).or_default() += 1;
        }
        let rows = categories
            .into_iter()
            .map(|(category, count)| vec![category, count.to_string()])
            .collect::<Vec<_>>();
        return format!(
            "{}\n\nUse `:docs <topic>` to search document names and contents.",
            render_table(&["Category", "Documents"], &rows)
        );
    };

    let query = name.trim().to_ascii_lowercase();
    let normalized_query = query.trim_end_matches(".mec");
    let exact = docs
        .iter()
        .filter(|doc| {
            let topic = doc.topic.to_ascii_lowercase();
            let stem = Path::new(&doc.path)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            topic == normalized_query || stem == normalized_query
        })
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return render_doc(exact[0]);
    }

    let mut matches = docs
        .iter()
        .filter(|doc| {
            doc.topic.to_ascii_lowercase().contains(normalized_query)
                || doc.content.to_ascii_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.topic.cmp(&right.topic));
    if matches.len() == 1 {
        return render_doc(matches[0]);
    }
    if matches.is_empty() {
        return format!("No embedded documentation matched `{name}`.");
    }

    let rows = matches
        .into_iter()
        .map(|doc| vec![doc.topic.clone(), doc.path.clone()])
        .collect::<Vec<_>>();
    format!(
        "Multiple documents matched `{name}`:\n{}\n\nUse a more specific topic or path.",
        render_table(&["Topic", "Path"], &rows)
    )
}

fn render_doc(doc: &EmbeddedDoc) -> String {
    format!("{}\n\n{}", doc.path, doc.content.trim_end())
}

pub(super) fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    if headers.is_empty() {
        return String::new();
    }
    let columns = headers.len();
    let normalized_rows = rows
        .iter()
        .map(|row| {
            (0..columns)
                .map(|index| sanitize_cell(row.get(index).map(String::as_str).unwrap_or("")))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let normalized_headers = headers
        .iter()
        .map(|header| sanitize_cell(header))
        .collect::<Vec<_>>();
    let mut widths = normalized_headers
        .iter()
        .map(|header| header.chars().count())
        .collect::<Vec<_>>();
    for row in &normalized_rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.chars().count());
        }
    }

    let mut output = String::new();
    output.push_str(&table_row(&normalized_headers, &widths));
    for row in &normalized_rows {
        output.push('\n');
        output.push_str(&table_row(row, &widths));
    }
    output
}

fn sanitize_cell(value: &str) -> String {
    value
        .replace('\r', "")
        .replace('\n', " ↩ ")
        .replace('\t', "  ")
}

fn table_row(cells: &[String], widths: &[usize]) -> String {
    let mut row = String::new();
    for (index, (cell, width)) in cells.iter().zip(widths).enumerate() {
        if index > 0 {
            row.push_str("  ");
        }
        row.push_str(cell);
        if index + 1 < cells.len() {
            row.push_str(&" ".repeat(width.saturating_sub(cell.chars().count())));
        }
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{LegacyValue, matrix::Matrix};

    #[test]
    fn table_renderer_keeps_columns_aligned_without_box_drawing() {
        let table = render_table(
            &["Name", "Type"],
            &[
                vec!["x".to_string(), "f64".to_string()],
                vec!["long-name".to_string(), "[f64]:2,2".to_string()],
            ],
        );
        assert_eq!(
            table,
            "Name       Type\nx          f64\nlong-name  [f64]:2,2"
        );
        assert!(
            !table
                .chars()
                .any(|character| "┌┬┐├┼┤└┴┘│─".contains(character))
        );
    }

    #[test]
    fn help_capabilities_and_docs_are_structured() {
        let grants = EffectiveCliHostGrants {
            env_read_paths: vec!["*".to_string()],
            stdout_write_paths: vec!["text".to_string()],
            stderr_write_paths: Vec::new(),
        };
        let capabilities = rendered(capabilities(&grants));
        assert!(capabilities.contains("cli/env"));
        assert!(capabilities.contains("granted"));
        assert!(capabilities.contains("denied"));
        assert!(capabilities.contains("display.scene.3d"));

        let docs = docs(Some("capabilities".to_string()));
        assert!(docs.to_ascii_lowercase().contains("capabil"));
    }

    #[test]
    fn value_payloads_use_canonical_text_for_primary_and_plain_consumers() {
        let snapshot = RuntimeValueSnapshot::try_from(LegacyValue::MatrixString(Matrix::from_vec(
            vec!["a\"b".to_string(), "c\\d\nnext".to_string()],
            1,
            2,
        )))
        .unwrap();
        let OutputContent::Value(value) = value(&snapshot).unwrap() else {
            panic!("expected value content");
        };
        assert_eq!(value.text, "[\"a\\\"b\" \"c\\\\d\\nnext\"]");
        assert_eq!(value.inline_text, value.text);
    }

    fn rendered(content: OutputContent) -> String {
        match content {
            OutputContent::Text(text) => text.text,
            OutputContent::Table(table) => {
                let headers = table.columns.iter().map(String::as_str).collect::<Vec<_>>();
                render_table(&headers, &table.rows)
            }
            other => panic!("unexpected content: {other:?}"),
        }
    }
}
