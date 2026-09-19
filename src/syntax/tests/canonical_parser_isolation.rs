use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_CANONICAL_SOURCES: &[&str] = &[
    "mod.rs",
    "base/mod.rs",
    "base/continuation.rs",
    "terminal_spec.rs",
    "combinator.rs",
    "found/mod.rs",
    "found/source.rs",
    "grammar.rs",
    "roots.rs",
    "ports.rs",
    "mechdown/mod.rs",
    "mechdown/continuation.rs",
    "prose.rs",
    "statements.rs",
    "test_support.rs",
    "literals/mod.rs",
    "literals/continuation.rs",
    "strings.rs",
    "paths.rs",
    "kinds.rs",
    "operators/mod.rs",
    "operators/spec.rs",
    "operators/continuation.rs",
    "imports/mod.rs",
    "imports/continuation.rs",
    "source_imports/mod.rs",
    "source_imports/continuation.rs",
    "declarations/mod.rs",
    "declarations/continuation.rs",
    "document/mod.rs",
    "document/continuation.rs",
    "document_grammar.rs",
    "subscript_primitives.rs",
    "pattern_primitives.rs",
    "control_operators.rs",
    "primitives.rs",
    "structure_shell/mod.rs",
    "structure_shell/continuation.rs",
];

const PHASE_2B_PRODUCTION_SOURCES: &[&str] = &[
    "mechdown/mod.rs",
    "mechdown/continuation.rs",
    "prose.rs",
    "statements.rs",
];
const PHASE_2C_PRODUCTION_SOURCES: &[&str] = &[
    "literals/mod.rs",
    "literals/continuation.rs",
    "strings.rs",
    "paths.rs",
    "kinds.rs",
];
const PHASE_2D_PRODUCTION_SOURCES: &[&str] = &[
    "operators/mod.rs",
    "operators/spec.rs",
    "operators/continuation.rs",
];
const PHASE_2E_PRODUCTION_SOURCES: &[&str] = &["imports/mod.rs", "imports/continuation.rs"];
const PHASE_2F_PRODUCTION_SOURCES: &[&str] = &[
    "source_imports/mod.rs",
    "source_imports/continuation.rs",
    "declarations/mod.rs",
    "declarations/continuation.rs",
];
const PHASE_2G_PRODUCTION_SOURCES: &[&str] = &[
    "subscript_primitives.rs",
    "pattern_primitives.rs",
    "control_operators.rs",
    "primitives.rs",
];
const PHASE_2H_PRODUCTION_SOURCES: &[&str] =
    &["structure_shell/mod.rs", "structure_shell/continuation.rs"];
const PHASE_2I_PRODUCTION_SOURCES: &[&str] = &[
    "recursive_core/calls.rs",
    "recursive_core/comprehensions.rs",
    "recursive_core/expressions.rs",
    "recursive_core/fsm.rs",
    "recursive_core/kinds/continuation.rs",
    "recursive_core/kinds/mod.rs",
    "recursive_core/literals.rs",
    "recursive_core/mod.rs",
    "recursive_core/patterns.rs",
    "recursive_core/precedence/continuation/brace.rs",
    "recursive_core/precedence/continuation/bracket.rs",
    "recursive_core/precedence/continuation/collection.rs",
    "recursive_core/precedence/continuation/comprehension.rs",
    "recursive_core/precedence/continuation/definition.rs",
    "recursive_core/precedence/continuation/entry.rs",
    "recursive_core/precedence/continuation/expression.rs",
    "recursive_core/precedence/continuation/fsm.rs",
    "recursive_core/precedence/continuation/inline.rs",
    "recursive_core/precedence/continuation/map.rs",
    "recursive_core/precedence/continuation/mapping_probe.rs",
    "recursive_core/precedence/continuation/match_arm.rs",
    "recursive_core/precedence/continuation/matrix_row.rs",
    "recursive_core/precedence/continuation/mod.rs",
    "recursive_core/precedence/continuation/parenthesis.rs",
    "recursive_core/precedence/continuation/pattern.rs",
    "recursive_core/precedence/continuation/postfix.rs",
    "recursive_core/precedence/continuation/record.rs",
    "recursive_core/precedence/continuation/table.rs",
    "recursive_core/precedence/continuation/table_row.rs",
    "recursive_core/precedence/mod.rs",
    "recursive_core/required.rs",
    "recursive_core/structures/continuation.rs",
    "recursive_core/structures/mod.rs",
    "recursive_core/subscripts.rs",
    "recursive_core/variables.rs",
];

fn canonical_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/document/parser/canonical")
}

fn collect_rust_sources(directory: &Path, output: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
    {
        let path = entry
            .unwrap_or_else(|error| {
                panic!(
                    "failed to read an entry below {}: {error}",
                    directory.display()
                )
            })
            .path();
        if path.is_dir() {
            collect_rust_sources(&path, output);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            output.push(path);
        }
    }
}

fn blank_range(output: &mut [u8], start: usize, end: usize) {
    for byte in &mut output[start..end] {
        if *byte != b'\n' {
            *byte = b' ';
        }
    }
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'r') {
        return None;
    }
    let mut delimiter = start + 1;
    while bytes.get(delimiter) == Some(&b'#') {
        delimiter += 1;
    }
    if bytes.get(delimiter) != Some(&b'"') {
        return None;
    }
    let hashes = delimiter - start - 1;
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

fn quoted_string_end(bytes: &[u8], start: usize) -> usize {
    let mut cursor = start + 1;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\\' => cursor = (cursor + 2).min(bytes.len()),
            b'"' => return cursor + 1,
            _ => cursor += 1,
        }
    }
    bytes.len()
}

fn character_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
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
        let character = std::str::from_utf8(bytes.get(cursor..)?)
            .ok()?
            .chars()
            .next()?;
        cursor += character.len_utf8();
    }
    (bytes.get(cursor) == Some(&b'\'')).then_some(cursor + 1)
}

fn mask_comments_and_literals(source: &str, mask_literals: bool) -> String {
    let bytes = source.as_bytes();
    let mut output = bytes.to_vec();
    let mut cursor = 0;

    while cursor < bytes.len() {
        if bytes.get(cursor..cursor + 2) == Some(b"//") {
            let start = cursor;
            cursor += 2;
            while cursor < bytes.len() && bytes[cursor] != b'\n' {
                cursor += 1;
            }
            blank_range(&mut output, start, cursor);
            continue;
        }

        if bytes.get(cursor..cursor + 2) == Some(b"/*") {
            let start = cursor;
            let mut depth = 1_usize;
            cursor += 2;
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
            blank_range(&mut output, start, cursor);
            continue;
        }

        if let Some(end) = raw_string_end(bytes, cursor) {
            if mask_literals {
                blank_range(&mut output, cursor, end);
            }
            cursor = end;
            continue;
        }

        if bytes[cursor] == b'"' {
            let end = quoted_string_end(bytes, cursor);
            if mask_literals {
                blank_range(&mut output, cursor, end);
            }
            cursor = end;
            continue;
        }

        if bytes[cursor] == b'\''
            && let Some(end) = character_literal_end(bytes, cursor)
        {
            if mask_literals {
                blank_range(&mut output, cursor, end);
            }
            cursor = end;
            continue;
        }

        cursor += 1;
    }

    String::from_utf8(output).expect("masking Rust source must preserve UTF-8")
}

