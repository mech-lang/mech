#[macro_use]
use crate::*;
use nom::{
    branch::alt,
    combinator::{cut, map, opt},
    multi::many0,
    sequence::{delimited, preceded, tuple as nom_tuple},
};

fn module_import_name_segment(input: ParseString) -> ParseResult<ModuleImportPathSegment> {
    let (input, name) = identifier_path_segment(input)?;
    Ok((input, ModuleImportPathSegment::Name(name)))
}

fn module_import_intrinsic_segment(input: ParseString) -> ParseResult<ModuleImportPathSegment> {
    let (input, marker) = underscore(input)?;
    let (input, name) = cut(identifier_path_segment)(input)?;
    Ok((
        input,
        ModuleImportPathSegment::Intrinsic(ModuleImportIntrinsicSegment {
            marker,
            name,
        }),
    ))
}

fn module_import_path_segment(input: ParseString) -> ParseResult<ModuleImportPathSegment> {
    alt((
        module_import_intrinsic_segment,
        module_import_name_segment,
    ))(input)
}

fn module_import_path(input: ParseString) -> ParseResult<ModuleImportPath> {
    let (input, first) = module_import_path_segment(input)?;
    let (input, mut rest) = many0(preceded(slash, module_import_path_segment))(input)?;

    let mut segments = vec![first];
    segments.append(&mut rest);

    Ok((input, ModuleImportPath { segments }))
}

fn module_import_alias_segment(input: ParseString) -> ParseResult<ModuleImportPathSegment> {
    let (input, name) = identifier_path_segment(input)?;
    Ok((input, ModuleImportPathSegment::Name(name)))
}

fn module_import_alias_path(input: ParseString) -> ParseResult<ModuleImportPath> {
    let (input, first) = module_import_alias_segment(input)?;
    let (input, mut rest) = many0(preceded(slash, module_import_alias_segment))(input)?;

    let mut segments = vec![first];
    segments.append(&mut rest);

    Ok((input, ModuleImportPath { segments }))
}

fn module_import_value_alias(input: ParseString) -> ParseResult<ModuleImportAlias> {
    map(module_import_alias_path, ModuleImportAlias::Value)(input)
}

fn module_import_context_alias(input: ParseString) -> ParseResult<ModuleImportAlias> {
    let (input, _) = at(input)?;
    let (next, name) = identifier(input)?;
    Ok((next, ModuleImportAlias::Context(name)))
}

fn module_import_alias(input: ParseString) -> ParseResult<ModuleImportAlias> {
    alt((module_import_context_alias, module_import_value_alias))(input)
}

fn module_root(input: ParseString) -> ParseResult<Identifier> {
    let (input, module) = identifier_path_segment(input)?;
    match module.to_string().as_str() {
        "math" | "stats" | "browser" => Ok((input, module)),
        _ => Err(nom::Err::Error(ParseError::new(input, "not a built-in module import"))),
    }
}

fn import_alias_operator(input: ParseString) -> ParseResult<()> {
    let (input, _) = space_tab0(input)?;
    let (input, _) = colon(input)?;
    let (input, _) = equal(input)?;
    let (input, _) = space_tab0(input)?;
    Ok((input, ()))
}

fn import_group_separator(input: ParseString) -> ParseResult<()> {
    let (input, _) = alt((
        list_separator,
        map(whitespace1, |_| ()),
    ))(input)?;
    Ok((input, ()))
}

fn import_group_item(input: ParseString) -> ParseResult<ModuleImportGroupItem> {
    let (input, item) = module_import_path(input)?;
    Ok((input, ModuleImportGroupItem { item }))
}

fn import_group_items(input: ParseString) -> ParseResult<Vec<ModuleImportGroupItem>> {
    let (input, _) = whitespace0(input)?;
    let (input, first) = import_group_item(input)?;
    let (input, mut rest) = many0(preceded(import_group_separator, import_group_item))(input)?;
    let (input, _) = whitespace0(input)?;

    let mut items = vec![first];
    items.append(&mut rest);

    Ok((input, items))
}

fn module_import_end(input: ParseString) -> ParseResult<()> {
    let (input, _) = space_tab0(input)?;
    let (input, _) = opt(new_line)(input)?;
    Ok((input, ()))
}


fn aliased_context_item_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, _) = at(input)?;
    let (input, alias) = identifier_path_segment(input)?;
    let (input, _) = import_alias_operator(input)?;
    let (input, (module, _, item)) = cut(nom_tuple((module_root, slash, module_import_path)))(input)?;
    Ok((input, ModuleImport {
        module,
        item: Some(item),
        group_items: None,
        alias: Some(ModuleImportAlias::Context(alias)),
        kind: ModuleImportKind::Item,
    }))
}

fn aliased_item_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, alias) = module_import_alias(input)?;
    let (input, _) = import_alias_operator(input)?;

    let (input, (module, _, item)) = cut(nom_tuple((
        module_root,
        slash,
        module_import_path,
    )))(input)?;

    Ok((
        input,
        ModuleImport {
            module,
            item: Some(item),
            group_items: None,
            alias: Some(alias),
            kind: ModuleImportKind::Item,
        },
    ))
}

fn module_suffix_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, module) = module_root(input)?;
    let (input, _) = slash(input)?;
    let (input, (item, group_items, kind)) = cut(alt((
        map(asterisk, |_| (None, None, ModuleImportKind::Glob)),
        map(
            delimited(left_brace, cut(import_group_items), right_brace),
            |group_items| (None, Some(group_items), ModuleImportKind::Group),
        ),
        map(module_import_path, |item| (Some(item), None, ModuleImportKind::Item)),
    )))(input)?;

    Ok((
        input,
        ModuleImport {
            module,
            item,
            group_items,
            alias: None,
            kind,
        },
    ))
}

fn module_only_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, module) = module_root(input)?;
    if slash(input.clone()).is_ok() {
        return Err(nom::Err::Error(ParseError::new(input, "not a module-only import")));
    }

    Ok((
        input,
        ModuleImport {
            module,
            item: None,
            group_items: None,
            alias: None,
            kind: ModuleImportKind::Module,
        },
    ))
}

pub fn module_import(input: ParseString) -> ParseResult<ModuleImport> {
    let (input, _) = whitespace0(input)?;
    let (input, _) = plus(input)?;
    let (input, _) = right_angle(input)?;
    let (input, _) = space_tab0(input)?;

    let (input, mut import) = alt((
        aliased_context_item_import,
        aliased_item_import,
        module_suffix_import,
        module_only_import,
    ))(input)?;

    let (next_input, _) = module_import_end(input.clone())?;
    import.module.name.src_range.end = next_input.loc();

    Ok((input, import))
}
