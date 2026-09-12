use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const CONSUMER_HEADER: &str = "consumer-id\tpackage\tsource-path\tcaller-symbol\tcall-count\t\
feature-gate\tinput-policy\tfailure-policy\tcurrent-result-use\tcanonical-target\t\
cutover-action\tfixture-id\tnotes";
const FIXTURE_HEADER: &str = "fixture-id\tsource-path\tcanonical-root\tcanonical-outcome\t\
consumer-contracts\tnotes";
const EXPECTED_CONSUMERS: usize = 27;
const EXPECTED_PRODUCTION_CALLS: usize = 29;
const EXPECTED_FIXTURES: usize = 9;

#[derive(Debug)]
struct ConsumerRow {
    source_path: String,
    caller: String,
    calls: usize,
    feature_gate: String,
    input_policy: String,
    failure_policy: String,
    current_result_use: String,
    canonical_target: String,
    cutover_action: String,
    fixture: String,
    notes: String,
}

#[derive(Debug)]
struct FixtureRow {
    source_path: String,
    root: String,
    outcome: String,
    contracts: String,
    notes: String,
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

fn consumers() -> BTreeMap<String, ConsumerRow> {
    let path = "docs/design/grammar-audit/source-parser-consumers.tsv";
    let source = fs::read_to_string(repository_root().join(path)).expect("read consumer inventory");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(CONSUMER_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(row.len(), 13, "invalid consumer row {}", index + 2);
        assert!(row[0] > previous.as_str(), "consumer rows are not ordered");
        previous = row[0].to_owned();
        let calls = row[4]
            .parse::<usize>()
            .unwrap_or_else(|_| panic!("invalid call count on row {}", index + 2));
        assert!(calls > 0, "empty call count on row {}", index + 2);
        let value = ConsumerRow {
            source_path: row[2].to_owned(),
            caller: row[3].to_owned(),
            calls,
            feature_gate: row[5].to_owned(),
            input_policy: row[6].to_owned(),
            failure_policy: row[7].to_owned(),
            current_result_use: row[8].to_owned(),
            canonical_target: row[9].to_owned(),
            cutover_action: row[10].to_owned(),
            fixture: row[11].to_owned(),
            notes: row[12].to_owned(),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    assert_eq!(rows.len(), EXPECTED_CONSUMERS);
    rows
}

fn fixtures() -> BTreeMap<String, FixtureRow> {
    let path = "docs/design/grammar-audit/source-parser-consumer-fixtures.tsv";
    let source = fs::read_to_string(repository_root().join(path)).expect("read fixture inventory");
    let mut lines = source.lines();
    assert_eq!(lines.next(), Some(FIXTURE_HEADER));
    let mut previous = String::new();
    let mut rows = BTreeMap::new();
    for (index, line) in lines.enumerate() {
        let row = fields(line);
        assert_eq!(row.len(), 6, "invalid fixture row {}", index + 2);
        assert!(row[0] > previous.as_str(), "fixture rows are not ordered");
        previous = row[0].to_owned();
        let value = FixtureRow {
            source_path: row[1].to_owned(),
            root: row[2].to_owned(),
            outcome: row[3].to_owned(),
            contracts: row[4].to_owned(),
            notes: row[5].to_owned(),
        };
        assert!(rows.insert(row[0].to_owned(), value).is_none());
    }
    assert_eq!(rows.len(), EXPECTED_FIXTURES);
    rows
}

fn visit_rust_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    {
        let path = entry.expect("read source-tree entry").path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("tests") {
                visit_rust_files(&path, files);
            }
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && path.file_name().and_then(|name| name.to_str()) != Some("tests.rs")
        {
            files.push(path);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RustToken<'source> {
    text: &'source str,
    offset: usize,
}

#[derive(Debug, Default)]
struct SourceScan {
    calls: BTreeMap<String, usize>,
    prohibited_aliases: Vec<String>,
}

fn quoted_literal_end(source: &str, start: usize, quote: u8) -> usize {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor = (cursor + 2).min(bytes.len()),
            found if found == quote => return cursor + 1,
            _ => cursor += 1,
        }
    }
    bytes.len()
}

fn character_literal_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start + 1;
    if bytes.get(cursor) == Some(&b'\\') {
        cursor += 1;
        match bytes.get(cursor).copied() {
            Some(b'u') if bytes.get(cursor + 1) == Some(&b'{') => {
                cursor += 2;
                while cursor < bytes.len() && bytes[cursor] != b'}' {
                    cursor += 1;
                }
                cursor = (cursor + 1).min(bytes.len());
            }
            Some(_) => cursor += 1,
            None => return None,
        }
    } else {
        let character = source.get(cursor..)?.chars().next()?;
        cursor += character.len_utf8();
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

fn raw_string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut delimiter = if bytes.get(start..start + 2) == Some(b"br")
        || bytes.get(start..start + 2) == Some(b"cr")
    {
        start + 2
    } else if bytes.get(start) == Some(&b'r') {
        start + 1
    } else {
        return None;
    };
    while bytes.get(delimiter) == Some(&b'#') {
        delimiter += 1;
    }
    if bytes.get(delimiter) != Some(&b'"') {
        return None;
    }
    let hashes = delimiter
        - if bytes.get(start) == Some(&b'r') {
            start + 1
        } else {
            start + 2
        };
    let mut cursor = delimiter + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"'
            && bytes
                .get(cursor + 1..cursor + 1 + hashes)
                .is_some_and(|suffix| suffix.iter().all(|byte| *byte == b'#'))
        {
            return Some(cursor + 1 + hashes);
        }
        cursor += 1;
    }
    Some(bytes.len())
}

fn rust_tokens(source: &str) -> Vec<RustToken<'_>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"//") {
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            cursor += 2;
            let mut depth = 1usize;
            while cursor < bytes.len() && depth > 0 {
                if bytes.get(cursor..cursor + 2) == Some(b"/*") {
                    depth += 1;
                    cursor += 2;
                } else if bytes.get(cursor..cursor + 2) == Some(b"*/") {
                    depth -= 1;
                    cursor += 2;
                } else {
                    cursor += 1;
                }
            }
            continue;
        }
        if let Some(end) = raw_string_end(source, cursor) {
            cursor = end;
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"b\"")
            || bytes.get(cursor..cursor + 2) == Some(b"c\"")
        {
            cursor = quoted_literal_end(source, cursor + 1, b'"');
            continue;
        }
        if bytes.get(cursor) == Some(&b'"') {
            cursor = quoted_literal_end(source, cursor, b'"');
            continue;
        }
        if bytes.get(cursor..cursor + 2) == Some(b"b'")
            && let Some(end) = character_literal_end(source, cursor + 1)
        {
            cursor = end;
            continue;
        }
        if bytes.get(cursor) == Some(&b'\'')
            && let Some(end) = character_literal_end(source, cursor)
        {
            cursor = end;
            continue;
        }
        if bytes[cursor].is_ascii_alphabetic() || bytes[cursor] == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(RustToken {
                text: &source[start..cursor],
                offset: start,
            });
            continue;
        }
        let width = source[cursor..]
            .chars()
            .next()
            .expect("cursor is inside source")
            .len_utf8();
        tokens.push(RustToken {
            text: &source[cursor..cursor + width],
            offset: cursor,
        });
        cursor += width;
    }
    tokens
}

