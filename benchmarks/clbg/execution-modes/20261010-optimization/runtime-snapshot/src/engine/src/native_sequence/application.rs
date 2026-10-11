//! Application syntax normalized before canonical parsing in the opt-in AOT
//! frontends. `value |> f(_, bound)` evaluates value once; all holes in a stage
//! refer to that value. `f(x<T>) => <U> := expression` is a typed function body.
//! Normalization creates ordinary bindings/functions and adds no native kernel.

fn identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '-' | '/')
}
fn identifier(source: &str) -> bool {
    !source.is_empty()
        && source.chars().all(identifier_char)
        && source.chars().any(|c| c.is_alphabetic() || c == '_')
}

/// Code bytes, with strings and comments masked, retain their original offsets.
/// The parser still receives the original strings/comments after rewriting.
fn mask(source: &str) -> String {
    let mut bytes = source.as_bytes().to_vec();
    let mut quoted = false;
    let mut escaped = false;
    let mut comment = false;
    let mut i = 0;
    while i < bytes.len() {
        let ch = source.as_bytes()[i];
        if comment {
            if ch == b'\n' {
                comment = false;
            } else {
                bytes[i] = b' ';
            }
        } else if quoted {
            bytes[i] = b' ';
            if escaped {
                escaped = false;
            } else if ch == b'\\' {
                escaped = true;
            } else if ch == b'"' {
                quoted = false;
                bytes[i] = b'Q';
            }
        } else if ch == b'"' {
            quoted = true;
            bytes[i] = b'Q';
        } else if (source.as_bytes()[i..].starts_with(b"--")
            || source.as_bytes()[i..].starts_with(b"//")
            || source.as_bytes()[i..].starts_with(b"%%"))
            && (i == 0 || source.as_bytes()[i - 1].is_ascii_whitespace())
        {
            comment = true;
            bytes[i] = b' ';
        }
        i += 1;
    }
    // Masking may replace only part of a UTF-8 sequence, so consume raw bytes.
    // All non-ASCII code bytes remain unchanged; quoted/comment bytes are ASCII.
    String::from_utf8(bytes).expect("masked source remains UTF-8")
}

fn compact(source: &str) -> String {
    let mut output = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut comment = false;
    for (offset, ch) in source.char_indices() {
        if comment {
            if ch == '\n' {
                comment = false;
                output.push(' ');
            }
        } else if quoted {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
        } else if ch == '"' {
            quoted = true;
            output.push(ch);
        } else if (source[offset..].starts_with("--")
            || source[offset..].starts_with("//")
            || source[offset..].starts_with("%%"))
            && (offset == 0 || source.as_bytes()[offset - 1].is_ascii_whitespace())
        {
            comment = true;
        } else if ch == '\n' || ch == '\r' {
            output.push(' ');
        } else {
            output.push(ch);
        }
    }
    output.trim().to_string()
}

fn top_offsets(source: &str, token: &str) -> Result<Vec<usize>, String> {
    let code = mask(source);
    let mut stack = Vec::new();
    let mut offsets = Vec::new();
    let mut i = 0;
    while i < code.len() {
        let ch = code.as_bytes()[i];
        if stack.is_empty() && code.as_bytes()[i..].starts_with(token.as_bytes()) {
            offsets.push(i);
            i += token.len();
            continue;
        }
        match ch {
            b'(' | b'[' | b'{' => stack.push(ch),
            b')' | b']' | b'}' => {
                let open = stack.pop().ok_or("unbalanced application expression")?;
                if !matches!((open, ch), (b'(', b')') | (b'[', b']') | (b'{', b'}')) {
                    return Err("mismatched application delimiters".into());
                }
            }
            _ => {}
        }
        i += 1;
    }
    if !stack.is_empty() {
        return Err("unbalanced application expression".into());
    }
    Ok(offsets)
}
fn split_top<'a>(source: &'a str, token: &str) -> Result<Vec<&'a str>, String> {
    let offsets = top_offsets(source, token)?;
    let mut parts = Vec::new();
    let mut start = 0;
    for i in offsets {
        parts.push(source[start..i].trim());
        start = i + token.len();
    }
    parts.push(source[start..].trim());
    Ok(parts)
}

