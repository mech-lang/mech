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

    // Legacy file imports such as `+> ./dep.mec` remain statement imports.
    if path.starts_with('.') || path.contains('.') {
        return Err(nom::Err::Error(ParseError::new(
            original,
            "not a stdlib module import",
        )));
    }

    let parts: Vec<&str> = path.split('/').collect();
    let end = input.loc();
    let range = SourceRange { start, end };

    if path.is_empty() || parts.is_empty() || parts.len() > 2 {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid module import path",
        )));
    }

    let module = parts[0].trim();
    if module.is_empty() || module == "*" {
        return Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid module import module",
        )));
    }

    let module = identifier_from_part(module, range.clone());
    match parts.as_slice() {
        [_] => Ok((
            input,
            ModuleImport {
                module,
                item: None,
                kind: ModuleImportKind::Module,
            },
        )),
        [_, item] => {
            let item = item.trim();
            if item.is_empty() {
                return Err(nom::Err::Failure(ParseError::new(
                    input,
                    "Invalid empty module import item",
                )));
            }
            if item == "*" {
                return Ok((
                    input,
                    ModuleImport {
                        module,
                        item: None,
                        kind: ModuleImportKind::Glob,
                    },
                ));
            }
            Ok((
                input,
                ModuleImport {
                    module,
                    item: Some(identifier_from_part(item, range)),
                    kind: ModuleImportKind::Item,
                },
            ))
        }
        _ => Err(nom::Err::Failure(ParseError::new(
            input,
            "Invalid module import path",
        ))),
    }
}