fn rust_tokens(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = cursor;
            cursor += 1;
            while cursor < bytes.len()
                && (bytes[cursor].is_ascii_alphanumeric() || bytes[cursor] == b'_')
            {
                cursor += 1;
            }
            tokens.push(source[start..cursor].to_owned());
        } else if bytes.get(cursor..cursor + 2) == Some(b"::") {
            tokens.push("::".to_owned());
            cursor += 2;
        } else if matches!(byte, b'.' | b'(' | b')' | b'{' | b'}' | b',' | b';') {
            tokens.push(char::from(byte).to_string());
            cursor += 1;
        } else {
            cursor += 1;
        }
    }
    tokens
}

fn matching_close(tokens: &[String], open: usize) -> Option<usize> {
    let mut depth = 0_usize;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        match token.as_str() {
            "(" => depth += 1,
            ")" => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn invocation(tokens: &[String], index: usize) -> Option<(usize, usize)> {
    if tokens.get(index + 1).map(String::as_str) != Some("(") {
        return None;
    }
    let close = matching_close(tokens, index + 1)?;
    Some((index + 2, close))
}

fn named_function_body<'a>(source: &'a str, name: &str) -> &'a str {
    let masked = mask_comments_and_literals(source, true);
    let signature = format!("fn {name}(");
    let function = masked
        .find(&signature)
        .unwrap_or_else(|| panic!("missing function {name}"));
    let open = masked[function..]
        .find('{')
        .map(|relative| function + relative)
        .unwrap_or_else(|| panic!("missing body for function {name}"));
    let mut depth = 0_usize;
    for (relative, byte) in masked.as_bytes()[open..].iter().enumerate() {
        match byte {
            b'{' => depth = depth.saturating_add(1),
            b'}' => {
                depth = depth
                    .checked_sub(1)
                    .unwrap_or_else(|| panic!("unbalanced body for function {name}"));
                if depth == 0 {
                    return &source[open + 1..open + relative];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated body for function {name}")
}

fn contains_token(source: &str, expected: &str) -> bool {
    rust_tokens(&mask_comments_and_literals(source, true))
        .iter()
        .any(|token| token == expected)
}

fn contains_token_sequence(source: &str, expected: &[&str]) -> bool {
    let tokens = rust_tokens(&mask_comments_and_literals(source, true));
    tokens.windows(expected.len()).any(|window| {
        window
            .iter()
            .map(String::as_str)
            .eq(expected.iter().copied())
    })
}

fn use_statement_imports_prototype(tokens: &[String], start: usize) -> bool {
    let end = tokens[start..]
        .iter()
        .position(|token| token == ";")
        .map(|offset| start + offset)
        .unwrap_or(tokens.len());
    let statement = &tokens[start..end];

    if statement.iter().any(|token| token == "parse_document") {
        return true;
    }

    let prototype_target =
        |token: &str| matches!(token, "document" | "mech" | "mechdown" | "statements");
    let parser = statement.iter().position(|token| token == "parser");
    parser.is_some_and(|parser| {
        statement[parser + 1..]
            .iter()
            .any(|token| prototype_target(token))
    }) || (statement
        .iter()
        .filter(|token| token.as_str() == "super")
        .count()
        >= 2
        && statement.iter().any(|token| prototype_target(token)))
}

fn executable_violations(source: &str) -> Vec<&'static str> {
    let executable = mask_comments_and_literals(source, true);
    let tokens = rust_tokens(&executable);
    let mut violations = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        if token == "use" && use_statement_imports_prototype(&tokens, index) {
            violations.push("prototype parser import");
        }

        if token == "parse_document" {
            violations.push("prototype parse_document dependency");
        }

        if matches!(token.as_str(), "mech" | "mechdown" | "statements") {
            let follows_parser =
                index >= 2 && tokens[index - 1] == "::" && tokens[index - 2] == "parser";
            let follows_two_supers = index >= 4
                && tokens[index - 1] == "::"
                && tokens[index - 2] == "super"
                && tokens[index - 3] == "::"
                && tokens[index - 4] == "super";
            if follows_parser || follows_two_supers {
                violations.push("prototype parser module dependency");
            }
        }

        if token == "document" {
            let follows_parser =
                index >= 2 && tokens[index - 1] == "::" && tokens[index - 2] == "parser";
            let follows_two_supers = index >= 4
                && tokens[index - 1] == "::"
                && tokens[index - 2] == "super"
                && tokens[index - 3] == "::"
                && tokens[index - 4] == "super";
            if follows_parser || follows_two_supers {
                violations.push("prototype document parser dependency");
            }
        }

        if token == "to_contiguous_string"
            && invocation(&tokens, index).is_some()
            && tokens.get(index.wrapping_sub(1)).map(String::as_str) != Some("fn")
        {
            violations.push("complete-source to_contiguous_string extraction");
        }

        if token == "graphemes"
            && invocation(&tokens, index).is_some()
            && matches!(
                tokens.get(index.wrapping_sub(1)).map(String::as_str),
                Some(".") | Some("::")
            )
        {
            violations.push("whole-source Unicode grapheme traversal");
        }

        if token == "text"
            && matches!(
                tokens.get(index.wrapping_sub(1)).map(String::as_str),
                Some(".") | Some("::")
            )
            && let Some((arguments, close)) = invocation(&tokens, index)
            && tokens[arguments..close]
                .iter()
                .any(|argument| argument == "full_range")
        {
            violations.push("complete-source full_range text extraction");
        }
    }

    violations.sort_unstable();
    violations.dedup();
    violations
}

#[test]
fn canonical_phase_2b_sources_are_present_and_directly_isolated() {
    let canonical = canonical_root();
    for relative in PHASE_2B_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            executable_violations(&source).is_empty(),
            "{relative} has a prototype dependency: {:?}",
            executable_violations(&source)
        );
        for forbidden in [
            "nom",
            "parse",
            "CoverageStore",
            "CoverageGap",
            "UnportedInline",
        ] {
            if forbidden == "parse" {
                assert!(
                    !contains_token_sequence(&source, &["crate", "::", "parse"]),
                    "{relative} must not invoke the legacy public parser"
                );
            } else {
                assert!(
                    !contains_token(&source, forbidden),
                    "{relative} must not depend on {forbidden}"
                );
            }
        }
        for path in [
            &["super", "::", "super", "::", "document"][..],
            &["super", "::", "super", "::", "mech"][..],
            &["super", "::", "super", "::", "mechdown"][..],
        ] {
            assert!(
                !contains_token_sequence(&source, path),
                "{relative} must not import a prototype production module"
            );
        }
    }
}

