use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_CANONICAL_SOURCES: &[&str] = &[
  "mod.rs",
  "base.rs",
  "terminal_spec.rs",
  "combinator.rs",
  "found.rs",
  "grammar.rs",
  "roots.rs",
  "ports.rs",
  "mechdown.rs",
  "statements.rs",
  "test_support.rs",
  "literals.rs",
  "paths.rs",
  "kinds.rs",
  "operators.rs",
];

const PHASE_2B_PRODUCTION_SOURCES: &[&str] = &["mechdown.rs", "statements.rs"];
const PHASE_2C_PRODUCTION_SOURCES: &[&str] = &["literals.rs", "paths.rs", "kinds.rs"];
const PHASE_2D_PRODUCTION_SOURCES: &[&str] = &["operators.rs"];

fn canonical_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    .join("src/document/parser/canonical")
}

fn collect_rust_sources(directory: &Path, output: &mut Vec<PathBuf>) {
  for entry in fs::read_dir(directory)
    .unwrap_or_else(|error| panic!("failed to read {}: {error}", directory.display()))
  {
    let path = entry
      .unwrap_or_else(|error| {
        panic!("failed to read an entry below {}: {error}", directory.display())
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
    for forbidden in ["nom", "parse", "CoverageStore", "CoverageGap", "UnportedInline"] {
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
      &[
        "crate", "::", "document", "::", "parser", "::", "document",
      ][..],
      &["crate", "::", "document", "::", "parser", "::", "mech"][..],
      &[
        "crate", "::", "document", "::", "parser", "::", "mechdown",
      ][..],
      &[
        "crate", "::", "document", "::", "parser", "::", "structures",
      ][..],
      &[
        "crate", "::", "document", "::", "parser", "::", "patterns",
      ][..],
      &[
        "crate", "::", "document", "::", "parser", "::", "functions",
      ][..],
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
    assert_eq!(
      statement_calls,
      vec!["parse_comment_sigil"],
      "{relative} may depend on the Phase 2B canonical comment-sigil only"
    );
  }
}

#[test]
fn phase_2b_entry_points_bind_their_exact_generated_rule_ids() {
  let canonical = canonical_root();
  let expected = [
    ("statements.rs", "parse_comment_sigil", "COMMENT_SIGIL"),
    ("statements.rs", "parse_comment", "COMMENT"),
    ("mechdown.rs", "parse_codeblock_sigil", "CODEBLOCK_SIGIL"),
    ("mechdown.rs", "parse_inline_code", "INLINE_CODE"),
    ("mechdown.rs", "parse_inline_equation", "INLINE_EQUATION"),
    ("mechdown.rs", "parse_raw_hyperlink", "RAW_HYPERLINK"),
    ("mechdown.rs", "parse_footnote_reference", "FOOTNOTE_REFERENCE"),
    ("mechdown.rs", "parse_reference", "REFERENCE"),
    ("mechdown.rs", "parse_section_reference", "SECTION_REFERENCE"),
    ("mechdown.rs", "parse_paragraph_text", "PARAGRAPH_TEXT"),
    ("mechdown.rs", "parse_thematic_break", "THEMATIC_BREAK"),
    ("mechdown.rs", "parse_blank_line", "BLANK_LINE"),
    ("mechdown.rs", "parse_equation", "EQUATION"),
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
    ("literals.rs", "parse_empty", "EMPTY"),
    ("literals.rs", "parse_atom", "ATOM"),
    ("literals.rs", "parse_string", "STRING"),
    ("literals.rs", "parse_utf8_string", "UTF8_STRING"),
    ("literals.rs", "parse_raw_string", "RAW_STRING"),
    ("literals.rs", "parse_boolean", "BOOLEAN"),
    ("literals.rs", "parse_true_literal", "TRUE_LITERAL"),
    ("literals.rs", "parse_false_literal", "FALSE_LITERAL"),
    ("literals.rs", "parse_number", "NUMBER"),
    ("literals.rs", "parse_complex_number", "COMPLEX_NUMBER"),
    ("literals.rs", "parse_real_number", "REAL_NUMBER"),
    (
      "literals.rs",
      "parse_untyped_real_number",
      "UNTYPED_REAL_NUMBER",
    ),
    ("literals.rs", "parse_rational_literal", "RATIONAL_LITERAL"),
    (
      "literals.rs",
      "parse_scientific_literal",
      "SCIENTIFIC_LITERAL",
    ),
    (
      "literals.rs",
      "parse_float_decimal_start",
      "FLOAT_DECIMAL_START",
    ),
    ("literals.rs", "parse_float_full", "FLOAT_FULL"),
    ("literals.rs", "parse_float_literal", "FLOAT_LITERAL"),
    ("literals.rs", "parse_integer_literal", "INTEGER_LITERAL"),
    ("literals.rs", "parse_typed_integer", "TYPED_INTEGER"),
    ("literals.rs", "parse_untyped_integer", "UNTYPED_INTEGER"),
    ("literals.rs", "parse_decimal_literal", "DECIMAL_LITERAL"),
    (
      "literals.rs",
      "parse_hexadecimal_literal",
      "HEXADECIMAL_LITERAL",
    ),
    ("literals.rs", "parse_octal_literal", "OCTAL_LITERAL"),
    ("literals.rs", "parse_binary_literal", "BINARY_LITERAL"),
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
    ("operators.rs", "parse_add_sub_operator", "ADD_SUB_OPERATOR"),
    ("operators.rs", "parse_mul_div_operator", "MUL_DIV_OPERATOR"),
    ("operators.rs", "parse_power_operator", "POWER_OPERATOR"),
    ("operators.rs", "parse_matrix_operator", "MATRIX_OPERATOR"),
    ("operators.rs", "parse_range_operator", "RANGE_OPERATOR"),
    (
      "operators.rs",
      "parse_comparison_operator",
      "COMPARISON_OPERATOR",
    ),
    ("operators.rs", "parse_logic_operator", "LOGIC_OPERATOR"),
    ("operators.rs", "parse_table_operator", "TABLE_OPERATOR"),
    ("operators.rs", "parse_set_operator", "SET_OPERATOR"),
    ("operators.rs", "parse_add", "ADD"),
    ("operators.rs", "parse_subtract", "SUBTRACT"),
    ("operators.rs", "parse_raw_subtract", "RAW_SUBTRACT"),
    ("operators.rs", "parse_spaced_subtract", "SPACED_SUBTRACT"),
    ("operators.rs", "parse_multiply", "MULTIPLY"),
    ("operators.rs", "parse_divide", "DIVIDE"),
    ("operators.rs", "parse_modulus", "MODULUS"),
    ("operators.rs", "parse_power", "POWER"),
    ("operators.rs", "parse_matrix_multiply", "MATRIX_MULTIPLY"),
    ("operators.rs", "parse_matrix_solve", "MATRIX_SOLVE"),
    ("operators.rs", "parse_dot_product", "DOT_PRODUCT"),
    ("operators.rs", "parse_cross_product", "CROSS_PRODUCT"),
    ("operators.rs", "parse_transpose", "TRANSPOSE"),
    ("operators.rs", "parse_range_inclusive", "RANGE_INCLUSIVE"),
    ("operators.rs", "parse_range_exclusive", "RANGE_EXCLUSIVE"),
    ("operators.rs", "parse_not_equal", "NOT_EQUAL"),
    ("operators.rs", "parse_equal_to", "EQUAL_TO"),
    ("operators.rs", "parse_strict_not_equal", "STRICT_NOT_EQUAL"),
    ("operators.rs", "parse_strict_equal", "STRICT_EQUAL"),
    ("operators.rs", "parse_greater_than", "GREATER_THAN"),
    ("operators.rs", "parse_less_than", "LESS_THAN"),
    (
      "operators.rs",
      "parse_greater_than_equal",
      "GREATER_THAN_EQUAL",
    ),
    ("operators.rs", "parse_less_than_equal", "LESS_THAN_EQUAL"),
    ("operators.rs", "parse_or", "OR"),
    ("operators.rs", "parse_and", "AND"),
    ("operators.rs", "parse_not", "NOT"),
    ("operators.rs", "parse_xor", "XOR"),
    ("operators.rs", "parse_join", "JOIN"),
    ("operators.rs", "parse_left_join", "LEFT_JOIN"),
    ("operators.rs", "parse_right_join", "RIGHT_JOIN"),
    ("operators.rs", "parse_full_join", "FULL_JOIN"),
    ("operators.rs", "parse_left_semi_join", "LEFT_SEMI_JOIN"),
    ("operators.rs", "parse_left_anti_join", "LEFT_ANTI_JOIN"),
    ("operators.rs", "parse_union_op", "UNION_OP"),
    ("operators.rs", "parse_intersection", "INTERSECTION"),
    ("operators.rs", "parse_difference", "DIFFERENCE"),
    ("operators.rs", "parse_complement", "COMPLEMENT"),
    ("operators.rs", "parse_subset", "SUBSET"),
    ("operators.rs", "parse_superset", "SUPERSET"),
    ("operators.rs", "parse_proper_subset", "PROPER_SUBSET"),
    ("operators.rs", "parse_proper_superset", "PROPER_SUPERSET"),
    ("operators.rs", "parse_element_of", "ELEMENT_OF"),
    ("operators.rs", "parse_not_element_of", "NOT_ELEMENT_OF"),
    (
      "operators.rs",
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
fn parser_surface_has_only_the_two_phase_2a_root_pairs() {
  use mech_syntax::document::{
    DocumentId, ParseConfig, ParseRequestError, ParseRoot, ParserImplementation, Revision,
    TextSnapshot, parse_syntax,
  };

  let source = || TextSnapshot::new(DocumentId(205), Revision(0), "").unwrap();
  assert!(parse_syntax(
    source(),
    ParseRoot::Document,
    ParserImplementation::Prototype,
    ParseConfig::default(),
  )
  .is_ok());
  assert!(parse_syntax(
    source(),
    ParseRoot::Grammar,
    ParserImplementation::Canonical,
    ParseConfig::default(),
  )
  .is_ok());
  assert!(matches!(
    parse_syntax(
      source(),
      ParseRoot::Document,
      ParserImplementation::Canonical,
      ParseConfig::default(),
    ),
    Err(ParseRequestError::Unsupported {
      implementation: ParserImplementation::Canonical,
      root: ParseRoot::Document,
    })
  ));
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
fn migration_state_and_document_skeleton_sources_are_absent() {
  let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  for relative in [
    "src/document/coverage.rs",
    "src/document/parser/canonical/document.rs",
    "src/document/parser/canonical/migration.rs",
    "src/document/parser/physical.rs",
  ] {
    assert!(
      !manifest.join(relative).exists(),
      "Phase 2B must not retain {relative}"
    );
  }

  for relative in [
    "src/document/mod.rs",
    "src/document/parser/mod.rs",
    "src/document/parser/canonical/mechdown.rs",
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
  let path = canonical_root().join("mechdown.rs");
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
