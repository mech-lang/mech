#[macro_use]
use crate::*;

const STDLIB_MODULE_ROOTS: &[&str] = &["math", "stats", "io", "string", "combinatorics"];

fn is_valid_import_part(part: &str) -> bool {
    !part.is_empty()
        && !part.chars().any(|ch| {
            ch.is_whitespace()
                || ch == ','
                || ch == '{'
                || ch == '}'
                || ch == '*'
                || ch == ':'
                || ch == '='
        })
}

fn identifier_from_part(part: &str, range: SourceRange) -> Identifier {
    Identifier {
        name: Token {
            kind: TokenKind::Identifier,
            chars: part.chars().collect(),
            src_range: range,
        },
    }
}

fn identifiers_from_item_path<'a>(path: &str, range: SourceRange, input: ParseString<'a>) -> Result<Vec<Identifier>, nom::Err<ParseError<'a>>> {
    let parts: Vec<&str> = path.split('/').map(str::trim).collect();

    if parts.is_empty() || parts.iter().any(|part| !is_valid_import_part(part)) {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid module import item",
        )));
    }

    Ok(parts
        .iter()
        .map(|part| identifier_from_part(part, range.clone()))
        .collect())
}

fn module_import_body(mut input: ParseString) -> ParseResult<String> {
    let mut raw = String::new();
    let mut brace_depth = 0usize;
    let mut saw_group = false;

    loop {
        if input.is_empty() {
            if brace_depth == 0 {
                return Ok((input, raw));
            }
            return Err(nom::Err::Failure(ParseError::new(
                input,
                "Unclosed module import group",
            )));
        }

        let current = input.current().unwrap_or("");

        if graphemes::is_new_line(current) && brace_depth == 0 {
            let (next_input, _) = skip_till_eol(input)?;
            return Ok((next_input, raw));
        }

        let (next_input, grapheme) = any(input)?;

        if grapheme == "{" {
            saw_group = true;
            brace_depth += 1;
        } else if grapheme == "}" {
            if brace_depth == 0 {
                return Err(nom::Err::Failure(ParseError::new(
                    next_input,
                    "Unexpected module import group close",
                )));
            }
            brace_depth -= 1;
        }

        raw.push_str(&grapheme);
        input = next_input;

        if saw_group && brace_depth == 0 {
            if input.is_empty() {
                return Ok((input, raw));
            }

            if let Some(current) = input.current() {
                if graphemes::is_new_line(current) {
                    let (next_input, _) = skip_till_eol(input)?;
                    return Ok((next_input, raw));
                }
            }
        }
    }
}

fn parse_group_items<'a>(raw: &str, range: SourceRange, input: ParseString<'a>) -> Result<Vec<ModuleImportGroupItem>, nom::Err<ParseError<'a>>> {
    let normalized = raw.replace(',', "\n");
    let parts = normalized
        .split_whitespace()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid empty module import group",
        )));
    }

    let mut items = Vec::new();
    for part in parts {
        if part.contains(":=") {
            return Err(nom::Err::Failure(ParseError::new(
                input,
                "Module import group aliases are not supported",
            )));
        }

        if part == "*" {
            return Err(nom::Err::Failure(ParseError::new(
                input,
                "Invalid wildcard placement in module import path",
            )));
        }

        let item = identifiers_from_item_path(part, range.clone(), input.clone())?;
        items.push(ModuleImportGroupItem { item });
    }

    Ok(items)
}

fn is_source_import_path(path: &str) -> bool {
    path.contains("://")
        || path.starts_with("./")
        || path.starts_with("../")
        || path.ends_with(".mec")
        || path.contains('.')
}

pub fn module_import(input: ParseString) -> ParseResult<ModuleImport> {
    let original = input.clone();
    let (input, _) = whitespace0(input)?;
    let (input, _) = plus(input)?;
    let (input, _) = right_angle(input)?;
    let (input, _) = many0(space_tab)(input)?;
    let start = input.loc();
    let (input, raw) = module_import_body(input)?;
    let mut path = raw.trim();

    if is_source_import_path(path) {
        return Err(nom::Err::Error(ParseError::new(
            original,
            "not a stdlib module import",
        )));
    }

    if path.is_empty() {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid empty module import path",
        )));
    }

    let end = input.loc();
    let range = SourceRange { start, end };
    let mut alias = None;

    if path.contains(":=") {
        if path.contains('{') || path.contains('}') || path.matches(":=").count() != 1 {
            return Err(nom::Err::Failure(ParseError::new(input, "Invalid module import alias")));
        }
        let alias_split = path.split_once(":=").unwrap();
        let alias_part = alias_split.0.trim();
        if !is_valid_import_part(alias_part) {
            return Err(nom::Err::Failure(ParseError::new(input, "Invalid module import alias")));
        }
        alias = Some(identifier_from_part(alias_part, range.clone()));
        path = alias_split.1.trim();
        if path.is_empty() {
            return Err(nom::Err::Failure(ParseError::new(input, "Invalid module import alias")));
        }
    }

    let (module_part, rest) = if let Some((module, rest)) = path.split_once('/') {
        (module.trim(), Some(rest.trim()))
    } else {
        (path.trim(), None)
    };

    if !is_valid_import_part(module_part) {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid module import module",
        )));
    }

    if !STDLIB_MODULE_ROOTS.contains(&module_part) {
        return Err(nom::Err::Error(ParseError::new(
            original,
            "not a known stdlib module import",
        )));
    }

    let module = identifier_from_part(module_part, range.clone());

    match rest {
        None => {
            if alias.is_some() {
                return Err(nom::Err::Failure(ParseError::new(input, "Invalid module import alias")));
            }
            Ok((input, ModuleImport { module, item: None, group_items: None, alias: None, kind: ModuleImportKind::Module }))
        }
        Some("*") => {
            if alias.is_some() {
                return Err(nom::Err::Failure(ParseError::new(input, "Invalid module import alias")));
            }
            Ok((input, ModuleImport { module, item: None, group_items: None, alias: None, kind: ModuleImportKind::Glob }))
        }
        Some(rest) if rest.starts_with('{') && rest.ends_with('}') => {
            if alias.is_some() {
                return Err(nom::Err::Failure(ParseError::new(input, "Invalid module import alias")));
            }
            let group_raw = &rest[1..rest.len() - 1];
            let group_items = parse_group_items(group_raw, range, input.clone())?;
            Ok((input, ModuleImport { module, item: None, group_items: Some(group_items), alias: None, kind: ModuleImportKind::Group }))
        }
        Some(rest) => {
            if rest.contains('*') {
                return Err(nom::Err::Failure(ParseError::new(input, "Invalid wildcard placement in module import path")));
            }
            let item = identifiers_from_item_path(rest, range, input.clone())?;
            Ok((input, ModuleImport { module, item: Some(item), group_items: None, alias, kind: ModuleImportKind::Item }))
        }
    }
}
