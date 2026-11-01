//! The shipped examples are parser inputs, not snapshots to rewrite when parsing changes.
#![cfg(all(feature = "mika", feature = "invariant_define", feature = "f32"))]

use std::fs;
use std::path::{Path, PathBuf};

use mech_syntax::document::{
    AstNode, CodeBlockSyntax, CodeFenceScope, DocumentId, DocumentStream, DocumentSyntax,
    ParseConfig, Revision, StreamProgress, SyntaxKind, SyntaxNode, TextRange, TextSize,
    TextSnapshot, parse_canonical_document, reconstruct_source, validate_lossless,
};

fn source_files(root: &Path, output: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("{}: {error}", root.display()))
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            source_files(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "mec") {
            output.push(path);
        }
    }
}

fn field_key(line: &str) -> Option<&str> {
    let (key, _) = line.split_once(':')?;
    let key = key.trim();
    (!key.is_empty()
        && key
            .chars()
            .all(|character| character.is_alphanumeric() || character == '-'))
    .then_some(key)
}

fn delimiter(line: &str) -> bool {
    let line = line.trim();
    line.len() >= 3 && line.chars().all(|character| character == '=')
}

// Check obvious program introducers independently of parser success: accepting
// an assignment or import as fallback prose must not make this corpus green.
// Quoted text, displayed code and comments are deliberately outside this check.
fn program_marker(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("%%") {
        return None;
    }
    if trimmed.starts_with("+>") || trimmed.starts_with("<+") {
        return Some(line.len() - trimmed.len());
    }
    let mut quote = None;
    let mut escaped = false;
    for (offset, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quote.is_some() && character == '\\' {
            escaped = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            }
            continue;
        }
        if matches!(character, '"' | '`') {
            quote = Some(character);
            continue;
        }
        let tail = &line[offset..];
        if tail.starts_with("--") || tail.starts_with("//") {
            return None;
        }
        if tail.starts_with(":=") || tail.starts_with("<-") {
            return Some(offset);
        }
    }
    None
}

fn source_roles(node: SyntaxNode, program: &mut Vec<TextRange>, displayed: &mut Vec<TextRange>) {
    if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
        if !matches!(
            fence.info().map(|info| info.scope),
            Some(CodeFenceScope::Root | CodeFenceScope::Named(_))
        ) {
            displayed.push(node.range());
            return;
        }
    }
    match node.kind() {
        SyntaxKind::MechCode | SyntaxKind::ModuleImport | SyntaxKind::EvalInlineMechCode => {
            program.push(node.range());
        }
        SyntaxKind::InlineMechCode => {
            displayed.push(node.range());
            return;
        }
        _ => {}
    }
    for child in node.children() {
        source_roles(child, program, displayed);
    }
}

