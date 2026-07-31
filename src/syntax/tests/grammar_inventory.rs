//! Mechanical coverage checks for the Phase 0 parser inventory.
//!
//! This intentionally remains a small, dependency-free source scanner. It is
//! not a Rust parser: it removes comments and literals, tracks module-level
//! brace depth, and recognizes ordinary function items plus the three macros
//! that generate the terminal functions in `base.rs`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const MODULES: &[&str] = &[
    "base",
    "literals",
    "structures",
    "expressions",
    "patterns",
    "statements",
    "functions",
    "state_machines",
    "activation",
    "imports",
    "mechdown",
    "mika",
    "grammar",
    "repl",
    "parser",
];

const FUNCTION_GENERATING_MACROS: &[&str] = &["leaf", "ws0_leaf", "ws1_leaf"];

const REQUIRED_COLUMNS: &[&str] = &[
    "id",
    "grammar-name",
    "module",
    "rust-function",
    "classification",
    "feature-gate",
    "entry-point",
    "output-type",
    "parent-rules",
    "child-rules",
    "selection-behavior",
    "termination",
    "whitespace",
    "spec-location",
    "conformance-cases",
    "implementation-path",
    "notes",
];

const CLASSIFICATIONS: &[&str] = &[
    "root",
    "production",
    "terminal",
    "lexical-primitive",
    "semantic-validation",
    "parser-control",
    "recovery",
    "diagnostic",
    "helper",
    "not-grammar",
];

const SELECTION_BEHAVIORS: &[&str] = &[
    "none",
    "ordered-first-success",
    "alt-best-longest-success",
    "manual-fallback",
    "peek-gated",
    "prefix-committed",
    "caller-controlled",
];

const EXPECTED_INVENTORY_ROWS: usize = 562;
const EXPECTED_GRAMMAR_ROWS: usize = 539;
const EXPECTED_MECHANICS_ROWS: usize = 23;
const CANONICAL_SPECIFICATION: &str = "docs/design/specification.mec";

fn syntax_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repository_root() -> PathBuf {
    syntax_root()
        .parent()
        .and_then(Path::parent)
        .expect("syntax crate must live at <repository>/src/syntax")
        .to_path_buf()
}

fn inventory_path() -> PathBuf {
    repository_root().join("docs/design/grammar-audit/productions.tsv")
}

fn specification_path() -> PathBuf {
    repository_root().join(CANONICAL_SPECIFICATION)
}

/// Replace comments and literal contents with spaces while preserving newlines
/// and delimiters in Rust code. This makes brace-depth tracking insensitive to
/// examples such as `"}"` and `r#"{ not code }"#`.
fn mask_comments_and_literals(source: &str) -> String {
    #[derive(Clone, Copy, Debug)]
    enum State {
        Code,
        LineComment,
        BlockComment(usize),
        String { escaped: bool },
        Character { escaped: bool },
        RawString { hashes: usize },
    }

    fn mask_char(output: &mut String, ch: char) {
        output.push(if ch == '\n' { '\n' } else { ' ' });
    }

    fn starts_character_literal(chars: &[char], start: usize) -> bool {
        let mut index = start + 1;
        let mut escaped = false;
        while index < chars.len() && chars[index] != '\n' {
            let ch = chars[index];
            if !escaped && ch == '\'' {
                return index > start + 1;
            }
            if !escaped && ch == '\\' {
                escaped = true;
            } else {
                escaped = false;
            }
            index += 1;
        }
        false
    }

    let chars = source.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(source.len());
    let mut state = State::Code;
    let mut index = 0;

    while index < chars.len() {
        let ch = chars[index];
        let next = chars.get(index + 1).copied();
        match state {
            State::Code => {
                if ch == '/' && next == Some('/') {
                    output.push_str("  ");
                    index += 2;
                    state = State::LineComment;
                    continue;
                }
                if ch == '/' && next == Some('*') {
                    output.push_str("  ");
                    index += 2;
                    state = State::BlockComment(1);
                    continue;
                }

                // Recognize r"...", r#"..."#, and the `r` portion of br#"".
                if ch == 'r' {
                    let mut probe = index + 1;
                    while chars.get(probe) == Some(&'#') {
                        probe += 1;
                    }
                    if chars.get(probe) == Some(&'"') {
                        let hashes = probe - index - 1;
                        for literal_ch in &chars[index..=probe] {
                            mask_char(&mut output, *literal_ch);
                        }
                        index = probe + 1;
                        state = State::RawString { hashes };
                        continue;
                    }
                }

                if ch == '"' {
                    mask_char(&mut output, ch);
                    index += 1;
                    state = State::String { escaped: false };
                    continue;
                }
                if ch == '\'' && starts_character_literal(&chars, index) {
                    mask_char(&mut output, ch);
                    index += 1;
                    state = State::Character { escaped: false };
                    continue;
                }

                output.push(ch);
                index += 1;
            }
            State::LineComment => {
                mask_char(&mut output, ch);
                index += 1;
                if ch == '\n' {
                    state = State::Code;
                }
            }
            State::BlockComment(depth) => {
                if ch == '/' && next == Some('*') {
                    output.push_str("  ");
                    index += 2;
                    state = State::BlockComment(depth + 1);
                } else if ch == '*' && next == Some('/') {
                    output.push_str("  ");
                    index += 2;
                    state = if depth == 1 {
                        State::Code
                    } else {
                        State::BlockComment(depth - 1)
                    };
                } else {
                    mask_char(&mut output, ch);
                    index += 1;
                }
            }
            State::String { escaped } => {
                mask_char(&mut output, ch);
                index += 1;
                if escaped {
                    state = State::String { escaped: false };
                } else if ch == '\\' {
                    state = State::String { escaped: true };
                } else if ch == '"' {
                    state = State::Code;
                }
            }
            State::Character { escaped } => {
                mask_char(&mut output, ch);
                index += 1;
                if escaped {
                    state = State::Character { escaped: false };
                } else if ch == '\\' {
                    state = State::Character { escaped: true };
                } else if ch == '\'' {
                    state = State::Code;
                }
            }
            State::RawString { hashes } => {
                mask_char(&mut output, ch);
                index += 1;
                if ch == '"' {
                    let closes =
                        (0..hashes).all(|offset| chars.get(index + offset).copied() == Some('#'));
                    if closes {
                        for _ in 0..hashes {
                            mask_char(&mut output, '#');
                        }
                        index += hashes;
                        state = State::Code;
                    }
                }
            }
        }
    }

    output
}