#[test]
fn canonical_phase_2c_sources_are_present_and_directly_isolated() {
    let canonical = canonical_root();
    for relative in PHASE_2C_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            executable_violations(&source).is_empty(),
            "{relative} has a prototype dependency: {:?}",
            executable_violations(&source)
        );
        for forbidden in ["nom", "CoverageStore", "CoverageGap", "UnportedInline"] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not depend on {forbidden}"
            );
        }
        assert!(
            !contains_token_sequence(&source, &["crate", "::", "parse"]),
            "{relative} must not invoke the legacy public parser"
        );
        for path in [
            &["super", "::", "super", "::", "document"][..],
            &["super", "::", "super", "::", "mech"][..],
            &["super", "::", "super", "::", "mechdown"][..],
            &["crate", "::", "document", "::", "parser", "::", "document"][..],
            &["crate", "::", "document", "::", "parser", "::", "mech"][..],
            &["crate", "::", "document", "::", "parser", "::", "mechdown"][..],
            &["crate", "::", "literals"][..],
            &["crate", "::", "expressions"][..],
            &["crate", "::", "document", "::", "incremental"][..],
        ] {
            assert!(
                !contains_token_sequence(&source, path),
                "{relative} must not import or call a deferred parser module"
            );
        }
        for forbidden in ["to_contiguous_string", "full_range", "graphemes"] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not materialize or globally segment source through {forbidden}"
            );
        }
    }
}

#[test]
fn canonical_phase_2d_operator_source_is_present_and_directly_isolated() {
    let canonical = canonical_root();
    for relative in PHASE_2D_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            executable_violations(&source).is_empty(),
            "{relative} has a prototype dependency: {:?}",
            executable_violations(&source)
        );
        for forbidden in ["nom", "CoverageStore", "CoverageGap", "UnportedInline"] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not depend on {forbidden}"
            );
        }
        assert!(
            !contains_token_sequence(&source, &["crate", "::", "parse"]),
            "{relative} must not invoke the legacy public parser"
        );
        for path in [
            &["super", "::", "super", "::", "document"][..],
            &["super", "::", "super", "::", "mech"][..],
            &["super", "::", "super", "::", "mechdown"][..],
            &["crate", "::", "document", "::", "parser", "::", "document"][..],
            &["crate", "::", "document", "::", "parser", "::", "mech"][..],
            &["crate", "::", "document", "::", "parser", "::", "mechdown"][..],
            &[
                "crate",
                "::",
                "document",
                "::",
                "parser",
                "::",
                "structures",
            ][..],
            &["crate", "::", "document", "::", "parser", "::", "patterns"][..],
            &["crate", "::", "document", "::", "parser", "::", "functions"][..],
            &[
                "crate",
                "::",
                "document",
                "::",
                "parser",
                "::",
                "state_machines",
            ][..],
            &["crate", "::", "expressions"][..],
            &["crate", "::", "structures"][..],
            &["crate", "::", "patterns"][..],
            &["crate", "::", "functions"][..],
            &["crate", "::", "state_machines"][..],
            &["crate", "::", "document", "::", "incremental"][..],
        ] {
            assert!(
                !contains_token_sequence(&source, path),
                "{relative} must not import or call a deferred parser module"
            );
        }
        for forbidden in ["to_contiguous_string", "full_range", "graphemes"] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not materialize or globally segment source through {forbidden}"
            );
        }

        let tokens = rust_tokens(&mask_comments_and_literals(&source, true));
        let statement_calls = tokens
            .windows(3)
            .filter_map(|window| {
                (window[0] == "statements" && window[1] == "::").then_some(window[2].as_str())
            })
            .collect::<Vec<_>>();
        assert!(
            statement_calls
                .iter()
                .all(|name| matches!(*name, "Continuation" | "Progress")),
            "{relative} may use only the retained canonical comment-sigil owner"
        );
        if *relative == "operators/continuation.rs" {
            let constructors = tokens
                .windows(10)
                .filter(|window| {
                    window[..6] == ["statements", "::", "Continuation", "::", "new", "("]
                })
                .collect::<Vec<_>>();
            assert_eq!(constructors.len(), 1, "one canonical sigil guard owner");
            assert_eq!(
                &constructors[0][6..10],
                ["rules", "::", "COMMENT_SIGIL", ")"]
            );
        } else {
            assert!(
                statement_calls.is_empty(),
                "{relative} must not select comment rules"
            );
        }
    }
}

#[test]
fn canonical_phase_2e_import_source_is_present_and_directly_isolated() {
    let canonical = canonical_root();
    for relative in PHASE_2E_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            executable_violations(&source).is_empty(),
            "{relative} has a prototype dependency: {:?}",
            executable_violations(&source)
        );
        for forbidden in ["nom", "CoverageStore", "CoverageGap", "UnportedInline"] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not depend on {forbidden}"
            );
        }
        assert!(
            !contains_token_sequence(&source, &["crate", "::", "parse"]),
            "{relative} must not invoke the legacy public parser"
        );
        for path in [
            &["super", "::", "super", "::", "document"][..],
            &["super", "::", "super", "::", "mech"][..],
            &["super", "::", "super", "::", "mechdown"][..],
            &["super", "::", "super", "::", "imports"][..],
            &["super", "::", "super", "::", "statements"][..],
            &["crate", "::", "document", "::", "parser", "::", "document"][..],
            &["crate", "::", "document", "::", "parser", "::", "mech"][..],
            &["crate", "::", "document", "::", "parser", "::", "mechdown"][..],
            &["crate", "::", "document", "::", "parser", "::", "imports"][..],
            &[
                "crate",
                "::",
                "document",
                "::",
                "parser",
                "::",
                "statements",
            ][..],
            &[
                "crate",
                "::",
                "document",
                "::",
                "parser",
                "::",
                "structures",
            ][..],
            &["crate", "::", "document", "::", "parser", "::", "patterns"][..],
            &["crate", "::", "document", "::", "parser", "::", "functions"][..],
            &[
                "crate",
                "::",
                "document",
                "::",
                "parser",
                "::",
                "state_machines",
            ][..],
            &["crate", "::", "expressions"][..],
            &["crate", "::", "structures"][..],
            &["crate", "::", "patterns"][..],
            &["crate", "::", "functions"][..],
            &["crate", "::", "state_machines"][..],
            &["crate", "::", "document", "::", "incremental"][..],
        ] {
            assert!(
                !contains_token_sequence(&source, path),
                "{relative} must not import or call a deferred parser module"
            );
        }
        for forbidden in ["to_contiguous_string", "full_range", "graphemes"] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not materialize or globally segment source through {forbidden}"
            );
        }
        for rule in [
            "MODULE_IMPORT_SIGIL",
            "MODULE_IMPORT_END",
            "IMPORT_DECLARATION",
            "SOURCE_IMPORT_SPECIFIER",
            "STATEMENT",
            "MECH_CODE_ALT",
            "MECH_CODE",
            "CODE_TERMINAL",
            "EXPRESSION",
        ] {
            assert!(
                !contains_token_sequence(&source, &["rules", "::", rule]),
                "{relative} must not claim deferred rules::{rule}"
            );
        }
    }
}