fn matching_delimiter(
    tokens: &[RustToken<'_>],
    start: usize,
    open: &str,
    close: &str,
) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        if token.text == open {
            depth += 1;
        } else if token.text == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn production_tokens(source: &str) -> Vec<RustToken<'_>> {
    let tokens = rust_tokens(source);
    let mut production = Vec::with_capacity(tokens.len());
    let mut index = 0usize;
    while index < tokens.len() {
        let inline_test_module = tokens.get(index).is_some_and(|token| token.text == "mod")
            && tokens.get(index + 2).is_some_and(|token| token.text == "{")
            && (tokens
                .get(index + 1)
                .is_some_and(|token| token.text == "tests")
                || has_test_cfg_attribute(&tokens, index));
        if inline_test_module && let Some(end) = matching_delimiter(&tokens, index + 2, "{", "}") {
            index = end + 1;
            continue;
        }
        production.push(tokens[index]);
        index += 1;
    }
    production
}

fn has_test_cfg_attribute(tokens: &[RustToken<'_>], item: usize) -> bool {
    let mut cursor = item;
    while cursor >= 2 && tokens[cursor - 1].text == "]" {
        let close = cursor - 1;
        let mut depth = 0usize;
        let mut open = None;
        for index in (0..=close).rev() {
            match tokens[index].text {
                "]" => depth += 1,
                "[" => {
                    depth -= 1;
                    if depth == 0 {
                        open = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(open) = open else {
            return false;
        };
        if open == 0 || tokens[open - 1].text != "#" {
            return false;
        }
        let attribute = &tokens[open + 1..close];
        if attribute.first().is_some_and(|token| token.text == "cfg")
            && (attribute.get(2).is_some_and(|token| token.text == "test")
                || attribute.get(2).is_some_and(|token| token.text == "all")
                    && attribute.iter().any(|token| token.text == "test"))
        {
            return true;
        }
        cursor = open - 1;
    }
    false
}

fn function_scopes(tokens: &[RustToken<'_>]) -> Vec<(String, usize, usize)> {
    let mut scopes = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.text != "fn" {
            continue;
        }
        let Some(name) = tokens.get(index + 1) else {
            continue;
        };
        if !name
            .text
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        {
            continue;
        }
        let Some(parameters) = tokens
            .iter()
            .enumerate()
            .skip(index + 2)
            .find_map(|(position, token)| match token.text {
                "(" => Some(Some(position)),
                ";" | "{" => Some(None),
                _ => None,
            })
            .flatten()
        else {
            continue;
        };
        let Some(parameters_end) = matching_delimiter(tokens, parameters, "(", ")") else {
            continue;
        };
        let Some(body) = tokens
            .iter()
            .enumerate()
            .skip(parameters_end + 1)
            .find_map(|(position, token)| match token.text {
                "{" => Some(Some(position)),
                ";" => Some(None),
                _ => None,
            })
            .flatten()
        else {
            continue;
        };
        if let Some(body_end) = matching_delimiter(tokens, body, "{", "}") {
            scopes.push((name.text.to_owned(), body, body_end));
        }
    }
    scopes
}

fn parser_reference_len(tokens: &[RustToken<'_>], start: usize) -> Option<usize> {
    let token = |offset: usize| tokens.get(start + offset).map(|token| token.text);
    if token(0) == Some("mech_syntax")
        && token(1) == Some(":")
        && token(2) == Some(":")
        && token(3) == Some("parser")
        && token(4) == Some(":")
        && token(5) == Some(":")
        && token(6) == Some("parse")
    {
        Some(7)
    } else if token(0) == Some("mech_syntax")
        && token(1) == Some(":")
        && token(2) == Some(":")
        && token(3) == Some("parse")
    {
        Some(4)
    } else if token(0) == Some("parser")
        && token(1) == Some(":")
        && token(2) == Some(":")
        && token(3) == Some("parse")
    {
        Some(4)
    } else {
        None
    }
}

fn source_line(source: &str, offset: usize) -> usize {
    source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

fn imports_retiring_parser(import: &[RustToken<'_>]) -> bool {
    let Some(root) = import.iter().position(|token| token.text == "mech_syntax") else {
        return false;
    };
    if import.get(root + 1).is_some_and(|token| token.text == "as") {
        return true;
    }
    if !matches!(
        (import.get(root + 1), import.get(root + 2)),
        (Some(colon1), Some(colon2)) if colon1.text == ":" && colon2.text == ":"
    ) {
        return false;
    }
    match import.get(root + 3).map(|token| token.text) {
        Some("parse") => true,
        Some("parser") => import
            .get(root + 4)
            .is_some_and(|token| matches!(token.text, "as" | ":")),
        Some("{") => {
            let mut depth = 1usize;
            let mut index = root + 4;
            while index < import.len() && depth > 0 {
                match import[index].text {
                    "{" => depth += 1,
                    "}" => depth -= 1,
                    "parse" if depth == 1 => return true,
                    "parser"
                        if depth == 1
                            && import
                                .get(index + 1)
                                .is_some_and(|token| matches!(token.text, "as" | ":")) =>
                    {
                        return true;
                    }
                    "self"
                        if depth == 1
                            && import
                                .get(index + 1)
                                .is_some_and(|token| token.text == "as") =>
                    {
                        return true;
                    }
                    _ => {}
                }
                index += 1;
            }
            false
        }
        _ => false,
    }
}

fn is_declared_root_parser_reexport(
    source_path: &str,
    import: &[RustToken<'_>],
    public_item: bool,
) -> bool {
    source_path == "src/lib.rs"
        && public_item
        && !import.iter().any(|token| token.text == "as")
        && import.windows(4).any(|tokens| {
            tokens
                .iter()
                .map(|token| token.text)
                .eq(["mech_syntax", ":", ":", "{"])
        })
}

fn is_declared_syntax_crate_reexport(
    source_path: &str,
    tokens: &[RustToken<'_>],
    index: usize,
) -> bool {
    source_path == "src/lib.rs"
        && index > 0
        && tokens[index - 1].text == "pub"
        && tokens
            .get(index)
            .is_some_and(|token| token.text == "extern")
        && tokens
            .get(index + 1)
            .is_some_and(|token| token.text == "crate")
        && tokens
            .get(index + 2)
            .is_some_and(|token| token.text == "mech_syntax")
        && tokens
            .get(index + 3)
            .is_some_and(|token| token.text == "as")
        && tokens
            .get(index + 4)
            .is_some_and(|token| token.text == "syntax")
        && tokens.get(index + 5).is_some_and(|token| token.text == ";")
}

fn scan_rust_source(source: &str, source_path: &str) -> SourceScan {
    let tokens = production_tokens(source);
    let scopes = function_scopes(&tokens);
    let mut scan = SourceScan::default();

    for (index, token) in tokens.iter().enumerate() {
        let public_item = index > 0 && tokens[index - 1].text == "pub";
        if token.text == "use" {
            let end = tokens[index..]
                .iter()
                .position(|token| token.text == ";")
                .map_or(tokens.len(), |offset| index + offset);
            let import = &tokens[index..end];
            if imports_retiring_parser(import)
                && !is_declared_root_parser_reexport(source_path, import, public_item)
            {
                scan.prohibited_aliases.push(format!(
                    "line {} imports or aliases the retiring parser",
                    source_line(source, token.offset)
                ));
            }
        }
        if token.text == "extern"
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.text == "crate")
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.text == "mech_syntax")
            && tokens
                .get(index + 3)
                .is_some_and(|token| token.text == "as")
            && !is_declared_syntax_crate_reexport(source_path, &tokens, index)
        {
            scan.prohibited_aliases.push(format!(
                "line {} aliases the mech_syntax crate",
                source_line(source, token.offset)
            ));
        }
    }

    let mut index = 0usize;
    while index < tokens.len() {
        let Some(path_len) = parser_reference_len(&tokens, index) else {
            index += 1;
            continue;
        };
        let reference = &tokens[index..index + path_len];
        let is_import = tokens[..index]
            .iter()
            .rposition(|token| matches!(token.text, "use" | ";"))
            .is_some_and(|start| tokens[start].text == "use");
        if !is_import {
            if tokens
                .get(index + path_len)
                .is_some_and(|token| token.text == "(")
            {
                let caller = scopes
                    .iter()
                    .filter(|(_, body, end)| *body < index && index < *end)
                    .max_by_key(|(_, body, _)| *body)
                    .map_or("<module>", |(name, _, _)| name.as_str());
                *scan.calls.entry(caller.to_owned()).or_insert(0) += 1;
            } else {
                scan.prohibited_aliases.push(format!(
                    "line {} takes or aliases a retiring parser function",
                    source_line(source, reference[0].offset)
                ));
            }
        }
        index += path_len;
    }
    scan
}

fn is_test_only_caller(path: &str, caller: &str) -> bool {
    matches!(
        (path, caller),
        (
            "src/engine/src/program/compiler_planning.rs",
            "plan_source_for_test"
        ) | (
            "src/engine/src/structures.rs",
            "wildcard_table_column_uses_the_canonical_dynamic_schema"
        )
    )
}

fn discovered_production_calls() -> (BTreeMap<(String, String), usize>, Vec<String>) {
    let root = repository_root();
    let mut files = Vec::new();
    visit_rust_files(&root.join("src"), &mut files);
    let mut calls = BTreeMap::new();
    let mut prohibited_aliases = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&root)
            .expect("source path is below repository root")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).expect("read Rust source");
        let scan = scan_rust_source(&source, &relative);
        prohibited_aliases.extend(
            scan.prohibited_aliases
                .into_iter()
                .map(|violation| format!("{relative}:{violation}")),
        );
        for (caller, count) in scan.calls {
            if !is_test_only_caller(&relative, &caller) {
                assert!(
                    calls
                        .insert((relative.clone(), caller.clone()), count)
                        .is_none(),
                    "duplicate source/caller scan result for {relative}::{caller}"
                );
            }
        }
    }
    (calls, prohibited_aliases)
}

#[test]
fn production_parser_callers_are_exactly_inventoried() {
    let consumers = consumers();
    let mut expected = BTreeMap::new();
    let mut total = 0usize;
    for (id, row) in &consumers {
        let source = fs::read_to_string(repository_root().join(&row.source_path))
            .unwrap_or_else(|error| panic!("read consumer source for {id}: {error}"));
        assert!(
            source.contains(&format!("fn {}", row.caller)),
            "missing caller {} for {id}",
            row.caller
        );
        for (field, value) in [
            ("feature gate", &row.feature_gate),
            ("input policy", &row.input_policy),
            ("failure policy", &row.failure_policy),
            ("current result use", &row.current_result_use),
            ("canonical target", &row.canonical_target),
            ("cutover action", &row.cutover_action),
            ("notes", &row.notes),
        ] {
            assert!(!value.is_empty(), "empty {field} for {id}");
        }
        assert!(
            !row.canonical_target.contains("legacy")
                && !row.canonical_target.contains("compat")
                && !row.cutover_action.contains("legacy")
                && !row.cutover_action.contains("compat"),
            "noncanonical cutover target for {id}"
        );
        assert!(
            expected
                .insert((row.source_path.clone(), row.caller.clone()), row.calls)
                .is_none(),
            "duplicate source/caller contract for {id}"
        );
        total += row.calls;
    }
    assert_eq!(total, EXPECTED_PRODUCTION_CALLS);
    let (discovered, prohibited_aliases) = discovered_production_calls();
    assert!(
        prohibited_aliases.is_empty(),
        "production parser aliases are prohibited: {prohibited_aliases:#?}"
    );
    assert_eq!(discovered, expected);
}

#[test]
fn source_scanner_binds_each_call_to_its_enclosing_function() {
    let source = r##"
fn first() {
    let _ = "mech_syntax::parse(ignored)";
    let _ = r#"parser::parse(ignored)"#;
    // mech_syntax::parser::parse(ignored)
    mech_syntax::parse("one");
    parser::parse("two");
}

fn second() {
    /* parser::parse(ignored) */
    mech_syntax::parser::parse("three");
}
"##;
    let scan = scan_rust_source(source, "fixture.rs");
    assert!(scan.prohibited_aliases.is_empty());
    assert_eq!(
        scan.calls,
        BTreeMap::from([("first".to_owned(), 2), ("second".to_owned(), 1)])
    );
}

#[test]
fn source_scanner_rejects_every_parser_alias_shape() {
    for source in [
        "use mech_syntax::parse;\nfn run() { parse(\"source\"); }",
        "use mech_syntax::parser::parse as parse_source;\nfn run() { parse_source(\"source\"); }",
        "use mech_syntax::parser as old_parser;\nfn run() { old_parser::parse(\"source\"); }",
        "use mech_syntax::parser::*;\nfn run() { parse(\"source\"); }",
        "use mech_syntax as syntax;\nfn run() { syntax::parse(\"source\"); }",
        "extern crate mech_syntax as syntax;\nfn run() { syntax::parse(\"source\"); }",
        "fn run() { let parse_source = mech_syntax::parse; parse_source(\"source\"); }",
        "use mech_syntax::parser;\nfn run() { let parse_source = parser::parse; parse_source(\"source\"); }",
    ] {
        assert!(
            !scan_rust_source(source, "fixture.rs")
                .prohibited_aliases
                .is_empty(),
            "alias escaped the parser census: {source}"
        );
    }
}

#[test]
fn source_scanner_allows_only_the_declared_root_reexports() {
    let source = "pub extern crate mech_syntax as syntax;\npub use mech_syntax::{parse, parser};";
    let scan = scan_rust_source(source, "src/lib.rs");
    assert!(scan.prohibited_aliases.is_empty());
    assert!(scan.calls.is_empty());

    for path in ["fixture.rs", "src/lib.rs"] {
        let alias = scan_rust_source(
            "pub use mech_syntax::parse as old_parse;\nfn run() { old_parse(\"source\"); }",
            path,
        );
        assert!(!alias.prohibited_aliases.is_empty(), "{path}");
    }
}

#[test]
fn source_scanner_allows_the_canonical_document_parser_module() {
    let source = "use mech_syntax::document::parser as canonical_parser;";
    let scan = scan_rust_source(source, "fixture.rs");
    assert!(scan.prohibited_aliases.is_empty());
    assert!(scan.calls.is_empty());
}

#[test]
fn source_scanner_excludes_only_the_actual_test_module() {
    let external = scan_rust_source(
        "mod tests;\nfn after_tests() { mech_syntax::parse(\"source\"); }",
        "fixture.rs",
    );
    assert_eq!(
        external.calls,
        BTreeMap::from([("after_tests".to_owned(), 1)])
    );

    let inline = scan_rust_source(
        "mod tests { fn ignored() { mech_syntax::parse(\"test\"); } }\n\
         #[cfg(all(test, target_arch = \"wasm32\"))]\n\
         mod browser_tests { fn ignored_too() { mech_syntax::parse(\"test\"); } }\n\
         fn after_tests() { mech_syntax::parse(\"source\"); }",
        "fixture.rs",
    );
    assert_eq!(
        inline.calls,
        BTreeMap::from([("after_tests".to_owned(), 1)])
    );
}

#[test]
fn every_consumer_names_a_frozen_fixture() {
    let fixtures = fixtures();
    let fixture_names = fixtures.keys().cloned().collect::<BTreeSet<_>>();
    for (id, row) in consumers() {
        assert!(
            fixture_names.contains(&row.fixture),
            "unknown fixture {} for {id}",
            row.fixture
        );
    }
}

#[test]
fn frozen_fixtures_define_canonical_document_expectations() {
    for (id, row) in fixtures() {
        assert_eq!(row.root, "document", "canonical root for {id}");
        assert!(
            matches!(row.outcome.as_str(), "accept" | "reject"),
            "canonical outcome for {id}"
        );
        assert!(
            !row.contracts.is_empty(),
            "empty consumer contracts for {id}"
        );
        assert!(!row.notes.is_empty(), "empty fixture note for {id}");
        assert!(
            repository_root().join(&row.source_path).is_file(),
            "missing canonical fixture {id}"
        );
    }
}