fn strip_leading_attributes(mut line: &str) -> &str {
    loop {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("#[") {
            return trimmed;
        }
        let mut depth = 0usize;
        let mut end = None;
        for (index, ch) in trimmed.char_indices() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(index + ch.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            return trimmed;
        };
        line = &trimmed[end..];
    }
}

fn take_identifier(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    let mut end = 0;
    for (index, ch) in input.char_indices() {
        if (index == 0 && (ch == '_' || ch.is_alphabetic()))
            || (index > 0 && (ch == '_' || ch.is_alphanumeric()))
        {
            end = index + ch.len_utf8();
        } else {
            break;
        }
    }
    (end > 0).then(|| (&input[..end], &input[end..]))
}

fn consume_word<'a>(input: &'a str, word: &str) -> Option<&'a str> {
    let input = input.trim_start();
    let rest = input.strip_prefix(word)?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_alphanumeric())
    {
        None
    } else {
        Some(rest)
    }
}

fn function_declaration_name(line: &str) -> Option<String> {
    let mut input = strip_leading_attributes(line);

    if let Some(rest) = consume_word(input, "pub") {
        input = rest.trim_start();
        if input.starts_with('(') {
            let close = input.find(')')?;
            input = &input[close + 1..];
        }
    }

    loop {
        let mut consumed = false;
        for modifier in ["const", "async", "unsafe", "extern"] {
            if let Some(rest) = consume_word(input, modifier) {
                input = rest;
                consumed = true;
                break;
            }
        }
        if !consumed {
            break;
        }
    }

    let input = consume_word(input, "fn")?;
    let (name, _) = take_identifier(input)?;
    Some(name.to_string())
}

fn generated_function_name(line: &str) -> Option<String> {
    let input = strip_leading_attributes(line);
    for macro_name in FUNCTION_GENERATING_MACROS {
        let Some(rest) = input.strip_prefix(macro_name) else {
            continue;
        };
        let rest = rest.trim_start();
        let rest = rest.strip_prefix('!')?.trim_start();
        let opening = rest.chars().next()?;
        if !matches!(opening, '(' | '{' | '[') {
            return None;
        }
        let (name, _) = take_identifier(&rest[opening.len_utf8()..])?;
        return Some(name.to_string());
    }
    None
}

fn scan_top_level_functions(source: &str) -> BTreeSet<String> {
    let masked = mask_comments_and_literals(source);
    let mut brace_depth = 0usize;
    let mut functions = BTreeSet::new();

    for line in masked.lines() {
        if brace_depth == 0 {
            if let Some(name) =
                function_declaration_name(line).or_else(|| generated_function_name(line))
            {
                assert!(
                    functions.insert(name.clone()),
                    "source declares module-level function {name:?} more than once"
                );
            }
        }

        for ch in line.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth = brace_depth
                        .checked_sub(1)
                        .expect("unbalanced closing brace in scanned Rust source")
                }
                _ => {}
            }
        }
    }

    assert_eq!(brace_depth, 0, "unbalanced braces in scanned Rust source");
    functions
}