#[test]
fn canonical_phase_2f_sources_are_present_and_directly_isolated() {
    let canonical = canonical_root();
    for relative in PHASE_2F_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            executable_violations(&source).is_empty(),
            "{relative} has a prototype dependency: {:?}",
            executable_violations(&source)
        );
        for forbidden in ["nom", "CoverageStore", "CoverageGap", "UnportedInline"] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not depend on {forbidden}"
            );
        }
        assert!(
            !contains_token_sequence(&source, &["crate", "::", "parse"]),
            "{relative} must not invoke the legacy public parser"
        );
        for forbidden in ["to_contiguous_string", "full_range", "graphemes"] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not materialize or globally segment source through {forbidden}"
            );
        }
        for rule in [
            "STATEMENT",
            "MECH_CODE_ALT",
            "MECH_CODE",
            "CODE_TERMINAL",
            "EXPRESSION",
            "DOCUMENT",
            "BODY",
            "PROGRAM",
            "PARSE",
            "CONTEXT_SEND",
            "VARIABLE_DEFINE",
            "VARIABLE_ASSIGN",
            "OP_ASSIGN",
            "ENUM_DEFINE",
            "KIND_DEFINE",
            "FSM_DECLARE",
        ] {
            assert!(
                !contains_token_sequence(&source, &["rules", "::", rule]),
                "{relative} must not claim deferred rules::{rule}"
            );
        }
    }
}

#[test]
fn phase_2f_entry_points_bind_their_exact_generated_rule_ids() {
    let canonical = canonical_root();
    let expected = [
        (
            "source_imports/mod.rs",
            "parse_source_import_tail",
            "SOURCE_IMPORT_TAIL",
        ),
        (
            "source_imports/mod.rs",
            "parse_source_path_component_token",
            "SOURCE_PATH_COMPONENT_TOKEN",
        ),
        (
            "source_imports/mod.rs",
            "parse_source_path_component",
            "SOURCE_PATH_COMPONENT",
        ),
        (
            "source_imports/mod.rs",
            "parse_source_mec_path",
            "SOURCE_MEC_PATH",
        ),
        (
            "source_imports/mod.rs",
            "parse_source_mec_path_wildcard_suffix",
            "SOURCE_MEC_PATH_WILDCARD_SUFFIX",
        ),
        (
            "source_imports/mod.rs",
            "parse_relative_source_import_specifier",
            "RELATIVE_SOURCE_IMPORT_SPECIFIER",
        ),
        (
            "source_imports/mod.rs",
            "parse_absolute_source_import_specifier",
            "ABSOLUTE_SOURCE_IMPORT_SPECIFIER",
        ),
        (
            "source_imports/mod.rs",
            "parse_bare_source_import_specifier",
            "BARE_SOURCE_IMPORT_SPECIFIER",
        ),
        (
            "source_imports/mod.rs",
            "parse_uri_scheme_part",
            "URI_SCHEME_PART",
        ),
        (
            "source_imports/mod.rs",
            "parse_source_import_uri_scheme",
            "SOURCE_IMPORT_URI_SCHEME",
        ),
        (
            "source_imports/mod.rs",
            "parse_uri_source_import_specifier",
            "URI_SOURCE_IMPORT_SPECIFIER",
        ),
        (
            "source_imports/mod.rs",
            "parse_source_import_specifier",
            "SOURCE_IMPORT_SPECIFIER",
        ),
        (
            "source_imports/mod.rs",
            "parse_import_declaration",
            "IMPORT_DECLARATION",
        ),
        (
            "declarations/mod.rs",
            "parse_export_declaration",
            "EXPORT_DECLARATION",
        ),
        (
            "declarations/mod.rs",
            "parse_context_declaration",
            "CONTEXT_DECLARATION",
        ),
        (
            "declarations/mod.rs",
            "parse_context_base_context",
            "CONTEXT_BASE_CONTEXT",
        ),
        (
            "declarations/mod.rs",
            "parse_context_base_resource_uri",
            "CONTEXT_BASE_RESOURCE_URI",
        ),
        (
            "declarations/mod.rs",
            "parse_context_capability_declaration",
            "CONTEXT_CAPABILITY_DECLARATION",
        ),
        (
            "declarations/mod.rs",
            "parse_context_capability_path_token",
            "CONTEXT_CAPABILITY_PATH_TOKEN",
        ),
        (
            "declarations/mod.rs",
            "parse_context_capability_path",
            "CONTEXT_CAPABILITY_PATH",
        ),
        (
            "declarations/mod.rs",
            "parse_context_capability_scope",
            "CONTEXT_CAPABILITY_SCOPE",
        ),
    ];
    assert_eq!(expected.len(), 21);
    for (file, function, rule) in expected {
        let path = canonical.join(file);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let body = named_function_body(&source, function);
        assert!(
            contains_token_sequence(body, &["rules", "::", rule]),
            "{file}::{function} does not bind rules::{rule}"
        );
    }
}

#[test]
fn canonical_phase_2g_sources_are_present_and_directly_isolated() {
    let canonical = canonical_root();
    for relative in PHASE_2G_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            executable_violations(&source).is_empty(),
            "{relative} has a prototype dependency: {:?}",
            executable_violations(&source)
        );
        for forbidden in [
            "nom",
            "CoverageStore",
            "CoverageGap",
            "UnportedInline",
            "to_contiguous_string",
            "full_range",
            "graphemes",
        ] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not depend on {forbidden}"
            );
        }
        assert!(
            !contains_token_sequence(&source, &["crate", "::", "parse"]),
            "{relative} must not invoke the legacy public parser"
        );
        for rule in [
            "SUBSCRIPT",
            "BRACKET_SUBSCRIPT",
            "BRACE_SUBSCRIPT",
            "FORMULA_SUBSCRIPT",
            "RANGE_SUBSCRIPT",
            "SLICE",
            "SLICE_REF",
            "PATTERN",
            "PATTERN_ARRAY",
            "PATTERN_ARRAY_ITEM",
            "PATTERN_ARRAY_TOKEN",
            "PATTERN_TUPLE",
            "PATTERN_TUPLE_STRUCT",
            "PATTERN_ATOM_STRUCT",
            "CONTEXT_SEND",
            "OP_ASSIGN",
            "VARIABLE_ASSIGN",
            "VARIABLE_DEFINE",
            "TUPLE_DESTRUCTURE",
            "STATEMENT",
            "MATCH_ARM",
            "MATCH_EXPRESSION",
            "FSM_GUARD",
            "FSM_STATE_DEFINITION",
            "FSM_TRANSITION",
            "ACTIVATION_ARM",
            "FORMULA",
            "FACTOR",
            "EXPRESSION",
        ] {
            assert!(
                !contains_token_sequence(&source, &["rules", "::", rule]),
                "{relative} must not claim the unported rules::{rule} parent"
            );
        }
    }
}

