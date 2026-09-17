use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_CANONICAL_SOURCES: &[&str] = &[
  "mod.rs",
  "base.rs",
  "terminal_spec.rs",
  "combinator.rs",
  "grammar.rs",
  "roots.rs",
  "ports.rs",
];

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

fn use_statement_imports_prototype(tokens: &[String], start: usize) -> bool {
  let end = tokens[start..]
    .iter()
    .position(|token| token == ";")
    .map(|offset| start + offset)
    .unwrap_or(tokens.len());
  let statement = &tokens[start..end];

  if statement
    .iter()
    .any(|token| matches!(token.as_str(), "mech" | "mechdown" | "parse_document"))
  {
    return true;
  }

  let parser = statement.iter().position(|token| token == "parser");
  parser.is_some_and(|parser| {
    statement[parser + 1..]
      .iter()
      .any(|token| token == "document")
  }) || (statement
    .iter()
    .filter(|token| token.as_str() == "super")
    .count()
    >= 2
    && statement.iter().any(|token| token == "document"))
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

    if matches!(token.as_str(), "mech" | "mechdown")
      && (tokens.get(index.wrapping_sub(1)).map(String::as_str) == Some("::")
        || tokens.get(index + 1).map(String::as_str) == Some("::")
        || tokens.get(index.wrapping_sub(1)).map(String::as_str) == Some("mod"))
    {
      violations.push("prototype parser module dependency");
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