fn audit(path: &Path) -> Vec<String> {
    let text = fs::read_to_string(path).unwrap();
    let started = std::time::Instant::now();
    let snapshot = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x5a17), Revision(0), text.as_str()).unwrap(),
        ParseConfig::default(),
    );
    let parse_elapsed = started.elapsed();
    let mut errors = Vec::new();
    // The complete-file path defers tree construction; the retained path must
    // still publish completed subtrees. Compare both against the authored corpus.
    let mut stream = DocumentStream::new(DocumentId(0x5a17), ParseConfig::default());
    stream.append(&text, 0).unwrap();
    if stream.finish(u64::MAX).progress != StreamProgress::Finished {
        errors.push("retained parse did not finish".to_owned());
    } else {
        let retained = stream.materialize().unwrap();
        if retained.root.structural_hash != snapshot.root.structural_hash
            || retained.stats != snapshot.stats
            || retained.is_strictly_clean() != snapshot.is_strictly_clean()
        {
            errors.push("complete-file and retained parsing disagree".to_owned());
        }
    }
    if !snapshot.is_strictly_clean() {
        let diagnostics = snapshot
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let position = diagnostic
                    .primary
                    .resolve(snapshot.revision, &snapshot.nodes)
                    .map(|range| {
                        let before = &text[..range.start.0 as usize];
                        let row = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
                        let column = before.rsplit('\n').next().unwrap().chars().count() + 1;
                        format!("{row}:{column}")
                    })
                    .unwrap_or_else(|| "unpositioned".to_owned());
                format!("{position} {}: {}", diagnostic.code.0, diagnostic.message)
            })
            .collect::<Vec<_>>()
            .join("\n");
        errors.push(format!("strict parse failed:\n{diagnostics}"));
    }
    if let Err(error) = validate_lossless(&snapshot.root, &snapshot.source) {
        errors.push(format!("lossless tree validation failed: {error:?}"));
    }
    if reconstruct_source(&snapshot.root, &snapshot.source)
        .ok()
        .as_deref()
        != Some(&text)
    {
        errors.push("tree does not reconstruct the original source bytes".to_owned());
    }
    let document = DocumentSyntax::cast(snapshot.syntax()).expect("canonical document root");
    let mut program = Vec::new();
    let mut displayed = Vec::new();
    source_roles(document.syntax().clone(), &mut program, &mut displayed);
    let mut lines = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        lines.push((offset, line.trim_end_matches(['\r', '\n'])));
        offset += line.len();
    }
    for (row, (start, line)) in lines.iter().enumerate() {
        let Some(marker) = program_marker(line) else {
            continue;
        };
        let at = TextSize((*start + marker) as u32);
        if !displayed.iter().any(|range| range.contains(at))
            && !program.iter().any(|range| range.contains(at))
        {
            errors.push(format!(
                "line {}: program source became prose: {line}",
                row + 1
            ));
        }
    }
    if lines.len() >= 2 && delimiter(lines[1].1) {
        match document.title() {
            Some(title) => {
                if title.syntax().text().unwrap().lines().next() != Some(lines[0].1) {
                    errors.push("authored title is not the document title".to_owned());
                }
                if lines.get(2).is_some_and(|(_, line)| {
                    line.trim_start().starts_with("+>") || field_key(line).is_some()
                }) {
                    let closing = lines
                        .iter()
                        .enumerate()
                        .skip(2)
                        .find(|(_, (_, line))| delimiter(line));
                    match (closing, title.front_matter()) {
                        (Some((end, _)), Some(front)) => {
                            let authored = &lines[2..end];
                            let keys = authored
                                .iter()
                                .filter_map(|(_, line)| field_key(line))
                                .collect::<Vec<_>>();
                            let retained = front
                                .keys()
                                .iter()
                                .map(|key| key.syntax().text().unwrap())
                                .collect::<Vec<_>>();
                            if retained != keys {
                                errors.push(format!("front-matter fields differ: source={keys:?}, parsed={retained:?}"));
                            }
                            let imports = authored
                                .iter()
                                .filter_map(|(start, line)| {
                                    let trimmed = line.trim_start();
                                    trimmed.starts_with("+>").then_some(TextSize(
                                        (start + line.len() - trimmed.len()) as u32,
                                    ))
                                })
                                .collect::<Vec<_>>();
                            let retained = front.imports();
                            if retained.len() != imports.len()
                                || imports.iter().any(|at| {
                                    !retained
                                        .iter()
                                        .any(|import| import.syntax().range().contains(*at))
                                })
                            {
                                errors.push(format!("front-matter imports lost their title ownership: source={}, parsed={}", imports.len(), retained.len()));
                            }
                        }
                        _ => errors.push(
                            "authored front-matter envelope is not retained as title metadata"
                                .to_owned(),
                        ),
                    }
                }
            }
            None => errors.push("authored title became document body".to_owned()),
        }
    }
    eprintln!(
        "{}: {} diagnostics, {} corpus contract failures, parse_ms={:.3}, parser_steps={}",
        path.display(),
        snapshot.diagnostics.len(),
        errors.len(),
        parse_elapsed.as_secs_f64() * 1000.0,
        snapshot.stats.parser_steps,
    );
    errors
}

#[test]
fn maintained_examples_parse_losslessly_with_authored_source_roles() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let mut paths = Vec::new();
    let mut examples = Vec::new();
    source_files(&repository.join("examples"), &mut examples);
    for path in examples {
        let relative = path.strip_prefix(repository.join("examples")).unwrap();
        let category = relative.components().next().unwrap().as_os_str();
        // examples/README.md marks aspirational programs as not yet working.
        // The prototype README explicitly uses an opt-in application normalizer
        // and says canonical parser integration for its surface is separate work.
        if category != "aspirational" && category != "function-application-prototype" {
            paths.push(path);
        }
    }
    source_files(&repository.join("hosts/robot-arm/examples"), &mut paths);
    assert!(
        paths
            .iter()
            .any(|path| path.ends_with("working/fizzbuzz.mec"))
    );
    let failures = paths
        .iter()
        .flat_map(|path| {
            audit(path)
                .into_iter()
                .map(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