#[test]
fn phase_2g_entry_points_bind_their_exact_generated_rule_ids() {
    let canonical = canonical_root();
    let expected = [
        ("subscript_primitives.rs", "parse_select_all", "SELECT_ALL"),
        (
            "subscript_primitives.rs",
            "parse_swizzle_subscript",
            "SWIZZLE_SUBSCRIPT",
        ),
        (
            "subscript_primitives.rs",
            "parse_dot_subscript",
            "DOT_SUBSCRIPT",
        ),
        (
            "subscript_primitives.rs",
            "parse_dot_subscript_int",
            "DOT_SUBSCRIPT_INT",
        ),
        ("pattern_primitives.rs", "parse_wildcard", "WILDCARD"),
        (
            "pattern_primitives.rs",
            "parse_spread_operator",
            "SPREAD_OPERATOR",
        ),
        (
            "control_operators.rs",
            "parse_statement_separator",
            "STATEMENT_SEPARATOR",
        ),
        (
            "control_operators.rs",
            "parse_op_assign_operator",
            "OP_ASSIGN_OPERATOR",
        ),
        (
            "control_operators.rs",
            "parse_add_assign_operator",
            "ADD_ASSIGN_OPERATOR",
        ),
        (
            "control_operators.rs",
            "parse_sub_assign_operator",
            "SUB_ASSIGN_OPERATOR",
        ),
        (
            "control_operators.rs",
            "parse_mul_assign_operator",
            "MUL_ASSIGN_OPERATOR",
        ),
        (
            "control_operators.rs",
            "parse_div_assign_operator",
            "DIV_ASSIGN_OPERATOR",
        ),
        (
            "control_operators.rs",
            "parse_exp_assign_operator",
            "EXP_ASSIGN_OPERATOR",
        ),
        (
            "control_operators.rs",
            "parse_send_operator",
            "SEND_OPERATOR",
        ),
        (
            "control_operators.rs",
            "parse_guard_operator",
            "GUARD_OPERATOR",
        ),
    ];
    assert_eq!(expected.len(), 15);
    for (file, function, rule) in expected {
        let path = canonical.join(file);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let body = named_function_body(&source, function);
        assert!(
            contains_token_sequence(body, &["rules", "::", rule]),
            "{file}::{function} does not bind rules::{rule}"
        );
    }
}

#[test]
fn canonical_phase_2h_source_is_present_and_directly_isolated() {
    let canonical = canonical_root();
    for relative in PHASE_2H_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            executable_violations(&source).is_empty(),
            "{relative} has a prototype dependency: {:?}",
            executable_violations(&source)
        );
        for forbidden in [
            "nom",
            "CoverageStore",
            "CoverageGap",
            "UnportedInline",
            "to_contiguous_string",
            "full_range",
            "graphemes",
        ] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not depend on {forbidden}"
            );
        }
        assert!(
            !contains_token_sequence(&source, &["crate", "::", "parse"]),
            "{relative} must not invoke the legacy public parser"
        );
        for rule in [
            "STRUCTURE",
            "MATRIX",
            "MATRIX_ROW",
            "MATRIX_COLUMN",
            "TABLE",
            "INLINE_TABLE",
            "REGULAR_TABLE",
            "FANCY_TABLE",
            "TABLE_HEADER",
            "INLINE_TABLE_HEADER",
            "FANCY_TABLE_HEADER",
            "TABLE_ROW",
            "TABLE_ROW2",
            "TABLE_COLUMN",
            "HEADER_FIELD",
            "FIELD",
            "MAP",
            "MAPPING",
            "SET",
            "EXPRESSION",
            "KIND_ANNOTATION",
            "DOCUMENT",
        ] {
            assert!(
                !contains_token_sequence(&source, &["rules", "::", rule]),
                "{relative} must not claim the unported rules::{rule} parent"
            );
        }
    }
}

#[test]
fn phase_2h_entry_points_bind_their_exact_generated_rule_ids() {
    let canonical = canonical_root();
    let expected = [
        (
            "structure_shell/mod.rs",
            "parse_matrix_start",
            "MATRIX_START",
        ),
        ("structure_shell/mod.rs", "parse_matrix_end", "MATRIX_END"),
        ("structure_shell/mod.rs", "parse_table_start", "TABLE_START"),
        ("structure_shell/mod.rs", "parse_table_end", "TABLE_END"),
        (
            "structure_shell/mod.rs",
            "parse_table_separator",
            "TABLE_SEPARATOR",
        ),
        ("structure_shell/mod.rs", "parse_table_horz", "TABLE_HORZ"),
        ("structure_shell/mod.rs", "parse_table_top", "TABLE_TOP"),
        (
            "structure_shell/mod.rs",
            "parse_row_separator",
            "ROW_SEPARATOR",
        ),
        ("structure_shell/mod.rs", "parse_empty_map", "EMPTY_MAP"),
        ("structure_shell/mod.rs", "parse_empty_set", "EMPTY_SET"),
    ];
    assert_eq!(expected.len(), 10);
    for (file, function, rule) in expected {
        let path = canonical.join(file);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let body = named_function_body(&source, function);
        assert!(
            contains_token_sequence(body, &["rules", "::", rule]),
            "{file}::{function} does not bind rules::{rule}"
        );
    }
}

#[test]
fn canonical_phase_2i_sources_are_present_and_directly_isolated() {
    let canonical = canonical_root();
    for relative in PHASE_2I_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            executable_violations(&source).is_empty(),
            "{relative} has a prototype dependency: {:?}",
            executable_violations(&source)
        );
        for forbidden in [
            "nom",
            "ParseString",
            "ParseResult",
            "unsafe",
            "to_contiguous_string",
            "full_range",
            "graphemes",
            "missing_token",
            "skip_error",
            "abandon_error",
        ] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} must not depend on {forbidden}"
            );
        }
        assert!(
            !contains_token_sequence(&source, &["crate", "::", "parse"]),
            "{relative} must not invoke the legacy public parser"
        );
        if *relative != "recursive_core/mod.rs" {
            for recovery_api in [
                "insert_missing",
                "abandon_to_delimiter",
                "abandon_to_restart",
            ] {
                assert!(
                    !contains_token(&source, recovery_api),
                    "{relative} must use the recursive-core recovery boundary instead of {recovery_api} directly"
                );
            }
        }
    }
}