fn substitute(stage: &str, value: &str) -> Result<String, String> {
    let code = mask(stage);
    let mut output = String::new();
    let mut start = 0;
    let mut holes = 0;
    for (i, ch) in code.char_indices() {
        if ch == '_'
            && (i == 0 || !code[..i].chars().next_back().is_some_and(identifier_char))
            && !code[i + 1..].chars().next().is_some_and(identifier_char)
        {
            output.push_str(&stage[start..i]);
            output.push_str(value);
            start = i + 1;
            holes += 1;
        }
    }
    output.push_str(&stage[start..]);
    if holes > 0 {
        return Ok(output);
    }
    if identifier(stage) {
        return Ok(format!("{stage}({value})"));
    }
    Err("pipeline stage requires an argument hole `_` or a function name".into())
}

fn unparenthesized(expression: &str) -> Result<&str, String> {
    let mut expression = expression.trim();
    loop {
        if !expression.starts_with('(') || !expression.ends_with(')') {
            return Ok(expression);
        }
        // Only unwrap a pair enclosing the entire expression, not `(a) + (b)`.
        let code = mask(expression);
        let mut depth = 0;
        let mut closes_early = false;
        for (i, ch) in code.bytes().enumerate() {
            match ch {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            if depth == 0 && i + 1 < code.len() {
                closes_early = true;
                break;
            }
        }
        if closes_early {
            return Ok(expression);
        }
        expression = expression[1..expression.len() - 1].trim();
    }
}
fn pipeline(
    expression: &str,
    prefix: &str,
    serial: &mut usize,
) -> Result<(Vec<String>, String), String> {
    let expression = unparenthesized(expression)?;
    let stages = split_top(expression, "|>")?;
    if stages.iter().any(|s| s.is_empty()) {
        return Err("empty pipeline stage".into());
    }
    let mut definitions = Vec::new();
    let mut value = stages[0].to_string();
    if mask(&value).contains("|>") {
        // Parenthesized initial pipelines can be flattened without moving them
        // across another operation or changing its evaluation/branch order.
        let input = unparenthesized(&value)?;
        if top_offsets(input, "|>")?.is_empty() {
            return Err("a nested pipeline must be a complete parenthesized expression".into());
        }
        let (nested, result) = pipeline(input, prefix, serial)?;
        definitions.extend(nested);
        value = result;
    }
    for stage in &stages[1..] {
        if mask(stage).contains("|>") {
            return Err("pipelines inside stage arguments require a separate binding".into());
        }
        let name = format!("{prefix}{}", *serial);
        *serial += 1;
        definitions.push(format!("{name} := {value}"));
        value = substitute(stage, &name)?;
    }
    Ok((definitions, value))
}

fn is_candidate(source: &str) -> bool {
    let code = mask(source);
    let code = code.trim();
    if code.starts_with("%%") || code.starts_with('#') || code.starts_with('`') {
        return false;
    }
    let initial = code
        .split(|c: char| !identifier_char(c))
        .next()
        .unwrap_or("");
    // Documentation/prose is not a definition just because it quotes an operator.
    identifier(initial)
        && (code[initial.len()..].trim_start().starts_with(":=")
            || code[initial.len()..].trim_start().starts_with('(')
            || code[initial.len()..].trim_start().starts_with("|>"))
}

fn normalize_statement(
    source: &str,
    prefix: &str,
    serial: &mut usize,
) -> Result<Option<String>, String> {
    let code = mask(source);
    if !code.contains("|>") && !code.contains("=>") {
        return Ok(None);
    }
    let compacted = compact(source);
    let source = compacted.as_str();
    let assignments = top_offsets(source, ":=")?;
    let (head, expression) = if let Some(&i) = assignments.first() {
        (&source[..i], &source[i + 2..])
    } else {
        if top_offsets(source, "|>")?.is_empty() {
            return Ok(None);
        }
        ("", source)
    };
    let signature = split_top(head.trim(), "=>")?;
    // Canonical match-arm functions use => but do not have := after the kind.
    // Canonical statement functions have their own := in the header; retain it.
    if signature.len() == 1 && !head.trim().is_empty() && !identifier(head.trim()) {
        return Ok(None);
    }
    let expression = expression.trim();
    let terminated = expression.ends_with('.');
    let expression = if terminated {
        expression[..expression.len() - 1].trim_end()
    } else {
        expression
    };
    if signature.len() == 1 && !mask(expression).contains("|>") {
        return Ok(None);
    }
    let (mut definitions, value) = pipeline(expression, prefix, serial)?;
    if signature.len() == 2 {
        let header = signature[0];
        let open = header
            .find('(')
            .ok_or("function requires a parameter list")?;
        if !identifier(header[..open].trim()) || !header.ends_with(')') {
            return Err("invalid expression-bodied function signature".into());
        }
        let kind = signature[1];
        if !kind.starts_with('<') || !kind.ends_with('>') {
            return Err("function requires an explicit output kind".into());
        }
        let result = format!("{prefix}result");
        definitions.push(format!("{result} := {value}"));
        Ok(Some(format!(
            "{header} = ({result}{kind}) := {}.",
            definitions.join("; ")
        )))
    } else if signature.len() == 1 {
        let value = if head.trim().is_empty() {
            value
        } else {
            format!("{} := {value}", head.trim())
        };
        definitions.push(value);
        let mut normalized = definitions.join("; ");
        if terminated {
            normalized.push('.');
        }
        Ok(Some(normalized))
    } else {
        Err("one return arrow is accepted per function definition".into())
    }
}

/// Preserve untouched canonical documents byte-for-byte. Generated statements
/// stay on the first physical source line, and blank lines keep later original
/// line numbers stable. Application errors report their original line number.
/// Parenthesized/multiline expressions and leading continuation pipes are allowed;
/// arbitrary pipelines inside conditionals/call arguments are rejected explicitly.
pub(super) fn normalize(source: &str) -> Result<String, String> {
    if !source.contains("|>") && !source.contains("=>") {
        return Ok(source.to_string());
    }
    let mut prefix = "application-temp-".to_string();
    while source.contains(&prefix) {
        prefix.push_str("local-");
    }
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut output = String::new();
    let mut serial = 0;
    let mut index = 0;
    let mut fence: Option<bool> = None;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            if fence.is_some() {
                fence = None;
            } else {
                let language = trimmed[3..].trim();
                fence = Some(matches!(language, "mech" | "mec"));
            }
            output.push_str(line);
            index += 1;
            continue;
        }
        if fence == Some(false) || !is_candidate(line) {
            output.push_str(line);
            index += 1;
            continue;
        }
        let start = index;
        let mut statement = line.to_string();
        loop {
            let code = mask(&statement);
            let unbalanced = top_offsets(&statement, "|>").is_err();
            let continuation = code.trim_end().ends_with("|>")
                || (code.trim_end().ends_with(":=")
                    && (code.contains("=>")
                        || code
                            .split(":=")
                            .next()
                            .is_some_and(|s| identifier(s.trim()))))
                || lines
                    .get(index + 1)
                    .is_some_and(|s| mask(s).trim_start().starts_with("|>"));
            if !(unbalanced || continuation) || index + 1 >= lines.len() {
                break;
            }
            index += 1;
            statement.push_str(lines[index]);
        }
        let normalized = normalize_statement(&statement, &prefix, &mut serial)
            .map_err(|e| format!("function application line {}: {e}", start + 1))?;
        if let Some(normalized) = normalized {
            output.push_str(&normalized);
            // Preserve the physical line count and final newline of this block.
            for _ in normalized.bytes().filter(|b| *b == b'\n').count()
                ..statement.bytes().filter(|b| *b == b'\n').count()
            {
                output.push('\n');
            }
        } else {
            output.push_str(&statement);
        }
        index += 1;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn holes_share_one_evaluation_and_ignore_strings_and_identifiers() {
        let n = normalize("r := input() |> f(_, g(_), \"_|>\", some_name)\n").unwrap();
        assert_eq!(
            n,
            "application-temp-0 := input(); r := f(application-temp-0, g(application-temp-0), \"_|>\", some_name)\n"
        );
    }
    #[test]
    fn canonical_and_documented_operators_are_unchanged() {
        let source = "Make(depth<u64>) => <ref>\n  | 0 => memory/null().\nx := (true ? | true => 1 | * => 2).\nlabel := \"|> =>\"\n-- r := x |> f\n// r := x |> f\n%% r := x |> f\nUse `r := x |> f` in examples.\n```rust\nr := x |> f\n```\n";
        assert_eq!(normalize(source).unwrap(), source);
    }
    #[test]
    fn typed_functions_unary_stages_and_periods() {
        let n = normalize("f(x<f64>) => <f64> := x |> math/sqrt.\ny := 4.0 |> f\n").unwrap();
        assert!(n.contains("f(x<f64>) = (application-temp-result<f64>)"));
        assert!(n.contains("math/sqrt(application-temp-0)."));
        assert!(n.contains("y := f(application-temp-1)"));
        assert_eq!(normalize(&n).unwrap(), n);
    }
    #[test]
    fn multiline_calls_and_continuations_keep_original_lines() {
        let source = "f(x<[f64]>) => <[f64]> :=\n  x\n  |> g(\n     _, h(_), \"→\")\n  |> k\nr := input()\n  |> f\n";
        let n = normalize(source).unwrap();
        assert!(n.contains("f(x<[f64]>) = (application-temp-result<[f64]>)"));
        assert!(n.contains("r := f(application-temp-2)"));
        assert_eq!(n.lines().count(), source.lines().count());
    }
    #[test]
    fn parenthesized_input_and_effect_pipeline() {
        let n = normalize("(x |> f) |> g\nimage |> image/pbm |> io/write\n");
        // A standalone parenthesized expression is deliberately not recognized
        // as document code; immutable definitions can contain parenthesized input.
        assert!(n.unwrap().contains("io/write(application-temp-1)"));
        let n = normalize("r := (input() |> f) |> g\n").unwrap();
        assert_eq!(n.matches("input()").count(), 1);
        assert!(n.contains("application-temp-1 := f(application-temp-0)"));
    }
    #[test]
    fn no_unchecked_hoisting_from_conditionals_or_arguments() {
        assert!(
            normalize("r := f(input(), (x |> g))\n")
                .unwrap_err()
                .contains("nested pipeline")
        );
        assert!(
            normalize("r := x |> f(_, (y |> g))\n")
                .unwrap_err()
                .contains("stage arguments")
        );
        assert!(normalize("r := 1.0 |> math/sqrt(4.0)\n").is_err());
        assert!(normalize("r := 1.0 |>\n").is_err());
    }
    #[test]
    fn temporary_names_are_hygienic_and_diagnostics_reference_original_lines() {
        let n = normalize("application-temp-0 := 1\nr := application-temp-0 |> f\n").unwrap();
        assert!(n.contains("application-temp-local-0"));
        assert!(
            normalize("-- intro\nr := input()\n  |> f(4.0)\n")
                .unwrap_err()
                .starts_with("function application line 2:")
        );
    }
    #[test]
    fn canonical_function_terminator_is_retained() {
        let n = normalize("f(x<f64>) = (out<f64>) :=\n  out := x |> math/sqrt.\n").unwrap();
        assert!(n.ends_with("out := math/sqrt(application-temp-0).\n"));
    }
}
