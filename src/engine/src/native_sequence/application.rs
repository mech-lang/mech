//! Application syntax normalized before canonical parsing in the opt-in AOT
//! frontends. `value |> f(_, bound)` evaluates value once; all holes in a stage
//! refer to that value. `f(x<T>) => <U> := expression` is a typed function body.
//! Indented bodies use the same signature: local definitions and mutations
//! precede the final expression, binding, or local assignment result.
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
        // Commas inside this pair denote a tuple, not scalar grouping. Keep
        // that pair when normalizing its result or a pipeline input.
        if !top_offsets(&expression[1..expression.len() - 1], ",")?.is_empty() {
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

fn arrow_definition(source: &str) -> bool {
    let Ok(assignments) = top_offsets(source, ":=") else {
        return false;
    };
    assignments.first().is_some_and(|&offset| {
        split_top(&source[..offset], "=>").is_ok_and(|parts| parts.len() == 2)
    })
}

fn indentation(line: &str) -> usize {
    line.bytes()
        .take_while(|ch| matches!(ch, b' ' | b'\t'))
        .fold(0, |width, ch| {
            if ch == b'\t' {
                (width / 8 + 1) * 8
            } else {
                width + 1
            }
        })
}

/// Keep parenthesized multiline expressions and continuation pipes together,
/// while retaining ordinary statement order in a function body.
fn body_statements(source: &str) -> Result<Vec<String>, String> {
    let mut statements: Vec<String> = Vec::new();
    for line in split_top(source, "\n")? {
        for part in split_top(line, ";")? {
            let part = compact(part);
            if part.is_empty() {
                continue;
            }
            let continuation = part.starts_with("|>")
                || statements.last().is_some_and(|previous| {
                    let previous = mask(previous);
                    let previous = previous.trim_end();
                    [
                        "|>", "+", "-", "*", "/", "·", "&&", "||", "==", "!=", "<", ">", "<=",
                        ">=", ":=", "=", "+=", "-=", "*=", "/=", "^=",
                    ]
                    .iter()
                    .any(|token| previous.ends_with(token))
                });
            if continuation && !statements.is_empty() {
                let previous = statements.last_mut().unwrap();
                previous.push(' ');
                previous.push_str(&part);
            } else {
                statements.push(part);
            }
        }
    }
    Ok(statements)
}

enum ArrowStatement<'a> {
    Binding {
        target: &'a str,
        value: &'a str,
    },
    Mutation {
        target: &'a str,
        operator: &'a str,
        value: &'a str,
    },
    Expression(&'a str),
}

/// Split only statement operators. Comparison/match operators, quoted text,
/// and operators inside selectors or RHS expressions keep their parser meaning.
/// Targets are retained verbatim for the canonical parser to validate.
fn arrow_statement(source: &str) -> Result<ArrowStatement<'_>, String> {
    let bindings = top_offsets(source, ":=")?;
    if let Some(&offset) = bindings.first() {
        if bindings.len() != 1 {
            return Err("function local requires one binding operator".into());
        }
        return Ok(ArrowStatement::Binding {
            target: source[..offset].trim(),
            value: source[offset + 2..].trim(),
        });
    }
    let code = mask(source);
    let mut assignment = None;
    for offset in top_offsets(source, "=")? {
        let previous = offset.checked_sub(1).and_then(|i| code.as_bytes().get(i));
        let next = code.as_bytes().get(offset + 1);
        if matches!(
            previous,
            Some(b'=' | b'!' | b'<' | b'>' | b':' | b'?' | b'@')
        ) || matches!(next, Some(b'=' | b'>'))
            || code[..offset].ends_with('¬')
            || code[..offset].ends_with("..")
        {
            continue;
        }
        let start = if matches!(previous, Some(b'+' | b'-' | b'*' | b'/' | b'^')) {
            offset - 1
        } else {
            offset
        };
        if assignment.replace((start, offset + 1)).is_some() {
            return Err("function local requires one assignment operator".into());
        }
    }
    if let Some((start, end)) = assignment {
        Ok(ArrowStatement::Mutation {
            target: source[..start].trim(),
            operator: &source[start..end],
            value: source[end..].trim(),
        })
    } else {
        Ok(ArrowStatement::Expression(source))
    }
}

fn assignment_base(target: &str) -> Option<&str> {
    let end = target
        .char_indices()
        .find_map(|(offset, ch)| (!identifier_char(ch)).then_some(offset))
        .unwrap_or(target.len());
    let name = &target[..end];
    let remainder = target[end..].trim_start();
    (identifier(name) && (remainder.is_empty() || remainder.starts_with(['.', '[', '{'])))
        .then_some(name)
}