#[test]
fn phase_2b_entry_points_bind_their_exact_generated_rule_ids() {
    let canonical = canonical_root();
    let expected = [
        ("statements.rs", "parse_comment_sigil", "COMMENT_SIGIL"),
        ("statements.rs", "parse_comment", "COMMENT"),
        (
            "mechdown/mod.rs",
            "parse_codeblock_sigil",
            "CODEBLOCK_SIGIL",
        ),
        ("mechdown/mod.rs", "parse_inline_code", "INLINE_CODE"),
        (
            "mechdown/mod.rs",
            "parse_inline_equation",
            "INLINE_EQUATION",
        ),
        ("mechdown/mod.rs", "parse_raw_hyperlink", "RAW_HYPERLINK"),
        (
            "mechdown/mod.rs",
            "parse_footnote_reference",
            "FOOTNOTE_REFERENCE",
        ),
        ("mechdown/mod.rs", "parse_reference", "REFERENCE"),
        (
            "mechdown/mod.rs",
            "parse_section_reference",
            "SECTION_REFERENCE",
        ),
        ("mechdown/mod.rs", "parse_paragraph_text", "PARAGRAPH_TEXT"),
        ("mechdown/mod.rs", "parse_thematic_break", "THEMATIC_BREAK"),
        ("mechdown/mod.rs", "parse_blank_line", "BLANK_LINE"),
        ("mechdown/mod.rs", "parse_equation", "EQUATION"),
    ];

    for (file, function, rule) in expected {
        let path = canonical.join(file);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let body = named_function_body(&source, function);
        assert!(
            contains_token_sequence(body, &["rules", "::", rule]),
            "{file}::{function} does not bind rules::{rule}"
        );
    }
}

#[test]
fn phase_2c_entry_points_bind_their_exact_generated_rule_ids() {
    let canonical = canonical_root();
    let expected = [
        ("literals/mod.rs", "parse_empty", "EMPTY"),
        ("literals/mod.rs", "parse_atom", "ATOM"),
        ("literals/mod.rs", "parse_string", "STRING"),
        ("literals/mod.rs", "parse_utf8_string", "UTF8_STRING"),
        ("literals/mod.rs", "parse_raw_string", "RAW_STRING"),
        ("literals/mod.rs", "parse_boolean", "BOOLEAN"),
        ("literals/mod.rs", "parse_true_literal", "TRUE_LITERAL"),
        ("literals/mod.rs", "parse_false_literal", "FALSE_LITERAL"),
        ("literals/mod.rs", "parse_number", "NUMBER"),
        ("literals/mod.rs", "parse_complex_number", "COMPLEX_NUMBER"),
        ("literals/mod.rs", "parse_real_number", "REAL_NUMBER"),
        (
            "literals/mod.rs",
            "parse_untyped_real_number",
            "UNTYPED_REAL_NUMBER",
        ),
        (
            "literals/mod.rs",
            "parse_rational_literal",
            "RATIONAL_LITERAL",
        ),
        (
            "literals/mod.rs",
            "parse_scientific_literal",
            "SCIENTIFIC_LITERAL",
        ),
        (
            "literals/mod.rs",
            "parse_float_decimal_start",
            "FLOAT_DECIMAL_START",
        ),
        ("literals/mod.rs", "parse_float_full", "FLOAT_FULL"),
        ("literals/mod.rs", "parse_float_literal", "FLOAT_LITERAL"),
        (
            "literals/mod.rs",
            "parse_integer_literal",
            "INTEGER_LITERAL",
        ),
        ("literals/mod.rs", "parse_typed_integer", "TYPED_INTEGER"),
        (
            "literals/mod.rs",
            "parse_untyped_integer",
            "UNTYPED_INTEGER",
        ),
        (
            "literals/mod.rs",
            "parse_decimal_literal",
            "DECIMAL_LITERAL",
        ),
        (
            "literals/mod.rs",
            "parse_hexadecimal_literal",
            "HEXADECIMAL_LITERAL",
        ),
        ("literals/mod.rs", "parse_octal_literal", "OCTAL_LITERAL"),
        ("literals/mod.rs", "parse_binary_literal", "BINARY_LITERAL"),
        (
            "paths.rs",
            "parse_context_address_path_token",
            "CONTEXT_ADDRESS_PATH_TOKEN",
        ),
        (
            "paths.rs",
            "parse_context_address_path",
            "CONTEXT_ADDRESS_PATH",
        ),
        (
            "paths.rs",
            "parse_prefixed_context_path",
            "PREFIXED_CONTEXT_PATH",
        ),
        ("kinds.rs", "parse_kind_any", "KIND_ANY"),
        ("kinds.rs", "parse_kind_empty", "KIND_EMPTY"),
        ("kinds.rs", "parse_kind_atom", "KIND_ATOM"),
    ];
    assert_eq!(expected.len(), 30);

    for (file, function, rule) in expected {
        let path = canonical.join(file);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let body = named_function_body(&source, function);
        assert!(
            contains_token_sequence(body, &["rules", "::", rule]),
            "{file}::{function} does not bind rules::{rule}"
        );
    }
}