fn is_kebab_case(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn is_grammar_bearing(classes: &[&str]) -> bool {
    classes.iter().any(|class| {
        matches!(
            *class,
            "root" | "production" | "terminal" | "lexical-primitive"
        )
    })
}

fn inline_grammar_comment_definitions(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter_map(|(offset, line)| {
            let comment = line.trim_start().strip_prefix("//")?;
            let text = comment
                .trim_start_matches(|ch| ch == '/' || ch == '!')
                .trim_start();
            let (left_hand_side, _) = text.split_once(":=")?;
            let name = left_hand_side.trim();
            let mut chars = name.chars();
            let starts_like_name = chars
                .next()
                .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_');
            let is_name = starts_like_name
                && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
            is_name.then(|| (offset + 1, line.trim().to_string()))
        })
        .collect()
}

fn canonical_specification_rules(source: &str) -> BTreeSet<String> {
    const FENCE: &str = "```ebnf:canonical";
    assert_eq!(
        source.matches(FENCE).count(),
        1,
        "specification must contain exactly one canonical grammar fence"
    );
    let (_, after_opening) = source
        .split_once(FENCE)
        .expect("specification has no canonical grammar fence");
    let (grammar, _) = after_opening
        .split_once("```")
        .expect("canonical grammar fence is not closed");

    let mut rules = BTreeSet::new();
    for line in grammar.lines() {
        let Some((left_hand_side, _)) = line.split_once(":=") else {
            continue;
        };
        let name = left_hand_side.trim();
        if !is_kebab_case(name) {
            // A quoted terminal may itself contain `:=` on a continuation
            // line; only a kebab-case left-hand side starts a definition.
            continue;
        }
        assert!(
            rules.insert(name.to_string()),
            "canonical grammar defines {name:?} more than once"
        );
    }
    rules
}

fn cell<'a>(fields: &[&'a str], column_index: &BTreeMap<&str, usize>, name: &str) -> &'a str {
    fields[column_index[name]].trim()
}

#[test]
fn productions_inventory_covers_selected_parser_modules() {
    let manifest = fs::read_to_string(inventory_path()).expect("read productions.tsv");
    let mut lines = manifest.lines();
    let header = lines.next().expect("productions.tsv must have a header");
    let columns = header
        .trim_end_matches('\r')
        .split('\t')
        .collect::<Vec<_>>();

    assert_eq!(
        columns, REQUIRED_COLUMNS,
        "productions.tsv columns changed; update the audit and checker together"
    );

    let column_index = columns
        .iter()
        .enumerate()
        .map(|(index, name)| (*name, index))
        .collect::<BTreeMap<_, _>>();

    let mut inventoried = BTreeSet::new();
    let mut stable_ids = BTreeSet::new();
    let mut canonical_grammar_names = BTreeSet::new();
    let mut grammar_rows = 0usize;
    let mut mechanics_rows = 0usize;

    for (offset, raw_line) in lines.enumerate() {
        let line_number = offset + 2;
        let line = raw_line.trim_end_matches('\r');
        assert!(
            !line.trim().is_empty(),
            "blank TSV row at line {line_number}"
        );
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(
            fields.len(),
            columns.len(),
            "line {line_number} has {} cells; expected {}",
            fields.len(),
            columns.len()
        );

        let id = cell(&fields, &column_index, "id");
        let grammar_name = cell(&fields, &column_index, "grammar-name");
        let module = cell(&fields, &column_index, "module");
        let rust_function = cell(&fields, &column_index, "rust-function");
        let classification = cell(&fields, &column_index, "classification");
        let spec_location = cell(&fields, &column_index, "spec-location");
        let implementation_path = cell(&fields, &column_index, "implementation-path");

        assert!(!id.is_empty(), "line {line_number} has no stable ID");
        assert!(
            stable_ids.insert(id.to_string()),
            "stable ID {id:?} is claimed by more than one row"
        );
        assert!(
            MODULES.contains(&module),
            "line {line_number} names unscanned module {module:?}"
        );
        assert!(
            !rust_function.is_empty(),
            "line {line_number} has no Rust function"
        );
        assert!(
            inventoried.insert((module.to_string(), rust_function.to_string())),
            "two rows claim {module}::{rust_function}"
        );

        assert!(
            !classification.is_empty(),
            "line {line_number} has no classification"
        );
        let classes = classification.split('/').collect::<Vec<_>>();
        for class in &classes {
            assert!(
                CLASSIFICATIONS.contains(class),
                "line {line_number} has unknown classification {class:?}"
            );
        }

        if is_grammar_bearing(&classes) {
            grammar_rows += 1;
            assert!(
                is_kebab_case(grammar_name),
                "canonical row {id} needs a kebab-case grammar name"
            );
            assert!(
                canonical_grammar_names.insert(grammar_name.to_string()),
                "canonical grammar name {grammar_name:?} is claimed by more than one row"
            );
            assert_eq!(
                spec_location,
                format!("{CANONICAL_SPECIFICATION}::{grammar_name}"),
                "canonical row {id} needs a stable specification reference"
            );
        } else {
            mechanics_rows += 1;
            assert!(
                grammar_name.is_empty() || is_kebab_case(grammar_name),
                "line {line_number} has invalid grammar name {grammar_name:?}"
            );
            assert_eq!(
                spec_location, "not-applicable",
                "mechanics/non-grammar row {id} must not claim a grammar location"
            );
        }

        let expected_path = format!("src/syntax/src/{module}.rs::{rust_function}");
        assert_eq!(
            implementation_path, expected_path,
            "line {line_number} has no canonical implementation path"
        );

        for required in [
            "feature-gate",
            "entry-point",
            "output-type",
            "termination",
            "whitespace",
        ] {
            assert!(
                !cell(&fields, &column_index, required).is_empty(),
                "line {line_number} has no {required}"
            );
        }

        let selection = cell(&fields, &column_index, "selection-behavior");
        assert!(
            !selection.is_empty(),
            "line {line_number} has no selection behavior"
        );
        for behavior in selection.split('/') {
            assert!(
                SELECTION_BEHAVIORS.contains(&behavior),
                "line {line_number} has unknown selection behavior {behavior:?}"
            );
        }
    }

    assert_eq!(
        stable_ids.len(),
        EXPECTED_INVENTORY_ROWS,
        "unexpected productions.tsv row count"
    );
    assert_eq!(
        grammar_rows, EXPECTED_GRAMMAR_ROWS,
        "unexpected grammar-bearing row count"
    );
    assert_eq!(
        mechanics_rows, EXPECTED_MECHANICS_ROWS,
        "unexpected mechanics/non-grammar row count"
    );

    let specification =
        fs::read_to_string(specification_path()).expect("read canonical specification");
    assert_eq!(
        canonical_grammar_names,
        canonical_specification_rules(&specification),
        "inventory grammar names and canonical specification rules differ"
    );

    let mut discovered = BTreeSet::new();
    for module in MODULES {
        let source_path = syntax_root().join(format!("src/{module}.rs"));
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", source_path.display()));
        for function in scan_top_level_functions(&source) {
            discovered.insert(((*module).to_string(), function));
        }
    }

    let missing = discovered.difference(&inventoried).collect::<Vec<_>>();
    let stale = inventoried.difference(&discovered).collect::<Vec<_>>();
    assert!(
        missing.is_empty() && stale.is_empty(),
        "parser inventory drifted\nmissing rows: {missing:#?}\nstale rows: {stale:#?}\n\
         discovered: {}\ninventoried: {}",
        discovered.len(),
        inventoried.len()
    );
}