fn normalize_arrow_body(
    source: &str,
    result: &str,
    prefix: &str,
    serial: &mut usize,
) -> Result<Option<Vec<String>>, String> {
    let statements = body_statements(source)?;
    if statements.is_empty() {
        return Err("function body requires a final expression or binding".into());
    }
    // Preserve existing expression normalization byte-for-byte.
    if statements.len() == 1
        && matches!(
            arrow_statement(&statements[0])?,
            ArrowStatement::Expression(_)
        )
    {
        return Ok(None);
    }
    let mut definitions = Vec::new();
    let last = statements.len() - 1;
    for (index, statement) in statements.iter().enumerate() {
        let classified = arrow_statement(statement)?;
        let (binding, mutation, expression) = match classified {
            ArrowStatement::Binding { target, value } => (Some(target), None, value),
            ArrowStatement::Mutation {
                target,
                operator,
                value,
            } => (None, Some((target, operator)), value),
            ArrowStatement::Expression(value) => (None, None, value),
        };
        if expression.is_empty() {
            return Err("function local requires a value".into());
        }
        let (temporary, value) = pipeline(expression, prefix, serial)?;
        definitions.extend(temporary);
        if let Some((target, operator)) = mutation {
            definitions.push(format!("{target} {operator} {value}"));
            if index == last {
                // Canonical statement assignment returns the updated whole
                // state, including its schema conversion and selected writes.
                // A subsequent base read selects that source-order candidate;
                // re-reading a selector could repeat an effect or select only
                // part of the assignment's result.
                let base = assignment_base(target)
                    .ok_or("a final assignment requires a local state target")?;
                definitions.push(format!("{result} := {base}"));
            }
            continue;
        }
        if index == last {
            // A typed final binding returns the value after its annotation's
            // conversion, rather than converting the unannotated RHS again.
            if let Some((binding, name)) = binding.and_then(|binding| {
                binding.split_once('<').and_then(|(name, _)| {
                    let name = name.trim().trim_start_matches('~').trim_start();
                    identifier(name).then_some((binding, name))
                })
            }) {
                definitions.push(format!("{binding} := {value}"));
                definitions.push(format!("{result} := {name}"));
                continue;
            }
            // Return the final binding's value without evaluating its RHS twice.
            definitions.push(format!("{result} := {value}"));
            if let Some(binding) = binding {
                definitions.push(format!("{binding} := {result}"));
            }
        } else if let Some(binding) = binding {
            definitions.push(format!("{binding} := {value}"));
        } else {
            let effect = format!("{prefix}effect-{}", *serial);
            *serial += 1;
            definitions.push(format!("{effect} := {value}"));
        }
    }
    Ok(Some(definitions))
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
    let original = source;
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
        let original_assignment = top_offsets(original, ":=")?[0];
        let original_body = original[original_assignment + 2..].trim();
        let original_body = original_body.strip_suffix('.').unwrap_or(original_body);
        let definitions =
            if let Some(block) = normalize_arrow_body(original_body, &result, prefix, serial)? {
                block
            } else {
                let (mut definitions, value) = pipeline(expression, prefix, serial)?;
                definitions.push(format!("{result} := {value}"));
                definitions
            };
        Ok(Some(format!(
            "{header} = ({result}{kind}) := {}.",
            definitions.join("; ")
        )))
    } else if signature.len() == 1 {
        let (mut definitions, value) = pipeline(expression, prefix, serial)?;
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
        let arrow = arrow_definition(line);
        let header_indent = indentation(line);
        loop {
            let code = mask(&statement);
            let unbalanced = top_offsets(&statement, "|>").is_err();
            if arrow && !unbalanced && code.trim_end().ends_with('.') {
                break;
            }
            let arrow_continuation = arrow
                && lines.get(index + 1).is_some_and(|next| {
                    mask(next).trim().is_empty() || indentation(next) > header_indent
                });
            let continuation = code.trim_end().ends_with("|>")
                || (!arrow
                    && code.trim_end().ends_with(":=")
                    && (code.contains("=>")
                        || code
                            .split(":=")
                            .next()
                            .is_some_and(|s| identifier(s.trim()))))
                || lines
                    .get(index + 1)
                    .is_some_and(|s| mask(s).trim_start().starts_with("|>"));
            if !(unbalanced || continuation || arrow_continuation) || index + 1 >= lines.len() {
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
    fn arrow_blocks_keep_local_definition_order_and_return_final_expression() {
        let source = "Value(x<u64>) => <u64> :=\n  first := x + 1\n  second := first * 2\n  second + 3\nresult := Value(4)\n";
        let normalized = normalize(source).unwrap();
        assert!(normalized.contains(
            "first := x + 1; second := first * 2; application-temp-result := second + 3."
        ));
        assert_eq!(normalized.lines().count(), source.lines().count());
        assert!(normalized.ends_with("result := Value(4)\n"));
        assert_eq!(normalize(&normalized).unwrap(), normalized);
    }
    #[test]
    fn arrow_blocks_return_final_binding_without_repeating_its_effect() {
        let source = "Value(x<u64>) => <u64> :=\n  first := x + 1\n  answer := Touch(first)\n";
        let normalized = normalize(source).unwrap();
        assert_eq!(normalized.matches("Touch(first)").count(), 1);
        assert!(normalized.contains(
            "application-temp-result := Touch(first); answer := application-temp-result."
        ));
    }
    #[test]
    fn arrow_blocks_return_the_converted_value_of_a_typed_final_binding() {
        let source = "Value(x<bigint>) => <u64> :=\n  answer<u64> := Touch(x)\n";
        let normalized = normalize(source).unwrap();
        assert_eq!(normalized.matches("Touch(x)").count(), 1);
        assert!(normalized.contains("answer<u64> := Touch(x); application-temp-result := answer."));
    }
    #[test]
    fn arrow_blocks_return_destructured_final_binding_as_one_tuple_value() {
        let source = "Value(x<u64>) => <(u64,u64)> :=\n  (left, right) := Pair(x)\n";
        let normalized = normalize(source).unwrap();
        assert_eq!(normalized.matches("Pair(x)").count(), 1);
        assert!(normalized.contains("application-temp-result<(u64,u64)>"));
        assert!(normalized.contains(
            "application-temp-result := Pair(x); (left, right) := application-temp-result."
        ));
    }
    #[test]
    fn arrow_expression_and_block_results_keep_tuple_parentheses() {
        let source = "Pair(x<u64>) => <(u64,u64)> := (x, x + 1)\nNested(x<u64>) => <(u64,(u64,bool))> :=\n  next := x + 1\n  ((x, (next, true)))\n";
        let normalized = normalize(source).unwrap();
        assert!(normalized.contains("application-temp-result := (x, x + 1)."));
        assert!(normalized.contains("application-temp-result := (x, (next, true))."));
        assert_eq!(normalize(&normalized).unwrap(), normalized);
    }
    #[test]
    fn tuple_pipeline_inputs_are_retained_while_scalar_grouping_is_unwrapped() {
        let normalized = normalize(
            "pair := (1, (2, 3)) |> Take(_)\nvalue := (Input() |> Add(_, _)) |> Next(_)\n",
        )
        .unwrap();
        assert!(
            normalized
                .contains("application-temp-0 := (1, (2, 3)); pair := Take(application-temp-0)")
        );
        assert_eq!(normalized.matches("Input()").count(), 1);
        assert!(normalized.contains("value := Next(application-temp-2)"));
    }
    #[test]
    fn arrow_blocks_keep_effect_statements_and_multiline_pipeline_expressions() {
        let source = "Value(x<u64>) => <u64> :=\n  io/print(\"touch:{}\\n\", x)\n  next := Add(\n    x, 1)\n  next\n  |> Add(_, _)\n";
        let normalized = normalize(source).unwrap();
        assert_eq!(normalized.matches("io/print(").count(), 1);
        assert!(normalized.contains("effect-0 := io/print("));
        assert!(normalized.contains("next := Add("));
        assert!(normalized.contains("x, 1)"));
        assert!(normalized.contains("application-temp-1 := next; application-temp-result := Add(application-temp-1, application-temp-1)."));
        assert_eq!(normalized.lines().count(), source.lines().count());
    }
    #[test]
    fn arrow_blocks_reject_missing_body_or_missing_binding_value() {
        assert!(
            normalize("Empty(x<u64>) => <u64> :=\nresult := 1\n")
                .unwrap_err()
                .contains("final expression or binding")
        );
        assert!(
            normalize("Empty(x<u64>) => <u64> :=\n  -- no result\n")
                .unwrap_err()
                .contains("final expression or binding")
        );
        assert!(
            normalize("Empty(x<u64>) => <u64> :=\n  result :=\n")
                .unwrap_err()
                .contains("requires a value")
        );
    }
    #[test]
    fn arrow_blocks_accept_multiline_binding_and_arithmetic_expressions() {
        let source = "Value(x<u64>) => <u64> :=\n  first :=\n    x +\n    1\n  first * 2\n";
        let normalized = normalize(source).unwrap();
        assert!(normalized.contains("first := x + 1; application-temp-result := first * 2."));
        assert_eq!(normalized.lines().count(), source.lines().count());
    }
    #[test]
    fn arrow_blocks_preserve_mutations_before_the_final_expression() {
        let source = "Value(x<f64>) => <[f64]:1,2> :=\n  ~state := [x, 0.0]\n  ~iteration := 0.0\n  state = [state[1] + 1.0, state[2]]\n  iteration += 5.0\n  [state[1], iteration].\n";
        let normalized = normalize(source).unwrap();
        assert!(normalized.contains("~state := [x, 0.0]; ~iteration := 0.0; state = [state[1] + 1.0, state[2]]; iteration += 5.0; application-temp-result := [state[1], iteration]."));
        assert!(!normalized.contains("effect-"));
        assert_eq!(normalized.lines().count(), source.lines().count());
        assert_eq!(normalize(&normalized).unwrap(), normalized);
    }
    #[test]
    fn arrow_mutations_keep_field_index_targets_and_each_assignment_operator() {
        let source = "Value(x<f64>) => <f64> :=\n  state.field[Which(x == 1.0)] = x\n  state[1] += x\n  state.field -= x\n  state{2} *= x\n  state.1 /= x\n  state[1, 2] ^= x\n  x\n";
        let normalized = normalize(source).unwrap();
        assert!(normalized.contains("state.field[Which(x == 1.0)] = x; state[1] += x; state.field -= x; state{2} *= x; state.1 /= x; state[1, 2] ^= x; application-temp-result := x."));
        assert!(!normalized.contains("effect-"));
    }
    #[test]
    fn arrow_mutation_rhs_pipelines_evaluate_each_input_once() {
        let source = "Value(x<f64>) => <f64> :=\n  ~state := x\n  state = Input(x) |> Next(_)\n  state += Input(state) |> Next(_)\n  state\n";
        let normalized = normalize(source).unwrap();
        assert_eq!(normalized.matches("Input(x)").count(), 1);
        assert_eq!(normalized.matches("Input(state)").count(), 1);
        assert!(normalized.contains("application-temp-0 := Input(x); state = Next(application-temp-0); application-temp-1 := Input(state); state += Next(application-temp-1); application-temp-result := state."));
    }
    #[test]
    fn arrow_comparisons_strings_comments_and_match_arrows_are_not_mutations() {
        let source = "Value(x<f64>) => <bool> :=\n  x == 1.0\n  x != 2.0\n  x <= 3.0\n  x >= 4.0\n  x ¬= 5.0\n  x === 6.0\n  x !== 7.0\n  0..=3\n  io/print(\"state = {} +=\", x) -- state = ignored\n  x == 5.0 ? | true => true | * => false\n";
        let normalized = normalize(source).unwrap();
        assert!(normalized.contains("effect-0 := x == 1.0; application-temp-effect-1 := x != 2.0; application-temp-effect-2 := x <= 3.0; application-temp-effect-3 := x >= 4.0;"));
        assert!(
            normalized
                .contains("application-temp-result := x == 5.0 ? | true => true | * => false.")
        );
        assert!(normalized.contains("effect-4 := x ¬= 5.0; application-temp-effect-5 := x === 6.0; application-temp-effect-6 := x !== 7.0; application-temp-effect-7 := 0..=3;"));
        assert!(normalized.contains("\"state = {} +=\""));
    }
    #[test]
    fn arrow_blocks_accept_multiline_assignment_values() {
        let source = "Value(x<f64>) => <f64> :=\n  ~state := x\n  state =\n    x + 1.0\n  state +=\n    x\n    |> Next(_)\n  state\n";
        let normalized = normalize(source).unwrap();
        assert!(normalized.contains("state = x + 1.0; application-temp-0 := x; state += Next(application-temp-0); application-temp-result := state."));
        assert_eq!(normalized.lines().count(), source.lines().count());
    }
    #[test]
    fn arrow_final_mutations_return_the_updated_base_without_repeating_selectors() {
        let source = "Value(x<f64>) => <u64> :=\n  ~state<u64> := 0\n  state += Touch(x)\nSelected(x<f64>) => <[f64]:1,2> :=\n  ~state := [x, x]\n  state[Which(x)] = Touch(x) |> Next(_)\nSingle(x<f64>) => <f64> := state.field = x\n";
        let normalized = normalize(source).unwrap();
        assert!(
            normalized
                .contains("~state<u64> := 0; state += Touch(x); application-temp-result := state.")
        );
        assert!(normalized.contains(
            "state[Which(x)] = Next(application-temp-0); application-temp-result := state."
        ));
        assert_eq!(normalized.matches("Which(x)").count(), 1);
        assert_eq!(normalized.matches("Touch(x)").count(), 2);
        assert!(normalized.contains("state.field = x; application-temp-result := state."));
    }
    #[test]
    fn arrow_final_typed_mutable_bindings_return_the_converted_value() {
        let normalized =
            normalize("Value(x<f64>) => <u64> :=\n  ~answer<u64> := Touch(x)\n").unwrap();
        assert!(
            normalized.contains("~answer<u64> := Touch(x); application-temp-result := answer.")
        );
        assert_eq!(normalized.matches("Touch(x)").count(), 1);
    }
    #[test]
    fn arrow_mutations_reject_missing_values_and_multiple_assignment_operators() {
        for source in [
            "Value(x<f64>) => <f64> :=\n  state =\n",
            "Value(x<f64>) => <f64> :=\n  state +=\n",
        ] {
            assert!(normalize(source).unwrap_err().contains("requires a value"));
        }
        assert!(
            normalize("Value(x<f64>) => <f64> :=\n  state = other = x\n  x\n")
                .unwrap_err()
                .contains("one assignment operator")
        );
        assert!(
            normalize("Value(x<f64>) => <f64> := @main/state = x\n")
                .unwrap_err()
                .contains("final assignment requires a local state target")
        );
    }
    #[cfg(feature = "native-sequence")]
    #[test]
    fn normalized_arrow_mutations_are_canonical_statements() {
        let source = "Value(x<f64>) => <[f64]:1,2> :=\n  ~state := [x, 0.0]\n  same := x == 1.0\n  state = [state[1] + 1.0, state[2]]\n  state[1] += x |> Next(_)\n  state[1] -= x\n  state[1] *= 2.0\n  state[1] /= 2.0\n  state[1] ^= 2.0\n  state[2] = x + 1.0\n";
        let parsed = super::super::ParsedSource::parse(source).unwrap();
        fn mutations(node: &mech_syntax::document::SyntaxNode) -> usize {
            use mech_syntax::document::SyntaxKind;
            usize::from(matches!(
                node.kind(),
                SyntaxKind::VariableAssign | SyntaxKind::OpAssign
            )) + node
                .children()
                .map(|child| mutations(&child))
                .sum::<usize>()
        }
        assert_eq!(mutations(&parsed.syntax), 7);
    }
    #[cfg(all(feature = "source", feature = "resident-artifact"))]
    #[test]
    fn arrow_final_mutations_execute_with_updated_state_results() {
        use crate::resident::{ActivationFacts, ResidentValueBorrow, activate};
        use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId};
        use mech_syntax::document::{AstNode, DocumentSyntax};

        let mut catalog = FunctionCatalogBuilder::new();
        crate::install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        for (source, expected) in [
            (
                "Value(x<f64>) => <f64> :=\n  ~state := 2.0\n  state += x\nanswer := Value(3.0)\nanswer\n",
                vec![5.0],
            ),
            (
                "Twice(x<f64>) => <f64> := x + x\nValue(x<f64>) => <[f64]:1,2> :=\n  ~state := [2.0, 4.0]\n  state[2] = x |> Twice(_)\nanswer := Value(3.0)\nanswer\n",
                vec![2.0, 6.0],
            ),
        ] {
            let parsed = super::super::ParsedSource::parse(source).unwrap();
            let document = DocumentSyntax::cast(parsed.syntax).unwrap();
            let compiled = crate::CanonicalSourceFrontend
                .compile_document(&document)
                .unwrap();
            let output = compiled
                .document_outputs()
                .iter()
                .find(|binding| binding.kind == crate::SourceDocumentOutputKind::Program)
                .unwrap()
                .output as usize;
            let artifact = compiled.compile_artifact().unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0xA220, 0),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            let candidate = instance.prepare_turn(&[]).unwrap();
            let Some(ResidentValueBorrow::F64 { values, .. }) = candidate.output_borrow(output)
            else {
                panic!("{source}: expected f64 result");
            };
            assert_eq!(values, expected.as_slice(), "{source}");
        }
    }
    #[test]
    fn explicit_arrow_terminator_does_not_capture_following_indented_code() {
        let source = "Value(x<u64>) => <u64> := x + 1.\n  result := Value(4)\n";
        let normalized = normalize(source).unwrap();
        assert!(normalized.ends_with("\n  result := Value(4)\n"));
        assert!(!normalized.contains("; result :="));
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