#[test]
fn phase_2d_entry_points_bind_their_exact_generated_rule_ids() {
    let canonical = canonical_root();
    let expected = [
        (
            "operators/mod.rs",
            "parse_add_sub_operator",
            "ADD_SUB_OPERATOR",
        ),
        (
            "operators/mod.rs",
            "parse_mul_div_operator",
            "MUL_DIV_OPERATOR",
        ),
        ("operators/mod.rs", "parse_power_operator", "POWER_OPERATOR"),
        (
            "operators/mod.rs",
            "parse_matrix_operator",
            "MATRIX_OPERATOR",
        ),
        ("operators/mod.rs", "parse_range_operator", "RANGE_OPERATOR"),
        (
            "operators/mod.rs",
            "parse_comparison_operator",
            "COMPARISON_OPERATOR",
        ),
        ("operators/mod.rs", "parse_logic_operator", "LOGIC_OPERATOR"),
        ("operators/mod.rs", "parse_table_operator", "TABLE_OPERATOR"),
        ("operators/mod.rs", "parse_set_operator", "SET_OPERATOR"),
        ("operators/mod.rs", "parse_add", "ADD"),
        ("operators/mod.rs", "parse_subtract", "SUBTRACT"),
        ("operators/mod.rs", "parse_raw_subtract", "RAW_SUBTRACT"),
        (
            "operators/mod.rs",
            "parse_spaced_subtract",
            "SPACED_SUBTRACT",
        ),
        ("operators/mod.rs", "parse_multiply", "MULTIPLY"),
        ("operators/mod.rs", "parse_divide", "DIVIDE"),
        ("operators/mod.rs", "parse_modulus", "MODULUS"),
        ("operators/mod.rs", "parse_power", "POWER"),
        (
            "operators/mod.rs",
            "parse_matrix_multiply",
            "MATRIX_MULTIPLY",
        ),
        ("operators/mod.rs", "parse_matrix_solve", "MATRIX_SOLVE"),
        ("operators/mod.rs", "parse_dot_product", "DOT_PRODUCT"),
        ("operators/mod.rs", "parse_cross_product", "CROSS_PRODUCT"),
        ("operators/mod.rs", "parse_transpose", "TRANSPOSE"),
        (
            "operators/mod.rs",
            "parse_range_inclusive",
            "RANGE_INCLUSIVE",
        ),
        (
            "operators/mod.rs",
            "parse_range_exclusive",
            "RANGE_EXCLUSIVE",
        ),
        ("operators/mod.rs", "parse_not_equal", "NOT_EQUAL"),
        ("operators/mod.rs", "parse_equal_to", "EQUAL_TO"),
        (
            "operators/mod.rs",
            "parse_strict_not_equal",
            "STRICT_NOT_EQUAL",
        ),
        ("operators/mod.rs", "parse_strict_equal", "STRICT_EQUAL"),
        ("operators/mod.rs", "parse_greater_than", "GREATER_THAN"),
        ("operators/mod.rs", "parse_less_than", "LESS_THAN"),
        (
            "operators/mod.rs",
            "parse_greater_than_equal",
            "GREATER_THAN_EQUAL",
        ),
        (
            "operators/mod.rs",
            "parse_less_than_equal",
            "LESS_THAN_EQUAL",
        ),
        ("operators/mod.rs", "parse_or", "OR"),
        ("operators/mod.rs", "parse_and", "AND"),
        ("operators/mod.rs", "parse_not", "NOT"),
        ("operators/mod.rs", "parse_xor", "XOR"),
        ("operators/mod.rs", "parse_join", "JOIN"),
        ("operators/mod.rs", "parse_left_join", "LEFT_JOIN"),
        ("operators/mod.rs", "parse_right_join", "RIGHT_JOIN"),
        ("operators/mod.rs", "parse_full_join", "FULL_JOIN"),
        ("operators/mod.rs", "parse_left_semi_join", "LEFT_SEMI_JOIN"),
        ("operators/mod.rs", "parse_left_anti_join", "LEFT_ANTI_JOIN"),
        ("operators/mod.rs", "parse_union_op", "UNION_OP"),
        ("operators/mod.rs", "parse_intersection", "INTERSECTION"),
        ("operators/mod.rs", "parse_difference", "DIFFERENCE"),
        ("operators/mod.rs", "parse_complement", "COMPLEMENT"),
        ("operators/mod.rs", "parse_subset", "SUBSET"),
        ("operators/mod.rs", "parse_superset", "SUPERSET"),
        ("operators/mod.rs", "parse_proper_subset", "PROPER_SUBSET"),
        (
            "operators/mod.rs",
            "parse_proper_superset",
            "PROPER_SUPERSET",
        ),
        ("operators/mod.rs", "parse_element_of", "ELEMENT_OF"),
        ("operators/mod.rs", "parse_not_element_of", "NOT_ELEMENT_OF"),
        (
            "operators/mod.rs",
            "parse_symmetric_difference",
            "SYMMETRIC_DIFFERENCE",
        ),
    ];
    assert_eq!(expected.len(), 53);

    for (file, function, rule) in expected {
        let path = canonical.join(file);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let body = named_function_body(&source, function);
        assert!(
            contains_token_sequence(body, &["rules", "::", rule]),
            "{file}::{function} does not bind rules::{rule}"
        );
    }
}

#[test]
fn phase_2e_entry_points_bind_their_exact_generated_rule_ids() {
    let canonical = canonical_root();
    let expected = [
        (
            "imports/mod.rs",
            "parse_module_import_name_segment",
            "MODULE_IMPORT_NAME_SEGMENT",
        ),
        (
            "imports/mod.rs",
            "parse_module_import_intrinsic_segment",
            "MODULE_IMPORT_INTRINSIC_SEGMENT",
        ),
        (
            "imports/mod.rs",
            "parse_module_import_path_segment",
            "MODULE_IMPORT_PATH_SEGMENT",
        ),
        (
            "imports/mod.rs",
            "parse_module_import_path",
            "MODULE_IMPORT_PATH",
        ),
        (
            "imports/mod.rs",
            "parse_module_import_alias_segment",
            "MODULE_IMPORT_ALIAS_SEGMENT",
        ),
        (
            "imports/mod.rs",
            "parse_module_import_alias_path",
            "MODULE_IMPORT_ALIAS_PATH",
        ),
        (
            "imports/mod.rs",
            "parse_module_import_value_alias",
            "MODULE_IMPORT_VALUE_ALIAS",
        ),
        (
            "imports/mod.rs",
            "parse_context_import_alias_segment",
            "CONTEXT_IMPORT_ALIAS_SEGMENT",
        ),
        (
            "imports/mod.rs",
            "parse_module_import_context_alias",
            "MODULE_IMPORT_CONTEXT_ALIAS",
        ),
        (
            "imports/mod.rs",
            "parse_module_import_alias",
            "MODULE_IMPORT_ALIAS",
        ),
        ("imports/mod.rs", "parse_module_root", "MODULE_ROOT"),
        (
            "imports/mod.rs",
            "parse_import_alias_operator",
            "IMPORT_ALIAS_OPERATOR",
        ),
        (
            "imports/mod.rs",
            "parse_import_group_separator",
            "IMPORT_GROUP_SEPARATOR",
        ),
        (
            "imports/mod.rs",
            "parse_import_group_item",
            "IMPORT_GROUP_ITEM",
        ),
        (
            "imports/mod.rs",
            "parse_import_group_items",
            "IMPORT_GROUP_ITEMS",
        ),
        (
            "imports/mod.rs",
            "parse_aliased_item_import",
            "ALIASED_ITEM_IMPORT",
        ),
        (
            "imports/mod.rs",
            "parse_module_suffix_import",
            "MODULE_SUFFIX_IMPORT",
        ),
        (
            "imports/mod.rs",
            "parse_module_only_import",
            "MODULE_ONLY_IMPORT",
        ),
        ("imports/mod.rs", "parse_module_import", "MODULE_IMPORT"),
    ];
    assert_eq!(expected.len(), 19);

    for (file, function, rule) in expected {
        let path = canonical.join(file);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let body = named_function_body(&source, function);
        assert!(
            contains_token_sequence(body, &["rules", "::", rule]),
            "{file}::{function} does not bind rules::{rule}"
        );
    }
}

#[test]
fn phase_2c_sources_do_not_reference_unported_recursive_parent_rules() {
    let canonical = canonical_root();
    let unported_parents = [
        "LITERAL",
        "VAR",
        "KIND",
        "KIND_ANNOTATION",
        "KIND_WITH_OPTION",
        "KIND_KIND",
        "KIND_TABLE",
        "KIND_SET",
        "KIND_MAP",
        "KIND_RECORD",
        "KIND_MATRIX",
        "KIND_TUPLE",
        "KIND_SCALAR",
        "RANGE_EXPRESSION",
        "FORMULA",
        "FACTOR",
        "EXPRESSION",
    ];

    for relative in PHASE_2C_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for rule in unported_parents {
            assert!(
                !contains_token_sequence(&source, &["rules", "::", rule]),
                "{relative} must not claim the unported rules::{rule} parent"
            );
        }
    }
}