#[test]
fn parser_sources_do_not_define_competing_inline_grammars() {
    let mut conflicts = Vec::new();
    for module in MODULES {
        let source_path = syntax_root().join(format!("src/{module}.rs"));
        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("read {}: {error}", source_path.display()));
        conflicts.extend(
            inline_grammar_comment_definitions(&source)
                .into_iter()
                .map(|(line, definition)| format!("{module}.rs:{line}: {definition}")),
        );
    }
    assert!(
        conflicts.is_empty(),
        "Rust parser sources contain competing inline grammar definitions; \
         keep formal grammar only in {CANONICAL_SPECIFICATION}:\n{}",
        conflicts.join("\n")
    );
}

#[test]
fn scanner_ignores_nested_and_non_code_functions() {
    let source = r##"
        // fn line_comment_fake() {}
        const EXAMPLE: &str = "fn string_fake() { }";
        const RAW: &str = r#"fn raw_string_fake() { }"#;
        type Callback = fn(u8) -> u8;

        pub(crate) async fn visible() -> impl Fn(u8) -> u8 {
            fn nested() {}
            |value| value
        }

        /* fn block_comment_fake() {
             /* fn nested_comment_fake() {} */
        } */

        leaf!(generated, "}", TokenKind::RightBrace);
    "##;

    assert_eq!(
        scan_top_level_functions(source),
        BTreeSet::from(["generated".to_string(), "visible".to_string()])
    );
}

#[test]
fn inline_grammar_comment_scanner_allows_ordinary_implementation_comments() {
    let source = r#"
        // Check `x := value` before trying the fallback.
        // This implementation recognizes the := token.
        // Example: x := 1
        let source = "https://example.com/rule := text";

        // expression := factor ;
        /// code-terminal := new-line | eof ;
    "#;

    assert_eq!(
        inline_grammar_comment_definitions(source),
        vec![
            (7, "// expression := factor ;".to_string()),
            (8, "/// code-terminal := new-line | eof ;".to_string()),
        ]
    );
}
