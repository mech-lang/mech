#[macro_use]
use crate::*;
fn identifier_from_part(part: &str, range: SourceRange) -> Identifier {
    Identifier {
        name: Token {
            kind: TokenKind::Identifier,
            chars: part.chars().collect(),
            src_range: range,
        },
    }
}

fn is_source_import_path(path: &str) -> bool {
    path.contains("://")
        || path.starts_with("./")
        || path.starts_with("../")
        || path.ends_with(".mec")
        || path.contains('.')
}

fn is_known_stdlib_root(module: &str) -> bool {
    matches!(module, "math" | "stats" | "io" | "string" | "combinatorics")
}

pub fn module_import(input: ParseString) -> ParseResult<ModuleImport> {
    let original = input.clone();
    let (input, _) = whitespace0(input)?;
    let (input, _) = plus(input)?;
    let (input, _) = right_angle(input)?;
    let (input, _) = many0(space_tab)(input)?;
    let start = input.loc();
    let (input, raw_token) = skip_till_eol(input)?;
    let raw = raw_token.to_string();
    let path = raw.trim();

    if is_source_import_path(path) {
        return Err(nom::Err::Error(ParseError::new(
            original,
            "not a stdlib module import",
        )));
    }

    let parts: Vec<&str> = path.split('/').map(str::trim).collect();
    let end = input.loc();
    let range = SourceRange { start, end };

    if path.is_empty() || parts.is_empty() {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid module import path",
        )));
    }

    let module = parts[0];
    if module.is_empty() || module == "*" {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid module import module",
        )));
    }

    if !is_known_stdlib_root(module) {
        return Err(nom::Err::Error(ParseError::new(
            original,
            "not a known stdlib module import",
        )));
    }

    if parts.iter().any(|part| part.is_empty()) {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid empty module import item",
        )));
    }

    let module = identifier_from_part(module, range.clone());
    match parts.as_slice() {
        [] => Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid module import path",
        ))),
        [_] => Ok((
            input,
            ModuleImport {
                module,
                item: None,
                kind: ModuleImportKind::Module,
            },
        )),
        [_, "*"] => Ok((
            input,
            ModuleImport {
                module,
                item: None,
                kind: ModuleImportKind::Glob,
            },
        )),
        [_, items @ ..] => {
            if items.iter().any(|item| *item == "*") {
                return Err(nom::Err::Failure(ParseError::new(
                    input,
                    "Invalid wildcard placement in module import path",
                )));
            }
            let item = items
                .iter()
                .map(|part| identifier_from_part(part, range.clone()))
                .collect();
            Ok((
                input,
                ModuleImport {
                    module,
                    item: Some(item),
                    kind: ModuleImportKind::Item,
                },
            ))
        }
    }
}