#[test]
fn phase_2d_sources_do_not_reference_unported_expression_parent_rules() {
    let canonical = canonical_root();
    let unported_parents = [
        "EXPRESSION",
        "FORMULA",
        "L1",
        "L2",
        "L3",
        "L4",
        "L5",
        "L6",
        "L7",
        "FACTOR",
        "RANGE_EXPRESSION",
        "PARENTHETICAL_TERM",
        "NEGATE_FACTOR",
        "NOT_FACTOR",
        "STRUCTURE",
        "FUNCTION_CALL",
        "LITERAL",
        "SLICE",
        "VAR",
    ];
    assert_eq!(unported_parents.len(), 19);

    for relative in PHASE_2D_PRODUCTION_SOURCES {
        let path = canonical.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for rule in unported_parents {
            assert!(
                !contains_token_sequence(&source, &["rules", "::", rule]),
                "{relative} must not claim the unported rules::{rule} parent"
            );
        }
    }
}

#[test]
fn parser_surface_adds_only_the_s7_canonical_document_pair() {
    use mech_syntax::document::{
        DocumentId, ParseConfig, ParseRequestError, ParseRoot, ParserImplementation, Revision,
        TextSnapshot, parse_syntax,
    };

    let source = || TextSnapshot::new(DocumentId(205), Revision(0), "").unwrap();
    assert!(
        parse_syntax(
            source(),
            ParseRoot::Document,
            ParserImplementation::Prototype,
            ParseConfig::default(),
        )
        .is_ok()
    );
    assert!(
        parse_syntax(
            source(),
            ParseRoot::Grammar,
            ParserImplementation::Canonical,
            ParseConfig::default(),
        )
        .is_ok()
    );
    assert!(
        parse_syntax(
            source(),
            ParseRoot::Document,
            ParserImplementation::Canonical,
            ParseConfig::default(),
        )
        .is_ok()
    );
    assert!(matches!(
        parse_syntax(
            source(),
            ParseRoot::Grammar,
            ParserImplementation::Prototype,
            ParseConfig::default(),
        ),
        Err(ParseRequestError::Unsupported {
            implementation: ParserImplementation::Prototype,
            root: ParseRoot::Grammar,
        })
    ));

    let parser_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/document/parser/mod.rs");
    let parser = fs::read_to_string(&parser_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", parser_path.display()));
    let variants = parser
        .split("pub enum ParseRoot")
        .nth(1)
        .and_then(|tail| tail.split('}').next())
        .map(|body| {
            rust_tokens(body)
                .into_iter()
                .filter(|token| {
                    token
                        .chars()
                        .next()
                        .is_some_and(|character| character.is_ascii_alphabetic())
                })
                .collect::<Vec<_>>()
        })
        .expect("ParseRoot declaration");
    assert_eq!(variants, vec!["Document", "Grammar"]);
}

#[test]
fn removed_migration_state_and_document_skeleton_types_stay_absent() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/document/coverage.rs",
        "src/document/parser/canonical/migration.rs",
        "src/document/parser/physical.rs",
    ] {
        assert!(
            !manifest.join(relative).exists(),
            "S7 must not retain {relative}"
        );
    }

    for relative in [
        "src/document/mod.rs",
        "src/document/parser/mod.rs",
        "src/document/parser/canonical/mechdown/mod.rs",
        "src/document/parser/canonical/statements.rs",
    ] {
        let path = manifest.join(relative);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for forbidden in [
            "parse_canonical_document_skeleton",
            "DocumentSkeleton",
            "DocumentRegion",
            "DocumentItem",
            "UnresolvedDocumentItem",
            "CoverageStore",
            "CoverageGap",
            "UNPORTED",
            "CONTAINS_UNPORTED",
            "consume_document_newline",
            "synthetic_document_newline_emitted",
            "KNOWN_BLOCK_PREFIXES",
            "HeadingBoundary",
            "TitleBoundary",
            "UnportedMechItem",
            "UnportedBlock",
            "UnportedInline",
            "ParseRequest",
        ] {
            assert!(
                !contains_token(&source, forbidden),
                "{relative} retains forbidden parser state {forbidden}"
            );
        }
    }

    let document_mod = manifest.join("src/document/mod.rs");
    let document_mod = fs::read_to_string(&document_mod).unwrap();
    assert!(
        !contains_token(&document_mod, "LexicalMode"),
        "LexicalMode must remain parser-internal"
    );
}

#[test]
fn direct_leaf_parsers_do_not_materialize_source_root_newlines() {
    let path = canonical_root().join("mechdown/mod.rs");
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    for function in ["parse_blank_line", "parse_thematic_break"] {
        let body = named_function_body(&source, function);
        for forbidden in ["consume_document_newline", "synthetic", "SYNTHETIC"] {
            assert!(
                !contains_token(body, forbidden),
                "{function} must require a physical new-line rather than materializing one"
            );
        }
    }
}

#[test]
fn lexical_classification_depends_on_mode_not_resource_rule() {
    let parser_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/document/parser/mod.rs");
    let parser = fs::read_to_string(&parser_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", parser_path.display()));
    let body = named_function_body(&parser, "found_syntax");
    assert!(contains_token(body, "lexical_mode"));
    assert!(
        !contains_token(body, "resource_rule"),
        "resource-rule attribution must not select lexical classification"
    );
}

#[test]
fn canonical_parser_production_sources_are_isolated() {
    let canonical = canonical_root();
    for required in REQUIRED_CANONICAL_SOURCES {
        assert!(
            canonical.join(required).is_file(),
            "missing canonical parser production source {required}"
        );
    }

    let mut sources = Vec::new();
    collect_rust_sources(&canonical, &mut sources);
    sources.sort();
    assert!(
        !sources.is_empty(),
        "canonical parser production source set is empty"
    );

    let mut failures = Vec::new();
    for path in sources {
        let relative = path
            .strip_prefix(&canonical)
            .expect("canonical source must remain below its root")
            .to_string_lossy()
            .replace('\\', "/");

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for violation in executable_violations(&source) {
            failures.push(format!("{relative}: {violation}"));
        }
    }

    assert!(
        failures.is_empty(),
        "canonical parser isolation violations:\n{}",
        failures.join("\n")
    );
}

#[test]
fn isolation_scanner_distinguishes_comments_and_test_names_from_calls() {
    let harmless = r#"
    // source.to_contiguous_string();
    /* UnicodeSegmentation::graphemes(complete_source, true); */
    fn rejects_to_contiguous_string_without_calling_it() {}
    const NOTE: &str = "parse_document is test data";
  "#;
    assert!(executable_violations(harmless).is_empty());

    for prohibited in [
        "fn parse(source: &TextSnapshot) { source.to_contiguous_string(); }",
        "fn parse(source: &TextSnapshot) { source.text(source.full_range()); }",
        "fn parse(source: &str) { UnicodeSegmentation::graphemes(source, true); }",
        "fn parse() { crate::document::parser::mech::parse_expression(); }",
        "fn parse() { use crate::document::parser::{document as prototype}; }",
        "fn parse(source: TextSnapshot) { parse_document(source, config); }",
    ] {
        assert!(
            !executable_violations(prohibited).is_empty(),
            "scanner missed prohibited production code: {prohibited}"
        );
    }
}
