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

fn visit_rust_files(path: &Path, files: &mut BTreeSet<PathBuf>) {
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
            files.insert(path);
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
        if is_test_cfg_attribute(&tokens, index)
            && let Some(item) = attributed_item_start(&tokens, index)
            && let Some(end) = rust_item_end(&tokens, item)
        {
            index = end;
            continue;
        }
        production.push(tokens[index]);
        index += 1;
    }
    production
}

fn attribute_close(tokens: &[RustToken<'_>], hash: usize) -> Option<usize> {
    (tokens.get(hash).is_some_and(|token| token.text == "#")
        && tokens.get(hash + 1).is_some_and(|token| token.text == "["))
    .then(|| matching_delimiter(tokens, hash + 1, "[", "]"))
    .flatten()
}

fn is_test_cfg_attribute(tokens: &[RustToken<'_>], hash: usize) -> bool {
    let Some(close) = attribute_close(tokens, hash) else {
        return false;
    };
    let attribute = &tokens[hash + 2..close];
    attribute.first().is_some_and(|token| token.text == "cfg")
        && attribute.get(1).is_some_and(|token| token.text == "(")
        && matching_delimiter(attribute, 1, "(", ")") == Some(attribute.len() - 1)
        && cfg_without_test(&attribute[2..attribute.len() - 1]) == Some(false)
}

/// Only exclude an item when its cfg is provably false with `test = false`.
/// Features, targets, and unknown predicates can vary across production builds.
fn cfg_without_test(predicate: &[RustToken<'_>]) -> Option<bool> {
    if predicate.len() == 1 && predicate[0].text == "test" {
        return Some(false);
    }
    let operator = predicate.first()?.text;
    if !matches!(operator, "all" | "any" | "not")
        || predicate.get(1)?.text != "("
        || matching_delimiter(predicate, 1, "(", ")") != Some(predicate.len() - 1)
    {
        return None;
    }
    let mut arguments = Vec::new();
    let mut depth = 0usize;
    let mut start = 2;
    for index in 2..predicate.len() - 1 {
        match predicate[index].text {
            "(" => depth += 1,
            ")" => depth = depth.checked_sub(1)?,
            "," if depth == 0 => {
                arguments.push(cfg_without_test(&predicate[start..index]));
                start = index + 1;
            }
            _ => {}
        }
    }
    if start < predicate.len() - 1 {
        arguments.push(cfg_without_test(&predicate[start..predicate.len() - 1]));
    }
    match operator {
        "not" if arguments.len() == 1 => arguments[0].map(|value| !value),
        "all" if arguments.contains(&Some(false)) => Some(false),
        "all" if arguments.iter().all(|value| *value == Some(true)) => Some(true),
        "any" if arguments.contains(&Some(true)) => Some(true),
        "any" if arguments.iter().all(|value| *value == Some(false)) => Some(false),
        _ => None,
    }
}

fn attributed_item_start(tokens: &[RustToken<'_>], hash: usize) -> Option<usize> {
    let mut cursor = hash;
    while let Some(close) = attribute_close(tokens, cursor) {
        cursor = close + 1;
    }
    (cursor < tokens.len()).then_some(cursor)
}

fn rust_item_end(tokens: &[RustToken<'_>], start: usize) -> Option<usize> {
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(start) {
        match token.text {
            "(" => parentheses += 1,
            ")" => parentheses = parentheses.checked_sub(1)?,
            "[" => brackets += 1,
            "]" => brackets = brackets.checked_sub(1)?,
            "{" if parentheses == 0 && brackets == 0 => {
                return matching_delimiter(tokens, index, "{", "}").map(|close| close + 1);
            }
            ";" if parentheses == 0 && brackets == 0 => return Some(index + 1),
            _ => {}
        }
    }
    None
}

fn raw_function_scopes(tokens: &[RustToken<'_>]) -> Vec<(String, usize, usize)> {
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

fn compact_rust_header(tokens: &[RustToken<'_>]) -> String {
    let mut header = String::new();
    let word_character = |character: char| character.is_ascii_alphanumeric() || character == '_';
    for token in tokens {
        if header.chars().next_back().is_some_and(word_character)
            && token.text.chars().next().is_some_and(word_character)
        {
            header.push(' ');
        }
        header.push_str(token.text);
    }
    header
}

/// Include lexical modules, impl headers, traits, and containing functions in
/// each caller key. Identically named methods are separate removal contracts.
fn function_scopes(tokens: &[RustToken<'_>]) -> Vec<(String, usize, usize)> {
    let functions = raw_function_scopes(tokens);
    let mut contexts = functions.clone();
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token.text, "mod" | "impl" | "trait") {
            continue;
        }
        if token.text == "impl"
            && index > 0
            && !matches!(
                tokens[index - 1].text,
                "{" | "}" | ";" | "]" | "unsafe" | "default"
            )
        {
            continue;
        }
        let Some(body) = tokens
            .iter()
            .enumerate()
            .skip(index + 2)
            .find_map(|(position, token)| match token.text {
                "{" => Some(Some(position)),
                ";" => Some(None),
                _ => None,
            })
            .flatten()
        else {
            continue;
        };
        let Some(end) = matching_delimiter(tokens, body, "{", "}") else {
            continue;
        };
        let label = match token.text {
            "impl" => format!("impl {}", compact_rust_header(&tokens[index + 1..body])),
            "trait" => format!("trait {}", tokens[index + 1].text),
            _ => tokens[index + 1].text.to_owned(),
        };
        contexts.push((label, body, end));
    }
    let mut qualified = functions
        .into_iter()
        .map(|(name, body, end)| {
            let mut parents = contexts
                .iter()
                .filter(|(_, outer_body, outer_end)| *outer_body < body && end < *outer_end)
                .collect::<Vec<_>>();
            parents.sort_by_key(|(_, body, _)| *body);
            let name = parents
                .into_iter()
                .map(|(name, _, _)| name.as_str())
                .chain([name.as_str()])
                .collect::<Vec<_>>()
                .join("::");
            (name, body, end)
        })
        .collect::<Vec<_>>();
    let mut counts = BTreeMap::new();
    for (name, _, _) in &qualified {
        *counts.entry(name.clone()).or_insert(0) += 1;
    }
    for (name, body, _) in &mut qualified {
        if counts[name] > 1 {
            // Mutually exclusive cfg implementations may share a full path.
            // A physical source position still prevents their counts merging.
            *name = format!("{name}@{}", tokens[*body].offset);
        }
    }
    qualified
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParserReexports {
    None,
    RootCrate,
    SyntaxCrate,
}

fn parser_reference_len(
    tokens: &[RustToken<'_>],
    start: usize,
    reexports: ParserReexports,
) -> Option<usize> {
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
    } else if reexports == ParserReexports::RootCrate
        && token(0) == Some("crate")
        && token(1) == Some(":")
        && token(2) == Some(":")
        && token(3) == Some("syntax")
        && token(4) == Some(":")
        && token(5) == Some(":")
        && token(6) == Some("parser")
        && token(7) == Some(":")
        && token(8) == Some(":")
        && token(9) == Some("parse")
    {
        Some(10)
    } else if reexports == ParserReexports::RootCrate
        && token(0) == Some("crate")
        && token(1) == Some(":")
        && token(2) == Some(":")
        && token(3) == Some("syntax")
        && token(4) == Some(":")
        && token(5) == Some(":")
        && token(6) == Some("parse")
    {
        Some(7)
    } else if reexports != ParserReexports::None
        && token(0) == Some("crate")
        && token(1) == Some(":")
        && token(2) == Some(":")
        && token(3) == Some("parser")
        && token(4) == Some(":")
        && token(5) == Some(":")
        && token(6) == Some("parse")
    {
        Some(7)
    } else if reexports != ParserReexports::None
        && token(0) == Some("crate")
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

fn imports_retiring_parser(import: &[RustToken<'_>], reexports: ParserReexports) -> bool {
    let Some(mut root) = import.iter().position(|token| {
        token.text == "mech_syntax" || reexports != ParserReexports::None && token.text == "crate"
    }) else {
        return false;
    };
    if import[root].text == "crate"
        && reexports == ParserReexports::RootCrate
        && import
            .get(root + 3)
            .is_some_and(|token| token.text == "syntax")
    {
        root += 3;
    }
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
        // An existing crate-root glob exposes the known root re-export. Bare
        // calls in that source file are counted by `bare_parser_available`.
        Some("*") => import[root].text != "crate",
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
                    "parse" | "*" if depth == 1 => return true,
                    "syntax" if depth == 1 && reexports == ParserReexports::RootCrate => {
                        return true;
                    }
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

fn is_existing_syntax_parser_import(
    source_path: &str,
    import: &[RustToken<'_>],
    public_item: bool,
) -> bool {
    matches!(
        (source_path, public_item),
        ("src/syntax/src/lib.rs", true) | ("src/syntax/src/base.rs", false)
    ) && import
        .iter()
        .map(|token| token.text)
        .eq(["use", "crate", ":", ":", "parser", ":", ":", "*"])
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

fn bare_parser_available(
    tokens: &[RustToken<'_>],
    source_path: &str,
    reexports: ParserReexports,
) -> bool {
    if reexports == ParserReexports::None {
        return false;
    }
    matches!(
        source_path,
        "src/lib.rs" | "src/syntax/src/lib.rs" | "src/syntax/src/base.rs"
    ) || tokens.windows(6).any(|tokens| {
        tokens
            .iter()
            .map(|token| token.text)
            .eq(["use", "crate", ":", ":", "*", ";"])
    })
}

fn scan_rust_source(source: &str, source_path: &str, reexports: ParserReexports) -> SourceScan {
    let tokens = production_tokens(source);
    let scopes = function_scopes(&tokens);
    let bare_parser = bare_parser_available(&tokens, source_path, reexports);
    let mut scan = SourceScan::default();

    for (index, token) in tokens.iter().enumerate() {
        let public_item = index > 0 && tokens[index - 1].text == "pub";
        if token.text == "use" {
            let end = tokens[index..]
                .iter()
                .position(|token| token.text == ";")
                .map_or(tokens.len(), |offset| index + offset);
            let import = &tokens[index..end];
            if imports_retiring_parser(import, reexports)
                && !is_declared_root_parser_reexport(source_path, import, public_item)
                && !is_existing_syntax_parser_import(source_path, import, public_item)
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
        let bare_reference = bare_parser
            && tokens[index].text == "parse"
            && (index == 0 || !matches!(tokens[index - 1].text, "fn" | "." | ":"));
        let Some(path_len) =
            parser_reference_len(&tokens, index, reexports).or_else(|| bare_reference.then_some(1))
        else {
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

fn workspace_source_roots(root: &Path) -> BTreeSet<PathBuf> {
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("read workspace manifest");
    let workspace = manifest
        .split_once("[workspace]")
        .map(|(_, workspace)| workspace)
        .expect("workspace table");
    let members = workspace
        .split_once("members = [")
        .map(|(_, members)| members)
        .and_then(|members| members.split_once(']').map(|(members, _)| members))
        .expect("workspace members array");
    let mut roots = BTreeSet::from([root.join("src")]);
    for member in members.split('"').skip(1).step_by(2) {
        let source = root.join(member).join("src");
        if source.is_dir() {
            roots.insert(source);
        }
    }
    roots
}

fn parser_reexports(root: &Path, path: &Path) -> ParserReexports {
    let mut directory = path.parent();
    while let Some(candidate) = directory {
        if candidate.join("Cargo.toml").is_file() {
            return if candidate == root {
                ParserReexports::RootCrate
            } else if candidate == root.join("src/syntax") {
                ParserReexports::SyntaxCrate
            } else {
                ParserReexports::None
            };
        }
        if candidate == root {
            break;
        }
        directory = candidate.parent();
    }
    ParserReexports::None
}

fn discovered_production_calls() -> (BTreeMap<(String, String), usize>, Vec<String>) {
    let root = repository_root();
    let mut files = BTreeSet::new();
    for source_root in workspace_source_roots(&root) {
        visit_rust_files(&source_root, &mut files);
    }
    let mut calls = BTreeMap::new();
    let mut prohibited_aliases = Vec::new();
    for path in files {
        let relative = path
            .strip_prefix(&root)
            .expect("source path is below repository root")
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path).expect("read Rust source");
        let scan = scan_rust_source(&source, &relative, parser_reexports(&root, &path));
        prohibited_aliases.extend(
            scan.prohibited_aliases
                .into_iter()
                .map(|violation| format!("{relative}:{violation}")),
        );
        for (caller, count) in scan.calls {
            assert!(
                calls
                    .insert((relative.clone(), caller.clone()), count)
                    .is_none(),
                "duplicate source/caller scan result for {relative}::{caller}"
            );
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
            function_scopes(&production_tokens(&source))
                .iter()
                .any(|(caller, _, _)| caller == &row.caller),
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
    let scan = scan_rust_source(source, "fixture.rs", ParserReexports::None);
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
            !scan_rust_source(source, "fixture.rs", ParserReexports::None)
                .prohibited_aliases
                .is_empty(),
            "alias escaped the parser census: {source}"
        );
    }
}

#[test]
fn source_scanner_allows_only_the_declared_root_reexports() {
    let source = "pub extern crate mech_syntax as syntax;\n\
                  pub use mech_syntax::{parse, parser};\n\
                  fn direct() { crate::parse(\"one\"); crate::parser::parse(\"two\"); }\n\
                  fn syntax() { crate::syntax::parse(\"three\"); crate::syntax::parser::parse(\"four\"); }";
    let scan = scan_rust_source(source, "src/lib.rs", ParserReexports::RootCrate);
    assert!(scan.prohibited_aliases.is_empty());
    assert_eq!(
        scan.calls,
        BTreeMap::from([("direct".to_owned(), 2), ("syntax".to_owned(), 2)])
    );

    for path in ["fixture.rs", "src/lib.rs"] {
        let alias = scan_rust_source(
            "pub use mech_syntax::parse as old_parse;\nfn run() { old_parse(\"source\"); }",
            path,
            if path == "src/lib.rs" {
                ParserReexports::RootCrate
            } else {
                ParserReexports::None
            },
        );
        assert!(!alias.prohibited_aliases.is_empty(), "{path}");
    }
}

#[test]
fn source_scanner_allows_the_canonical_document_parser_module() {
    let source = "use mech_syntax::document::parser as canonical_parser;";
    let scan = scan_rust_source(source, "fixture.rs", ParserReexports::None);
    assert!(scan.prohibited_aliases.is_empty());
    assert!(scan.calls.is_empty());
}

#[test]
fn source_scanner_excludes_test_only_modules_and_items() {
    let external = scan_rust_source(
        "mod tests;\nfn after_tests() { mech_syntax::parse(\"source\"); }",
        "fixture.rs",
        ParserReexports::None,
    );
    assert_eq!(
        external.calls,
        BTreeMap::from([("after_tests".to_owned(), 1)])
    );

    let inline = scan_rust_source(
        "#[cfg(test)] mod tests { fn ignored() { mech_syntax::parse(\"test\"); } }\n\
         #[cfg(all(test, target_arch = \"wasm32\"))]\n\
         mod browser_tests { fn ignored_too() { mech_syntax::parse(\"test\"); } }\n\
         #[cfg(test)]\n\
         fn ignored_helper() { mech_syntax::parse(\"test\"); }\n\
         #[cfg(test)]\n\
         impl Fixture { fn ignored_method() { mech_syntax::parse(\"test\"); } }\n\
         #[cfg(not(test))]\n\
         fn production_cfg() { mech_syntax::parse(\"source\"); }\n\
         fn after_tests() { mech_syntax::parse(\"source\"); }",
        "fixture.rs",
        ParserReexports::None,
    );
    assert_eq!(
        inline.calls,
        BTreeMap::from([
            ("after_tests".to_owned(), 1),
            ("production_cfg".to_owned(), 1),
        ])
    );
}

#[test]
fn source_scanner_preserves_cfg_branches_available_without_tests() {
    for predicate in [
        "not(test)",
        "all(not(test), feature = \"source\")",
        "all(feature = \"source\", not(test))",
        "all(not(test), any(feature = \"source\", target_arch = \"wasm32\"))",
        "any(test, feature = \"source\")",
        "not(all(not(test), feature = \"source\"))",
        "not(not(not(test)))",
        "all()",
    ] {
        let source =
            format!("#[cfg({predicate})] fn production() {{ mech_syntax::parse(\"source\"); }}");
        let scan = scan_rust_source(&source, "fixture.rs", ParserReexports::None);
        assert_eq!(
            scan.calls,
            BTreeMap::from([("production".to_owned(), 1)]),
            "{predicate}"
        );
    }
    for predicate in [
        "test",
        "all(test, feature = \"source\")",
        "not(not(test))",
        "any(test, all(test, feature = \"source\"))",
        "not(any(not(test), feature = \"source\"))",
    ] {
        for item in [
            "fn helper() { mech_syntax::parse(\"test\"); }",
            "impl Fixture { fn method() { mech_syntax::parse(\"test\"); } }",
            "mod nested { fn helper() { mech_syntax::parse(\"test\"); } }",
        ] {
            let source = format!("#[cfg({predicate})] {item}");
            assert!(
                scan_rust_source(&source, "fixture.rs", ParserReexports::None)
                    .calls
                    .is_empty(),
                "{source}"
            );
        }
    }
}

#[test]
fn source_scanner_counts_syntax_crate_root_reexports_in_its_own_sources() {
    let root = repository_root();
    for path in ["src/syntax/src/lib.rs", "src/syntax/src/document/mod.rs"] {
        let reexports = parser_reexports(&root, &root.join(path));
        assert_eq!(reexports, ParserReexports::SyntaxCrate, "{path}");
        let scan = scan_rust_source(
            "fn caller() { crate::parse(\"one\"); crate::parser::parse(\"two\"); }",
            path,
            reexports,
        );
        assert_eq!(scan.calls, BTreeMap::from([("caller".to_owned(), 2)]));
    }
    for (path, source) in [
        ("src/syntax/src/lib.rs", "pub use crate::parser::*;"),
        ("src/syntax/src/base.rs", "use crate::parser::*;"),
    ] {
        assert!(
            scan_rust_source(source, path, ParserReexports::SyntaxCrate)
                .prohibited_aliases
                .is_empty()
        );
        assert!(
            !scan_rust_source(
                source,
                "src/syntax/src/new_caller.rs",
                ParserReexports::SyntaxCrate
            )
            .prohibited_aliases
            .is_empty()
        );
    }
    for source in [
        "use crate::parse as old; fn caller() { old(\"source\"); }",
        "use crate::{parse as old}; fn caller() { old(\"source\"); }",
        "use crate::parser as old; fn caller() { old::parse(\"source\"); }",
        "fn caller() { let old = crate::parse; old(\"source\"); }",
    ] {
        assert!(
            !scan_rust_source(
                source,
                "src/syntax/src/lib.rs",
                ParserReexports::SyntaxCrate
            )
            .prohibited_aliases
            .is_empty(),
            "{source}"
        );
    }
    assert_eq!(
        parser_reexports(&root, &root.join("src/lib.rs")),
        ParserReexports::RootCrate
    );
    let reexports = parser_reexports(&root, &root.join("src/runtime/src/lib.rs"));
    assert_eq!(reexports, ParserReexports::None);
    assert!(
        scan_rust_source(
            "fn caller() { crate::parse(\"unrelated\"); }",
            "src/runtime/src/lib.rs",
            reexports
        )
        .calls
        .is_empty()
    );
}

#[test]
fn source_scanner_distinguishes_method_module_and_nested_function_owners() {
    let source = r#"
impl Outer { fn compile_source() {} }
impl<'a> View<'a> { fn compile_source() { mech_syntax::parse("source"); } }
mod one { fn run() { mech_syntax::parse("one"); } }
mod two { fn run() { mech_syntax::parse("two"); } }
fn enclosing() { fn run() { mech_syntax::parse("nested"); } }
"#;
    let scan = scan_rust_source(source, "fixture.rs", ParserReexports::None);
    assert_eq!(
        scan.calls,
        BTreeMap::from([
            ("impl <'a>View<'a>::compile_source".to_owned(), 1),
            ("one::run".to_owned(), 1),
            ("two::run".to_owned(), 1),
            ("enclosing::run".to_owned(), 1),
        ])
    );
    let moved = source
        .replace(
            "impl Outer { fn compile_source() {} }",
            "impl Outer { fn compile_source() { mech_syntax::parse(\"source\"); } }",
        )
        .replace(
            "impl<'a> View<'a> { fn compile_source() { mech_syntax::parse(\"source\"); } }",
            "impl<'a> View<'a> { fn compile_source() {} }",
        );
    let moved = scan_rust_source(&moved, "fixture.rs", ParserReexports::None);
    assert_eq!(
        scan.calls.values().sum::<usize>(),
        moved.calls.values().sum::<usize>()
    );
    assert_ne!(
        scan.calls, moved.calls,
        "moving a call between methods must invalidate the census"
    );
    assert_eq!(moved.calls.get("impl Outer::compile_source"), Some(&1));

    let same_path = "#[cfg(feature = \"a\")] impl Same { fn run() { mech_syntax::parse(\"one\"); } } #[cfg(not(feature = \"a\"))] impl Same { fn run() { mech_syntax::parse(\"two\"); } }";
    let scan = scan_rust_source(same_path, "fixture.rs", ParserReexports::None);
    assert_eq!(
        scan.calls.len(),
        2,
        "cfg alternatives with identical names remain distinct source owners"
    );
    assert!(
        scan.calls
            .keys()
            .all(|name| name.starts_with("impl Same::run@"))
    );
}

#[test]
fn source_scanner_closes_glob_and_root_reexport_call_bypasses() {
    for (source, reexports) in [
        (
            "use mech_syntax::*; fn run() { parse(\"source\"); }",
            ParserReexports::None,
        ),
        (
            "use mech_syntax::{*}; fn run() { parse(\"source\"); }",
            ParserReexports::None,
        ),
        (
            "use crate::{syntax as old}; fn run() { old::parse(\"source\"); }",
            ParserReexports::RootCrate,
        ),
        (
            "use crate::{syntax::{parse as old}}; fn run() { old(\"source\"); }",
            ParserReexports::RootCrate,
        ),
    ] {
        assert!(
            !scan_rust_source(source, "fixture.rs", reexports)
                .prohibited_aliases
                .is_empty(),
            "{source}"
        );
    }
    for (source, path, reexports) in [
        (
            "pub use mech_syntax::{parse, parser};",
            "src/lib.rs",
            ParserReexports::RootCrate,
        ),
        (
            "pub use crate::parser::*;",
            "src/syntax/src/lib.rs",
            ParserReexports::SyntaxCrate,
        ),
        (
            "use crate::parser::*;",
            "src/syntax/src/base.rs",
            ParserReexports::SyntaxCrate,
        ),
        (
            "use crate::*;",
            "src/syntax/src/example.rs",
            ParserReexports::SyntaxCrate,
        ),
    ] {
        let scan = scan_rust_source(
            &format!("{source} fn run() {{ parse(\"source\"); }}"),
            path,
            reexports,
        );
        assert!(scan.prohibited_aliases.is_empty(), "{source}");
        assert_eq!(
            scan.calls,
            BTreeMap::from([("run".to_owned(), 1)]),
            "{source}"
        );
        let alias = scan_rust_source(
            &format!("{source} fn run() {{ let alias = parse; alias(\"source\"); }}"),
            path,
            reexports,
        );
        assert!(!alias.prohibited_aliases.is_empty(), "{source}");
    }
    let ordinary = scan_rust_source(
        "fn run() { parse(\"canonical helper\"); }",
        "src/syntax/src/document/parser/canonical.rs",
        ParserReexports::SyntaxCrate,
    );
    assert!(
        ordinary.calls.is_empty(),
        "unrelated local callbacks have no root parser binding"
    );
}

#[test]
fn source_scanner_requires_cfg_proof_before_excluding_a_tests_module() {
    for attribute in [
        "",
        "#[cfg(not(test))]",
        "#[cfg(all(not(test), feature = \"source\"))]",
    ] {
        let source =
            format!("{attribute} mod tests {{ fn run() {{ mech_syntax::parse(\"source\"); }} }}");
        assert_eq!(
            scan_rust_source(&source, "fixture.rs", ParserReexports::None).calls,
            BTreeMap::from([("tests::run".to_owned(), 1)]),
            "{source}"
        );
    }
    let source = "#[cfg(test)] mod tests { fn run() { mech_syntax::parse(\"test\"); } }";
    assert!(
        scan_rust_source(source, "fixture.rs", ParserReexports::None)
            .calls
            .is_empty()
    );
}

#[test]
fn production_scan_roots_follow_the_workspace_members() {
    let root = repository_root();
    let roots = workspace_source_roots(&root);
    assert!(roots.contains(&root.join("src")));
    assert!(roots.contains(&root.join("hosts/gpu/src")));
    assert!(roots.contains(&root.join("tests/fixtures/native-live-host/src")));
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
